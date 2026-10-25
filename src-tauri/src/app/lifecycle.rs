//! Native lifecycle orchestration; transport handlers only delegate here.
use super::{events, state};
use crate::minecraft::launcher::{self, InstanceStatusEvent};
use crate::running::{BusyServer, LeftoverServer, QuitRequest};
use crate::server;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Runtime;

#[derive(Default)]
pub(crate) struct LifecycleState {
    quitting: AtomicBool,
    games_adopted: tokio::sync::OnceCell<()>,
}

impl LifecycleState {
    async fn adopt_games_once<F, Fut>(&self, adopt: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        self.games_adopted.get_or_init(adopt).await;
    }

    pub(crate) fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::SeqCst)
    }
}

fn get_process_manager<R: Runtime>(app: &tauri::AppHandle<R>) -> &launcher::ProcessManager {
    &state(app).games
}

fn get_server_process_manager<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> &server::ServerProcessManager {
    &state(app).servers
}

fn get_server_supervisor_manager<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> &server::ServerSupervisorManager {
    &state(app).supervisor
}

/// Re-attaches the games that were running when Ingot last closed (once per run)
pub(crate) async fn adopt_running_games<R: Runtime>(app: &tauri::AppHandle<R>) {
    state(app)
        .lifecycle
        .adopt_games_once(|| async {
            let app_stat = app.clone();
            let on_status = std::sync::Arc::new(move |ev: InstanceStatusEvent| {
                events::instance_status(&app_stat, ev);
            });
            crate::app::games::adopt_running(
                app.clone(),
                get_process_manager(app).clone(),
                on_status,
            )
            .await;
        })
        .await;
}

/// Servers that are running, starting or sleeping
pub(crate) async fn busy_servers<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<BusyServer> {
    let pm = get_server_process_manager(app);
    let configs = server::load_servers(app).unwrap_or_default();
    let mut busy = Vec::new();
    for config in configs {
        let status = pm.get_server_status(&config.id).await;
        if let Some(server) = quit_blocker(config.id, config.name, status) {
            busy.push(server);
        }
    }
    busy
}

fn quit_blocker(
    server_id: String,
    name: String,
    status: server::ServerStatus,
) -> Option<BusyServer> {
    if status == server::ServerStatus::Stopped {
        return None;
    }
    Some(BusyServer {
        server_id,
        name,
        sleeping: status == server::ServerStatus::Sleeping,
    })
}

pub(crate) async fn quit_request<R: Runtime>(app: &tauri::AppHandle<R>) -> QuitRequest {
    QuitRequest {
        servers: busy_servers(app).await,
        games: get_process_manager(app).get_running_instances().await.len() as u32,
    }
}

/// Stops every server and waits for them: each gets 30 seconds to save its worlds
/// before it's ended
pub(crate) async fn stop_all_servers<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), LifecycleError> {
    let app = app.clone();
    tokio::spawn(async move {
        let pm = get_server_process_manager(&app);
        let _admission = pm.pause_starts();
        let supervisor = get_server_supervisor_manager(&app);
        for id in pm.sleeping_ids().await {
            supervisor.release(&id).await;
            pm.set_server_status(&id, server::ServerStatus::Stopped)
                .await;
        }
        let mut jobs = Vec::new();
        for id in pm.process_ids().await {
            supervisor.release(&id).await;
            let manager = pm.clone();
            let server_id = id.clone();
            let work = tokio::spawn(async move {
                match manager
                    .stop_server_and_wait(&server_id, std::time::Duration::from_secs(30))
                    .await
                {
                    // A process may exit normally before this worker claims its shutdown.
                    Err(server::process::ProcessError::NotRunning(_)) => Ok(()),
                    result => result,
                }
            });
            jobs.push(ShutdownJob {
                server_id: id,
                work,
            });
        }
        collect_shutdowns(jobs).await
    })
    .await
    .map_err(LifecycleError::ShutdownWorker)?
}

struct ShutdownJob {
    server_id: String,
    work: tokio::task::JoinHandle<Result<(), server::process::ProcessError>>,
}

#[derive(Debug, thiserror::Error)]
#[error("Server {server_id} could not be stopped: {source}")]
pub(crate) struct ServerShutdownFailure {
    server_id: String,
    #[source]
    source: server::process::ProcessError,
}

#[derive(Debug)]
pub(crate) struct ShutdownFailures(Vec<ServerShutdownFailure>);

impl std::fmt::Display for ShutdownFailures {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, failure) in self.0.iter().enumerate() {
            if index > 0 {
                write!(formatter, "; ")?;
            }
            write!(formatter, "{failure}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ShutdownFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0
            .first()
            .map(|failure| failure as &dyn std::error::Error)
    }
}

