use crate::server::transfer::{self, TransferError, TransferService};
use crate::server::ServerConfig;
use tauri::{Manager, Runtime};

pub(crate) fn service<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<TransferService, TransferError> {
    let parent = app
        .path()
        .app_data_dir()
        .map_err(|error| crate::server::config::ConfigError::Directory(error.to_string()))?
        .join("servers");
    Ok(TransferService {
        repository: crate::app::state(app).server_configs.clone(),
        servers_dir: parent,
    })
}

pub(crate) fn upload_path<R: Runtime>(
    app: &tauri::AppHandle<R>,
    id: &str,
) -> Result<std::path::PathBuf, TransferError> {
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|error| crate::server::config::ConfigError::Directory(error.to_string()))?;
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
            .map_err(crate::server::config::ConfigError::from)?
            .into_iter()
            .find(|server| server.id == server_id)
            .ok_or(crate::server::config::ConfigError::InvalidId)?;
        service.duplicate(&source, &name, version.as_deref(), build)
    })
    .await?
}

fn run_owned<T: Send + 'static>(
    ownership: crate::server::process::StartingGuard,
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
        let manager = crate::server::ServerProcessManager::new();
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
}
