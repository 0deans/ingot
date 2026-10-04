//! Map lifecycle notifications to the frontend's transport at the native boundary.
use crate::ipc::events::TauRpcEventsApiEventTrigger;
use crate::minecraft::launcher::InstanceStatusEvent;
use crate::running::QuitRequest;
use crate::version_change::crash::VersionChangeCrash;
use tauri::Runtime;

pub(crate) fn instance_status<R: Runtime>(app: &tauri::AppHandle<R>, event: InstanceStatusEvent) {
    if let Err(error) =
        TauRpcEventsApiEventTrigger::new(app.clone()).on_instance_status_changed(event)
    {
        eprintln!("[Events] Failed to emit instance status: {error}");
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

pub(super) fn quit_requested<R: Runtime>(
    app: &tauri::AppHandle<R>,
    request: QuitRequest,
) -> tauri::Result<()> {
    TauRpcEventsApiEventTrigger::new(app.clone()).on_quit_requested(request)
}