async fn collect_shutdowns(jobs: Vec<ShutdownJob>) -> Result<(), LifecycleError> {
    let mut failures = Vec::new();
    for job in jobs {
        let result = match job.work.await {
            Ok(result) => result,
            Err(source) => Err(server::process::ProcessError::ShutdownWorker(source)),
        };
        if let Err(source) = result {
            failures.push(ServerShutdownFailure {
                server_id: job.server_id,
                source,
            });
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    Err(LifecycleError::Servers(ShutdownFailures(failures)))
}

/// Quits for real, past the checks
pub(crate) fn quit_now<R: Runtime>(app: &tauri::AppHandle<R>) {
    state(app).lifecycle.mark_quitting();
    app.exit(0);
}

pub(crate) fn is_quitting<R: Runtime>(app: &tauri::AppHandle<R>) -> bool {
    state(app).lifecycle.is_quitting()
}

/// Closing the window, Quit in the tray, or any other exit: quits right away unless a
/// server runs, then shows the window and asks what to do
pub(crate) fn request_quit<R: Runtime>(app: &tauri::AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let request = quit_request(&app).await;
        if request.servers.is_empty() {
            quit_now(&app);
            return;
        }
        #[cfg(desktop)]
        crate::tray::restore_main_window(&app);
        if let Err(e) = events::quit_requested(&app, request) {
            eprintln!("[IPC] Failed to emit on_quit_requested: {e}");
            // Nobody to ask: never leave a server behind without its console
            match stop_all_servers(&app).await {
                Ok(()) => quit_now(&app),
                Err(error) => {
                    eprintln!("Application remains open because server shutdown failed: {error}");
                    #[cfg(desktop)]
                    crate::tray::restore_main_window(&app);
                }
            }
        }
    });
}

/// Servers from the saved list that are still running but aren't Ingot's children now
pub(crate) async fn leftover_servers<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<LeftoverServer> {
    let pm = get_server_process_manager(app);
    let configs = server::load_servers(app).unwrap_or_default();
    let mut leftovers = Vec::new();
    for entry in crate::running::load(app, crate::running::Kind::Server) {
        if pm.owns(&entry.id, entry.pid).await {
            continue;
        }
        if !crate::running::is_alive(&entry) {
            crate::running::remove(app, crate::running::Kind::Server, &entry.id, entry.pid);
            continue;
        }
        let name = configs
            .iter()
            .find(|c| c.id == entry.id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "A server".to_string());
        leftovers.push(LeftoverServer {
            server_id: entry.id,
            name,
            pid: entry.pid,
            started_at: entry.started_at,
        });
    }
    leftovers
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LifecycleError {
    #[error("That server isn't running any more")]
    LeftoverServerMissing,
    #[error("Failed to terminate leftover server: {0}")]
    Termination(#[source] crate::process_tree::TerminationError),
    #[error("Leftover server shutdown worker failed: {0}")]
    ShutdownWorker(#[source] tokio::task::JoinError),
    #[error("Some servers remain unconfirmed: {0}")]
    Servers(#[source] ShutdownFailures),
}

pub(crate) async fn stop_leftover_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: String,
) -> Result<(), LifecycleError> {
    let entry = crate::running::load(app, crate::running::Kind::Server)
        .into_iter()
        .find(|entry| entry.id == server_id)
        .ok_or(LifecycleError::LeftoverServerMissing)?;
    if get_server_process_manager(app)
        .owns(&entry.id, entry.pid)
        .await
    {
        return Err(LifecycleError::LeftoverServerMissing);
    }
    let app = app.clone();
    stop_leftover_owned(entry, crate::process_tree::terminate, move |original| {
        crate::running::remove_original(&app, crate::running::Kind::Server, &original)
    })
    .await
}

async fn stop_leftover_owned<F, Fut, A>(
    original: crate::running::Entry,
    terminate: F,
    acknowledge: A,
) -> Result<(), LifecycleError>
where
    F: FnOnce(crate::process_tree::ProcessIdentity) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), crate::process_tree::TerminationError>>
        + Send
        + 'static,
    A: FnOnce(crate::running::Entry) + Send + 'static,
{
    // The worker owns both termination and acknowledgement after a caller disconnects.
    tokio::spawn(async move {
        let identity = crate::process_tree::ProcessIdentity::new(original.pid, original.proc_start);
        if let Err(error) = terminate(identity).await {
            eprintln!(
                "Leftover server {} termination failed: {error}",
                original.id
            );
            return Err(LifecycleError::Termination(error));
        }
        acknowledge(original);
        Ok(())
    })
    .await
    .map_err(LifecycleError::ShutdownWorker)?
}

#[cfg(test)]
mod tests;
