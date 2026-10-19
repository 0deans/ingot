use crate::app::instances::{ensure_instance_dir, update_last_played};
use crate::minecraft::instance::InstanceConfig;
use crate::minecraft::launcher::models::{InstanceStatusEvent, StatusFn};
use crate::minecraft::launcher::process::ProcessManager;
use crate::minecraft::launcher::quick_play::cleanup_quick_play_junction;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

pub fn handle_launcher_behavior_on_launch<R: Runtime>(
    app: &AppHandle<R>,
    behavior: &str,
    instance_id: &str,
) {
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
    on_status: &StatusFn,
) {
    let instance_id = instance.id.clone();
    let Some(exit_ownership) = pm.claim_exit(&instance_id, pid) else {
        return;
    };
    crate::running::remove(app, crate::running::Kind::Game, &instance_id, pid);

    if let Ok(instance_dir) = ensure_instance_dir(app, &instance_id) {
        cleanup_quick_play_junction(&instance_dir);

        if let Err(e) = crate::app::sync::sync_after_exit(app, instance, &instance_dir) {
            eprintln!("[Launcher] Warning: Post-exit sync failed: {e}");
        }

        let since = UNIX_EPOCH + std::time::Duration::from_secs(started_at.saturating_sub(1));
        if let Some(crash) = crate::version_change::crash::after_exit(
            &instance_dir,
            clean,
            crate::version_change::TargetKind::Instance,
            &instance_id,
            &instance.name,
            &["mods"],
            since,
        ) {
            crate::app::events::version_change_crash(app, crash);
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
        let should_restore = hide_to_tray
            || app
                .get_webview_window("main")
                .map(|w| !w.is_visible().unwrap_or(true))
                .unwrap_or(false);
        if should_restore {
            #[cfg(desktop)]
            crate::tray::restore_main_window(app);
            #[cfg(not(desktop))]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
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

        if !pm.register_if_vacant(&entry.id, entry.pid, entry.started_at) {
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
                &on_status,
            )
            .await;
        });
    }
}
