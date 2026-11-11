//! Map lifecycle notifications to the frontend's transport at the native boundary.
use crate::ipc::events::TauRpcEventsApiEventTrigger;
use crate::running::{LeftoverServer, QuitRequest};
use ingot_artifacts::crash::VersionChangeCrash;
use ingot_minecraft::launcher::{InstanceStatusEvent, LaunchProgressEvent};
use ingot_minecraft::logs::{InstanceCrashEvent, InstanceLogEvent};
use ingot_server::{ServerLogEvent, ServerStatusEvent};
use tauri::Runtime;

pub(crate) fn instance_status<R: Runtime>(app: &tauri::AppHandle<R>, event: InstanceStatusEvent) {
    if let Err(error) =
        TauRpcEventsApiEventTrigger::new(app.clone()).on_instance_status_changed(event)
    {
        eprintln!("[Events] Failed to emit instance status: {error}");
    }
}

pub(crate) fn instance_log<R: Runtime>(app: &tauri::AppHandle<R>, event: InstanceLogEvent) {
    if let Err(error) = TauRpcEventsApiEventTrigger::new(app.clone()).on_instance_log(event) {
        eprintln!("[Events] Failed to emit instance log: {error}");
    }
}

pub(crate) fn instance_crash<R: Runtime>(app: &tauri::AppHandle<R>, event: InstanceCrashEvent) {
    if let Err(error) = TauRpcEventsApiEventTrigger::new(app.clone()).on_instance_crash(event) {
        eprintln!("[Events] Failed to emit instance crash: {error}");
    }
}

pub(crate) fn launch_progress<R: Runtime>(app: &tauri::AppHandle<R>, event: LaunchProgressEvent) {
    if let Err(error) = TauRpcEventsApiEventTrigger::new(app.clone()).on_launch_progress(event) {
        eprintln!("[Events] Failed to emit launch progress: {error}");
    }
}

pub(crate) fn version_change_crash<R: Runtime>(
    app: &tauri::AppHandle<R>,
    event: VersionChangeCrash,
) {
    if let Err(error) = TauRpcEventsApiEventTrigger::new(app.clone()).on_version_change_crash(event)
    {
        eprintln!("[Events] Failed to emit version-change crash: {error}");
    }
}

pub(crate) fn server_status<R: Runtime>(app: &tauri::AppHandle<R>, event: ServerStatusEvent) {
    if let Err(error) =
        TauRpcEventsApiEventTrigger::new(app.clone()).on_server_status_changed(event)
    {
        eprintln!("[Events] Failed to emit server status: {error}");
    }
}

pub(crate) fn server_log<R: Runtime>(app: &tauri::AppHandle<R>, event: ServerLogEvent) {
    if let Err(error) = TauRpcEventsApiEventTrigger::new(app.clone()).on_server_log(event) {
        eprintln!("[Events] Failed to emit server log: {error}");
    }
}

pub(crate) fn leftover_servers<R: Runtime>(
    app: &tauri::AppHandle<R>,
    servers: Vec<LeftoverServer>,
) {
    if let Err(error) = TauRpcEventsApiEventTrigger::new(app.clone()).on_leftover_servers(servers) {
        eprintln!("[Events] Failed to emit leftover servers: {error}");
    }
}

pub(super) fn quit_requested<R: Runtime>(
    app: &tauri::AppHandle<R>,
    request: QuitRequest,
) -> tauri::Result<()> {
    TauRpcEventsApiEventTrigger::new(app.clone()).on_quit_requested(request)
}
