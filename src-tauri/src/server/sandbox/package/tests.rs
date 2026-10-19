use super::*;

#[tokio::test]
async fn output_retains_only_bounded_tail_without_blocking_producer() {
    use tokio::io::AsyncWriteExt;
    let (mut writer, reader) = tokio::io::duplex(1024);
    let producer = tokio::spawn(async move {
        writer
            .write_all(&vec![b'x'; OUTPUT_LIMIT * 3])
            .await
            .unwrap();
        writer.write_all(b"tail").await.unwrap();
    });
    let output = read_tail(Some(reader)).await.unwrap();
    producer.await.unwrap();
    assert_eq!(output.len(), OUTPUT_LIMIT);
    assert!(output.ends_with(b"tail"));
}

fn helper_command(address: std::net::SocketAddr) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--ignored",
        "--exact",
        "server::sandbox::package::tests::package_process_fixture",
        "--nocapture",
    ]);
    command.env("INGOT_SANDBOX_TEST_LISTENER", address.to_string());
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.kill_on_drop(true);
    command
}

#[tokio::test]
async fn timed_out_package_process_is_killed_and_reaped() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut child = helper_command(listener.local_addr().unwrap())
        .spawn()
        .unwrap();
    let (_connection, _) = listener.accept().await.unwrap();
    let error = execute_child(&mut child, "test-package", Duration::ZERO)
        .await
        .unwrap_err();
    assert!(matches!(error, SandboxError::ExecutionTimeout));
    let status = child.try_wait().unwrap().expect("process was not reaped");
    assert!(!status.success());
}

#[tokio::test]
async fn completed_failed_package_reports_bounded_output_and_status() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut child = helper_command(listener.local_addr().unwrap())
        .spawn()
        .unwrap();
    let (mut connection, _) = listener.accept().await.unwrap();
    connection.write_all(b"finish").await.unwrap();
    let error = execute_child(&mut child, "test-package", Duration::from_secs(30))
        .await
        .unwrap_err();
    match error {
        SandboxError::JavaInstallFailed {
            status, details, ..
        } => {
            assert_eq!(status.code(), Some(7));
            assert!(details.contains("package failure"));
            assert!(details.len() <= OUTPUT_LIMIT * 2);
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
#[ignore = "Internal child process fixture; exercised by parent tests"]
fn package_process_fixture() {
    use std::io::{Read, Write};
    let address = std::env::var("INGOT_SANDBOX_TEST_LISTENER").unwrap();
    let mut connection = std::net::TcpStream::connect(address).unwrap();
    let mut input = [0; 6];
    connection.read_exact(&mut input).unwrap();
    std::io::stderr().write_all(b"package failure").unwrap();
    std::process::exit(7);
}
