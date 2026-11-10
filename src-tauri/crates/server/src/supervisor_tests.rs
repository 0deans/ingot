use super::{wake_callback, WakeControl};
use crate::server::config::ServerConfig;
use crate::server::process::lifecycle_tests::{CommandFuture, Fixture};
use crate::server::process::ServerCommandBuilder;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

struct WaitingPreparation {
    entered: Arc<Notify>,
    seen: Arc<Mutex<Vec<ServerConfig>>>,
}
impl ServerCommandBuilder for WaitingPreparation {
    fn build_command(
        &self,
        _data_dir: &Path,
        _client: &reqwest::Client,
        config: &ServerConfig,
        _server_dir: &Path,
        _step: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> CommandFuture {
        let entered = self.entered.clone();
        self.seen.lock().unwrap().push(config.clone());
        Box::pin(async move {
            entered.notify_one();
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn released_generation_cannot_start_a_new_wake() {
    let fixture = Fixture::new();
    let control = Arc::new(Mutex::new(WakeControl::default()));
    let wake = wake_callback(
        fixture.context.clone(),
        "server".into(),
        25566,
        control.clone(),
    );
    control.lock().unwrap().release();
    assert!(!wake().await.unwrap());
    assert!(!fixture.context.processes.is_starting("server"));
    assert!(fixture.context.processes.process_ids().await.is_empty());
}

#[tokio::test]
async fn wake_reads_fresh_settings_and_release_cancels_its_registered_task() {
    let mut fixture = Fixture::new();
    let entered = Arc::new(Notify::new());
    let seen = Arc::new(Mutex::new(Vec::new()));
    fixture.context.command_builder = Arc::new(WaitingPreparation {
        entered: entered.clone(),
        seen: seen.clone(),
    });
    let control = Arc::new(Mutex::new(WakeControl::default()));
    let wake = wake_callback(
        fixture.context.clone(),
        "server".into(),
        25566,
        control.clone(),
    );
    fixture
        .context
        .repository
        .update(|records| {
            records[0].memory_max_mb = 8192;
            records[0].jvm_args = Some(vec!["-Dupdated=true".into()]);
            Ok(())
        })
        .unwrap();
    let result = wake();
    entered.notified().await;
    assert!(fixture.context.processes.is_starting("server"));
    assert_eq!(seen.lock().unwrap()[0].memory_max_mb, 8192);
    assert_eq!(
        seen.lock().unwrap()[0].jvm_args,
        Some(vec!["-Dupdated=true".into()])
    );
    assert!(!wake().await.unwrap());
    let task = control.lock().unwrap().task.as_ref().unwrap().clone();
    control.lock().unwrap().release();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !task.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(result.await.is_err());
    assert!(!fixture.context.processes.is_starting("server"));
    assert!(fixture.context.processes.process_ids().await.is_empty());
    assert!(!wake().await.unwrap());
}

#[tokio::test]
async fn proxy_address_conflict_preserves_bind_cause() {
    use crate::server::process::ProcessError;
    use std::error::Error;
    let failure = super::proxy_start_failure(
        25565,
        std::io::Error::new(std::io::ErrorKind::AddrInUse, "injected bind conflict"),
        async { Ok(()) },
    )
    .await;
    assert!(matches!(
        failure,
        ProcessError::PortInUse { port: 25565, .. }
    ));
    let source = failure
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .unwrap();
    assert_eq!(source.kind(), std::io::ErrorKind::AddrInUse);
}

#[tokio::test]
async fn proxy_permission_failure_is_not_a_port_conflict() {
    use crate::server::process::ProcessError;
    use std::error::Error;
    let failure = super::proxy_start_failure(
        25565,
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "injected permissions"),
        async { Ok(()) },
    )
    .await;
    assert!(matches!(
        failure,
        ProcessError::ProxyStartup { port: 25565, .. }
    ));
    let source = failure
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .unwrap();
    assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
}

#[tokio::test]
async fn proxy_cleanup_failure_preserves_primary_and_secondary_causes() {
    use crate::server::process::ProcessError;
    use std::error::Error;
    let failure = super::proxy_start_failure(
        25565,
        std::io::Error::new(std::io::ErrorKind::AddrInUse, "injected bind conflict"),
        async {
            Err(ProcessError::StdinWrite(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "injected stop failure",
            )))
        },
    )
    .await;
    let ProcessError::ProxyStartupCleanup { primary, cleanup } = failure else {
        panic!("expected incomplete startup cleanup");
    };
    assert!(matches!(
        *primary,
        ProcessError::PortInUse { port: 25565, .. }
    ));
    assert!(matches!(*cleanup, ProcessError::StdinWrite(_)));
    assert_eq!(
        primary
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .kind(),
        std::io::ErrorKind::AddrInUse
    );
    assert_eq!(
        cleanup
            .source()
            .unwrap()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .kind(),
        std::io::ErrorKind::BrokenPipe
    );
}
