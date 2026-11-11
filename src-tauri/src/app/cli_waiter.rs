//! Local CLI waiter IPC so Steam shortcuts stay alive while a game runs on an already-open
//! Ingot instance, and forward Steam's "Stop" termination to that specific instance.

mod endpoint;
mod native;
mod protocol;
#[cfg(test)]
mod tests;
mod transport;

use std::future::Future;
use tauri::{AppHandle, Runtime};

pub(crate) struct ReservedLaunch {
    pub(crate) reservation_id: u64,
    pub(crate) launch_result: tokio::sync::oneshot::Receiver<Result<u32, String>>,
}

pub(crate) trait WaiterSessionHost: Send + Sync + 'static {
    fn start_launch(
        &self,
        instance_id: String,
        steam_env: Vec<(String, String)>,
    ) -> impl Future<Output = Result<ReservedLaunch, String>> + Send;

    fn wait_for_reservation_end(
        &self,
        instance_id: String,
        reservation_id: u64,
    ) -> impl Future<Output = ()> + Send;

    fn cancel_or_kill_reservation(
        &self,
        instance_id: String,
        reservation_id: u64,
    ) -> impl Future<Output = ()> + Send;
}

/// Checks if `--launch-instance <id>` was requested and an existing Ingot instance is listening.
/// When delegated, blocks until the game finishes and returns `Some(exit_code)`.
/// Returns `None` if not a CLI launch or if no running Ingot instance is reachable.
pub(crate) async fn try_delegate_to_running_instance(args: &[String]) -> Option<i32> {
    let instance_id = super::cli::parse_cli_launch_instance(args)?;
    let endpoint_path = super::paths::default_cli_waiter_file()?;
    let steam_env = protocol::collect_steam_env(std::env::vars());
    transport::delegate_launch_with_file(&endpoint_path, &instance_id, steam_env).await
}

pub(crate) fn start_server<R: Runtime>(app: &AppHandle<R>) {
    native::start_server(app);
}

pub(crate) fn cleanup_endpoint_file<R: Runtime>(app: &AppHandle<R>) {
    native::cleanup_endpoint_file(app);
}
