use super::*;
use tokio::sync::Notify;

#[tokio::test]
async fn cancelled_caller_cannot_release_setup_while_owned_work_continues() {
    let setup = SandboxSetup::default();
    let entered = Arc::new(Notify::new());
    let (release, released) = std::sync::mpsc::channel();
    let caller = tokio::spawn({
        let setup = setup.clone();
        let entered = entered.clone();

        async move {
            setup
                .run_owned(async move {
                    tokio::task::spawn_blocking(move || {
                        entered.notify_one();
                        released.recv().unwrap();
                    })
                    .await?;
                    Ok(())
                })
                .await
        }
    });
    entered.notified().await;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(setup.gate.try_lock().is_err());
    let second = tokio::spawn({
        let setup = setup.clone();
        async move { setup.run_owned(async { Ok(42) }).await }
    });
    assert!(!second.is_finished());
    release.send(()).unwrap();
    assert_eq!(second.await.unwrap().unwrap(), 42);
    assert!(setup.gate.try_lock().is_ok());
}

#[test]
fn java_discovery_does_not_swallow_invalid_existing_paths() {
    let temp = tempfile::tempdir().unwrap();
    let java = temp.path().join("usr/lib/jvm/java-21-openjdk/bin/java");
    std::fs::create_dir_all(&java).unwrap();
    assert!(matches!(
        find_guest_java(temp.path(), 21),
        Err(SandboxError::InvalidRootfs)
    ));
}

#[test]
fn java_major_uses_supported_alpine_package() {
    assert_eq!(java_package(8), (8, "openjdk8-jre"));
    assert_eq!(java_package(9), (11, "openjdk11-jre-headless"));
    assert_eq!(java_package(16), (17, "openjdk17-jre-headless"));
    assert_eq!(java_package(20), (21, "openjdk21-jre-headless"));
    assert_eq!(java_package(25), (25, "openjdk25-jre-headless"));
}

#[tokio::test]
async fn unconfirmed_process_cleanup_blocks_new_setup() {
    let setup = SandboxSetup::default();
    let error = setup
        .run_owned(async {
            Err::<(), _>(SandboxError::ProcessCleanup {
                primary: Box::new(SandboxError::ExecutionTimeout),
                source: std::io::Error::new(std::io::ErrorKind::TimedOut, "reap timed out"),
            })
        })
        .await
        .unwrap_err();
    assert!(matches!(error, SandboxError::ProcessCleanup { .. }));
    let error = setup.run_owned(async { Ok(42) }).await.unwrap_err();
    assert!(matches!(error, SandboxError::RecoveryBlocked(_)));
}
