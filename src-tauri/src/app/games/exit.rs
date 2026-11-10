use crate::app::instances::{ensure_instance_dir, update_last_played};
use crate::minecraft::instance::InstanceConfig;
use crate::minecraft::launcher::models::{InstanceStatusEvent, StatusFn};
use crate::minecraft::launcher::process::ProcessManager;
use crate::minecraft::launcher::quick_play::cleanup_quick_play_junction;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PostExitAction {
    QuitApp,
    RestoreWindow,
    StayHidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PostExitContext {
    pub(crate) direct_launch_session: bool,
    pub(crate) launched_from_cli: bool,
    pub(crate) hide_to_tray: bool,
    pub(crate) window_visible: bool,
    pub(crate) has_busy_servers: bool,
    pub(crate) had_version_crash: bool,
}

pub(crate) fn decide_post_exit_action(ctx: PostExitContext) -> PostExitAction {
    if ctx.had_version_crash {
        return PostExitAction::RestoreWindow;
    }
    if ctx.direct_launch_session {
        if ctx.has_busy_servers {
            return PostExitAction::StayHidden;
        }
        return PostExitAction::QuitApp;
    }
    if ctx.launched_from_cli {
        return PostExitAction::StayHidden;
    }
    if ctx.hide_to_tray || !ctx.window_visible {
        return PostExitAction::RestoreWindow;
    }
    PostExitAction::StayHidden
}

pub fn handle_launcher_behavior_on_launch<R: Runtime>(
    app: &AppHandle<R>,
    behavior: &str,
    instance_id: &str,
    launched_from_cli: bool,
) {
    if crate::app::lifecycle::is_direct_launch_session(app) || launched_from_cli {
        if matches!(
            behavior,
            crate::system::BEHAVIOR_HIDE_TO_TRAY | crate::system::BEHAVIOR_CLOSE
        ) {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        return;
    }

    match behavior {
        crate::system::BEHAVIOR_HIDE_TO_TRAY => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        crate::system::BEHAVIOR_CLOSE => {
            let _ = update_last_played(app, instance_id, 0);
            let app_clone = app.clone();
            tokio::spawn(async move {
                if crate::app::lifecycle::busy_servers(&app_clone)
                    .await
                    .is_empty()
                {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    crate::app::lifecycle::quit_now(&app_clone);
                } else if let Some(window) = app_clone.get_webview_window("main") {
                    let _ = window.hide();
                }
            });
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn finish_game<R: Runtime>(
    app: &AppHandle<R>,
    pm: &ProcessManager,
    instance: &InstanceConfig,
    pid: u32,
    started_at: u64,
    ended_at: u64,
    clean: bool,
    exit_code: Option<i32>,
    launched_from_cli: bool,
    on_status: &StatusFn,
) {
    let instance_id = instance.id.clone();
    let Some(exit_ownership) = pm.claim_exit(&instance_id, pid) else {
        return;
    };
    crate::running::remove(app, crate::running::Kind::Game, &instance_id, pid);

    let mut had_version_crash = false;
    if let Ok(instance_dir) = ensure_instance_dir(app, &instance_id) {
        cleanup_quick_play_junction(&instance_dir);

        if let Err(e) = crate::app::sync::sync_after_exit(app, instance, &instance_dir) {
            eprintln!("[Launcher] Warning: Post-exit sync failed: {e}");
        }

        let since = UNIX_EPOCH + std::time::Duration::from_secs(started_at.saturating_sub(1));
        if let Some(crash) = crate::artifacts::crash::after_exit(
            &instance_dir,
            clean,
            crate::version_change::TargetKind::Instance,
            &instance_id,
            &instance.name,
            &["mods"],
            since,
        ) {
            had_version_crash = true;
            crate::app::events::version_change_crash(app, crash);
        } else if let Some(crash) = crate::minecraft::logs::analyze_instance_crash(
            &instance_dir,
            &instance_id,
            &instance.name,
            clean,
            exit_code,
            since,
        ) {
            had_version_crash = true;
            crate::app::events::instance_crash(app, crash);
        }
    }

    let _ = update_last_played(app, &instance_id, ended_at.saturating_sub(started_at));

    on_status(InstanceStatusEvent {
        instance_id,
        is_running: false,
        pid: 0,
        started_at: 0,
    });
    drop(exit_ownership);
    let no_more_running = !pm.has_active_entries();

    let hide_to_tray = match crate::system::load_settings(app) {
        Ok(settings) => settings.launcher_behavior.as_str() == crate::system::BEHAVIOR_HIDE_TO_TRAY,
        Err(error) => {
            eprintln!("[Settings] Cannot read launcher behavior on game exit: {error}");
            false
        }
    };

    if no_more_running {
        let window_visible = app
            .get_webview_window("main")
            .map(|w| w.is_visible().unwrap_or(true))
            .unwrap_or(false);
        let has_busy_servers = !crate::app::lifecycle::busy_servers(app).await.is_empty();
        let action = decide_post_exit_action(PostExitContext {
            direct_launch_session: crate::app::lifecycle::is_direct_launch_session(app),
            launched_from_cli,
            hide_to_tray,
            window_visible,
            has_busy_servers,
            had_version_crash,
        });
        match action {
            PostExitAction::QuitApp => {
                crate::app::lifecycle::quit_now(app);
            }
            PostExitAction::RestoreWindow => {
                #[cfg(desktop)]
                crate::tray::restore_main_window(app);
                #[cfg(not(desktop))]
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            PostExitAction::StayHidden => {}
        }
    }
}

pub async fn adopt_running<R: Runtime>(app: AppHandle<R>, pm: ProcessManager, on_status: StatusFn) {
    let entries = crate::running::load(&app, crate::running::Kind::Game);
    if entries.is_empty() {
        return;
    }
    let instances = match crate::app::instances::load_instances(&app) {
        Ok(instances) => instances,
        Err(error) => {
            eprintln!("[Launcher] Cannot inspect running games: {error}");
            return;
        }
    };
    for entry in entries {
        let Some(instance) = instances.iter().find(|i| i.id == entry.id).cloned() else {
            crate::running::remove(&app, crate::running::Kind::Game, &entry.id, entry.pid);
            continue;
        };

        if !pm.adopt_if_vacant(
            &entry.id,
            entry.pid,
            entry.started_at,
            Some(entry.proc_start),
        ) {
            continue;
        }
        if !crate::running::is_alive(&entry) {
            let ended = ensure_instance_dir(&app, &entry.id)
                .ok()
                .and_then(|dir| fs::metadata(dir.join("logs").join("latest.log")).ok())
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .filter(|&t| t > entry.started_at)
                .unwrap_or(entry.started_at);
            finish_game(
                &app,
                &pm,
                &instance,
                entry.pid,
                entry.started_at,
                ended,
                true,
                Some(0),
                false,
                &on_status,
            )
            .await;
            continue;
        }

        println!(
            "[Launcher] Re-attached to {} (pid {})",
            instance.name, entry.pid
        );
        on_status(InstanceStatusEvent {
            instance_id: entry.id.clone(),
            is_running: true,
            pid: entry.pid,
            started_at: entry.started_at,
        });

        let (app, pm, on_status) = (app.clone(), pm.clone(), on_status.clone());
        tokio::spawn(async move {
            while crate::running::is_alive(&entry) {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            let ended = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(entry.started_at);
            finish_game(
                &app,
                &pm,
                &instance,
                entry.pid,
                entry.started_at,
                ended,
                true,
                Some(0),
                false,
                &on_status,
            )
            .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_direct_launch_quits_app_when_idle_and_clean() {
        let action = decide_post_exit_action(PostExitContext {
            direct_launch_session: true,
            launched_from_cli: true,
            hide_to_tray: false,
            window_visible: false,
            has_busy_servers: false,
            had_version_crash: false,
        });
        assert_eq!(action, PostExitAction::QuitApp);
    }

    #[test]
    fn cold_direct_launch_stays_hidden_when_servers_are_busy() {
        let action = decide_post_exit_action(PostExitContext {
            direct_launch_session: true,
            launched_from_cli: true,
            hide_to_tray: false,
            window_visible: false,
            has_busy_servers: true,
            had_version_crash: false,
        });
        assert_eq!(action, PostExitAction::StayHidden);
    }

    #[test]
    fn version_crash_restores_window_even_in_direct_launch_session() {
        let action = decide_post_exit_action(PostExitContext {
            direct_launch_session: true,
            launched_from_cli: true,
            hide_to_tray: false,
            window_visible: false,
            has_busy_servers: false,
            had_version_crash: true,
        });
        assert_eq!(action, PostExitAction::RestoreWindow);
    }

    #[test]
    fn warm_cli_launch_does_not_quit_or_pop_hidden_tray_window() {
        let action = decide_post_exit_action(PostExitContext {
            direct_launch_session: false,
            launched_from_cli: true,
            hide_to_tray: true,
            window_visible: false,
            has_busy_servers: false,
            had_version_crash: false,
        });
        assert_eq!(action, PostExitAction::StayHidden);
    }

    #[test]
    fn normal_gui_launch_restores_window_when_hidden_to_tray() {
        let action = decide_post_exit_action(PostExitContext {
            direct_launch_session: false,
            launched_from_cli: false,
            hide_to_tray: true,
            window_visible: false,
            has_busy_servers: false,
            had_version_crash: false,
        });
        assert_eq!(action, PostExitAction::RestoreWindow);
    }
}
