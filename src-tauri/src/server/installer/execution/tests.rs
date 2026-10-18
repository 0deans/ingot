use super::*;
use tokio::io::AsyncReadExt;

const HELPER_TEST: &str = "server::installer::execution::tests::installer_process_helper";

fn helper_command(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--ignored", "--exact", HELPER_TEST, "--nocapture"]);
    command.env("INGOT_INSTALLER_HELPER_MODE", mode);
    command
}

#[test]
#[ignore = "Hermetic child-process fixture invoked only by installer execution tests"]
fn installer_process_helper() {
    let Ok(mode) = std::env::var("INGOT_INSTALLER_HELPER_MODE") else {
        return;
    };
    match mode.as_str() {
        "hang" => {
            let address = std::env::var("INGOT_INSTALLER_HELPER_ADDRESS").unwrap();
            let _connection = std::net::TcpStream::connect(address).unwrap();
            loop {
                std::thread::park();
            }
        }
        "fail" => {
            eprintln!("installer fixture failure");
            std::process::exit(1);
        }
        "success" => {}
        "flood" => {
            use std::io::Write;
            let output = vec![b'x'; OUTPUT_LIMIT_BYTES * 4];
            std::io::stdout().write_all(&output).unwrap();
            std::io::stdout().flush().unwrap();
            std::io::stderr().write_all(&output).unwrap();
            eprintln!("\nfinal installer marker");
            std::process::exit(1);
        }
        _ => panic!("unknown fixture mode"),
    }
}

#[tokio::test]
async fn drains_output_while_retaining_only_bounded_tail() {
    let mut data = vec![b'x'; OUTPUT_LIMIT_BYTES * 4];
    data.extend_from_slice(b"final output marker");
    let tail = read_tail(Some(std::io::Cursor::new(data))).await.unwrap();
    assert_eq!(tail.len(), OUTPUT_LIMIT_BYTES);
    assert!(tail.ends_with(b"final output marker"));
}

#[tokio::test]
async fn unsuccessful_process_reports_bounded_diagnostics() {
    let error = execute(
        helper_command("fail"),
        &ServerCoreType::Forge,
        INSTALLER_TIMEOUT,
    )
    .await
    .unwrap_err();
    let InstallerError::InstallationFailed { details, .. } = error else {
        panic!("expected unsuccessful exit");
    };
    assert!(details.contains("installer fixture failure"));
}

#[tokio::test]
async fn large_stdout_and_stderr_are_drained_without_unbounded_diagnostics() {
    let error = execute(
        helper_command("flood"),
        &ServerCoreType::Forge,
        INSTALLER_TIMEOUT,
    )
    .await
    .unwrap_err();
    let InstallerError::InstallationFailed { details, .. } = error else {
        panic!("expected unsuccessful exit");
    };
    assert!(details.contains("final installer marker"));
    assert!(details.len() <= OUTPUT_LIMIT_BYTES * 2);
}

#[tokio::test]
async fn timeout_terminates_and_reaps_started_installer() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut command = helper_command("hang");
    command.env(
        "INGOT_INSTALLER_HELPER_ADDRESS",
        listener.local_addr().unwrap().to_string(),
    );
    let operation =
        tokio::spawn(
            async move { execute(command, &ServerCoreType::Forge, INSTALLER_TIMEOUT).await },
        );
    let (mut connection, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
        .await
        .unwrap()
        .unwrap();
    tokio::time::pause();
    tokio::time::advance(INSTALLER_TIMEOUT).await;
    tokio::time::resume();
    let error = operation.await.unwrap().unwrap_err();
    assert!(matches!(error, InstallerError::ExecutionTimeout { .. }));
    let mut byte = [0u8; 1];
    let result = connection.read(&mut byte).await;
    match result {
        Ok(count) => assert_eq!(count, 0),
        Err(error) => assert_eq!(error.kind(), io::ErrorKind::ConnectionReset),
    }
}

#[tokio::test]
async fn cancelling_execution_terminates_owned_child() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut command = helper_command("hang");
    command.env(
        "INGOT_INSTALLER_HELPER_ADDRESS",
        listener.local_addr().unwrap().to_string(),
    );
    let operation =
        tokio::spawn(
            async move { execute(command, &ServerCoreType::Forge, INSTALLER_TIMEOUT).await },
        );
    let (mut connection, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
        .await
        .unwrap()
        .unwrap();
    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    let mut byte = [0u8; 1];
    let result = tokio::time::timeout(Duration::from_secs(10), connection.read(&mut byte))
        .await
        .unwrap();
    match result {
        Ok(count) => assert_eq!(count, 0),
        Err(error) => assert_eq!(error.kind(), io::ErrorKind::ConnectionReset),
    }
}

#[tokio::test]
async fn cleanup_keeps_primary_command_error_and_all_artifact_failures() {
    let fixture = tempfile::tempdir().unwrap();
    for name in ["installer.jar", "installer.jar.log"] {
        std::fs::create_dir(fixture.path().join(name)).unwrap();
    }
    let error = run_downloaded_installer(
        fixture.path(),
        &ServerCoreType::Forge,
        "installer.jar",
        Vec::new(),
        |_| -> Result<Command, io::Error> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "command denied",
            ))
        },
    )
    .await
    .unwrap_err();
    let InstallerError::Cleanup {
        primary: Some(primary),
        source,
    } = error
    else {
        panic!("expected combined error");
    };
    assert!(matches!(*primary, InstallerError::Process { .. }));
    let failures = source
        .get_ref()
        .unwrap()
        .downcast_ref::<ArtifactCleanupFailures>()
        .unwrap();
    assert_eq!(failures.0.len(), 2);
    for name in ["installer.jar", "installer.jar.log"] {
        assert!(fixture.path().join(name).is_dir());
    }
}

#[tokio::test]
async fn successful_execution_cleans_only_its_installer_artifacts() {
    let fixture = tempfile::tempdir().unwrap();
    for name in ["installer.jar", "installer.jar.log", "server.properties"] {
        std::fs::write(fixture.path().join(name), name).unwrap();
    }
    run_downloaded_installer(
        fixture.path(),
        &ServerCoreType::Forge,
        "installer.jar",
        Vec::new(),
        |_| -> Result<Command, io::Error> { Ok(helper_command("success")) },
    )
    .await
    .unwrap();
    assert!(!fixture.path().join("installer.jar").exists());
    assert!(!fixture.path().join("installer.jar.log").exists());
    assert_eq!(
        std::fs::read_to_string(fixture.path().join("server.properties")).unwrap(),
        "server.properties"
    );
}

#[tokio::test]
async fn cleanup_failure_after_success_is_reported_without_inventing_primary_failure() {
    let fixture = tempfile::tempdir().unwrap();
    std::fs::create_dir(fixture.path().join("installer.jar")).unwrap();
    let error = run_downloaded_installer(
        fixture.path(),
        &ServerCoreType::Forge,
        "installer.jar",
        Vec::new(),
        |_| -> Result<Command, io::Error> { Ok(helper_command("success")) },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        InstallerError::Cleanup { primary: None, .. }
    ));
    assert!(fixture.path().join("installer.jar").is_dir());
}
