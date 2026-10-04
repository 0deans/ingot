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
            launcher::adopt_running(app.clone(), get_process_manager(app).clone(), on_status).await;
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
pub(crate) async fn stop_all_servers<R: Runtime>(app: &tauri::AppHandle<R>) {
    let pm = get_server_process_manager(app);
    let supervisor = get_server_supervisor_manager(app);
    // Sleeping ones only have a proxy; stop those too so nothing wakes up meanwhile
    for id in pm.sleeping_ids().await {
        supervisor.release(&id).await;
        pm.set_server_status(&id, server::ServerStatus::Stopped)
            .await;
    }
    // A server still starting may get its process meanwhile: look again after each round
    for _ in 0..3 {
        let ids = pm.process_ids().await;
        if ids.is_empty() {
            break;
        }
        let mut waits = Vec::new();
        for id in ids {
            supervisor.release(&id).await;
            let _ = pm
                .stop_server_within(&id, std::time::Duration::from_secs(30))
                .await;
            let pm = pm.clone();
            waits.push(tokio::spawn(async move {
                pm.wait_stopped(&id, std::time::Duration::from_secs(35))
                    .await
            }));
        }
        for wait in waits {
            let _ = wait.await;
        }
    }
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
            stop_all_servers(&app).await;
            quit_now(&app);
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
}

pub(crate) async fn stop_leftover_server<R: Runtime>(
    app: &tauri::AppHandle<R>,
    server_id: String,
) -> Result<(), LifecycleError> {
    let leftover = leftover_servers(app)
        .await
        .into_iter()
        .find(|l| l.server_id == server_id)
        .ok_or(LifecycleError::LeftoverServerMissing)?;
    crate::running::kill_tree(leftover.pid);
    // Wait for it to be gone, so the port and the world are free for a start
    for _ in 0..20 {
        if crate::running::process_start(leftover.pid).is_none() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    crate::running::remove(app, crate::running::Kind::Server, &server_id, leftover.pid);
    Ok(())
}

#[cfg(test)]
mod tests;
