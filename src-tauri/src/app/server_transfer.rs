use ingot_server::transfer::{self, TransferError, TransferService};
use ingot_server::ServerConfig;
use tauri::{Manager, Runtime};

pub(crate) fn service<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<TransferService, TransferError> {
    let servers_dir = super::server_config::get_servers_dir(app)?;
    Ok(TransferService {
        repository: crate::app::state(app).server_configs.clone(),
        servers_dir,
    })
}

pub(crate) fn upload_path<R: Runtime>(
    app: &tauri::AppHandle<R>,
    id: &str,
) -> Result<std::path::PathBuf, TransferError> {
    let cache = app.path().app_cache_dir().map_err(|error| {
        ingot_server::config::ConfigError::Directory(std::io::Error::other(error))
    })?;
    Ok(cache.join("imports").join(format!("{id}.zip")))
}

pub(crate) async fn import<R: Runtime>(
    app: &tauri::AppHandle<R>,
    upload_id: &str,
) -> Result<ServerConfig, TransferError> {
    let ownership = crate::app::state(app)
        .server_uploads
        .claim(upload_id)
        .await?;
    let archive = upload_path(app, upload_id)?;
    let service = service(app)?;
    tokio::task::spawn_blocking(move || transfer::uploads::finalize(ownership, &service, &archive))
        .await?
}

pub(crate) async fn duplicate<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: String,
    name: String,
    version: Option<String>,
    build: Option<String>,
) -> Result<ServerConfig, TransferError> {
    let ownership = crate::app::state(app)
        .servers
        .reserve_maintenance(&server_id)
        .await?;
    let service = service(app)?;
    run_owned(ownership, move || {
        let source = service
            .repository
            .load()
            .map_err(ingot_server::config::ConfigError::from)?
            .into_iter()
            .find(|server| server.id == server_id)
            .ok_or(ingot_server::config::ConfigError::InvalidId)?;
        service.duplicate(&source, &name, version.as_deref(), build)
    })
    .await?
}

fn run_owned<T: Send + 'static>(
    ownership: ingot_server::process::StartingGuard,
    operation: impl FnOnce() -> Result<T, TransferError> + Send + 'static,
) -> tokio::task::JoinHandle<Result<T, TransferError>> {
    tokio::task::spawn_blocking(move || {
        let _ownership = ownership;
        operation()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_waiter_cannot_release_a_running_copy_reservation() {
        let manager = ingot_server::ServerProcessManager::new();
        let ownership = manager.reserve_maintenance("source").await.unwrap();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = run_owned(ownership, move || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(())
        });
        entered_rx.await.unwrap();
        worker.abort();
        assert!(manager.reserve_maintenance("source").await.is_err());
        release_tx.send(()).unwrap();
        worker.await.unwrap().unwrap();
        assert!(manager.reserve_maintenance("source").await.is_ok());
    }

    #[test]
    fn service_uses_shared_servers_dir_and_preserves_config_recovery_errors() {
        use crate::app::{AppPaths, AppState};
        use ingot_core::metadata::MetadataError;
        use ingot_server::config::ConfigError;

        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(AppPaths::new(dir.path().to_owned())).unwrap();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();

        let resolved = service(app.handle()).unwrap();
        let expected_dir = app.path().app_data_dir().unwrap().join("servers");
        assert_eq!(resolved.servers_dir, expected_dir);
        assert!(resolved.servers_dir.is_dir());

        let blocked_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(blocked_dir.path().join("servers.deletion.json")).unwrap();
        let blocked_state = AppState::new(AppPaths::new(blocked_dir.path().to_owned())).unwrap();
        let blocked_app = tauri::test::mock_builder()
            .manage(blocked_state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();

        let err = service(blocked_app.handle())
            .err()
            .expect("recovery-blocked servers store must propagate ConfigError");
        assert!(matches!(
            err,
            TransferError::Config(ConfigError::Metadata(MetadataError::RecoveryBlocked(_)))
        ));
    }
}
