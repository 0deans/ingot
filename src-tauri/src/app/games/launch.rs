use super::exit::{finish_game, handle_launcher_behavior_on_launch};
use super::preparation::NativePreparation;
use crate::app::instances::ensure_instance_dir;
use crate::system;
use ingot_minecraft::instance::InstanceConfig;
use ingot_minecraft::launcher::models::{
    InstanceStatusEvent, LaunchError, LaunchProgressEvent, QuickPlayOptions, StatusFn,
};
use ingot_minecraft::launcher::prepare::prepare_launch_environment;
use ingot_minecraft::launcher::process::ProcessManager;
use ingot_minecraft::launcher::progress::make_progress_reporter;
use ingot_minecraft::launcher::service::{
    prepare_game_command, spawn_and_commit_process, CommandContext,
};
use ingot_minecraft::logs::{InstanceLogEvent, LogLevel};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Default)]
pub(crate) struct LaunchSessionOptions {
    pub(crate) quick_play: Option<QuickPlayOptions>,
    pub(crate) launched_from_cli: bool,
    pub(crate) extra_env: Vec<(String, String)>,
    pub(crate) on_reserved: Option<tokio::sync::oneshot::Sender<u64>>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum GameLaunchError {
    #[error(transparent)]
    Launcher(#[from] LaunchError),

    #[error("Version change transaction is in incomplete state: {0:?}. Please recover or revert it first.")]
    IncompleteTransaction(ingot_version_change::TransactionPhase),

    #[error("Cannot inspect version change recovery state: {0}")]
    TransactionInspection(#[source] Box<ingot_version_change::VersionChangeError>),
}

impl GameLaunchError {
    pub(crate) fn is_reservation_cancelled(&self) -> bool {
        matches!(self, Self::Launcher(LaunchError::ReservationCancelled))
    }
}

pub(crate) async fn launch_minecraft_with_options<R: Runtime, FProg, FStatus>(
    app: AppHandle<R>,
    process_manager: ProcessManager,
    client: reqwest::Client,
    instance_id: String,
    options: LaunchSessionOptions,
    on_progress: FProg,
    on_status: FStatus,
) -> Result<u32, GameLaunchError>
where
    FProg: Fn(LaunchProgressEvent) + Send + Sync + 'static,
    FStatus: Fn(InstanceStatusEvent) + Send + Sync + 'static,
{
    let LaunchSessionOptions {
        quick_play,
        launched_from_cli,
        extra_env,
        on_reserved,
    } = options;
    let reservation = reserve_instance_launch(
        &crate::app::state(&app).instances,
        &process_manager,
        &instance_id,
    )
    .await?;
    let instance = reservation.instance;
    let starting_guard = reservation.ownership;
    if let Some(tx) = on_reserved {
        let _ = tx.send(starting_guard.reservation_id());
    }

    let report_prog = make_progress_reporter(instance_id.clone(), on_progress);

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|source| LaunchError::AppDataDir(std::io::Error::other(source)))?;

    let cache_dir = data_dir.join("cache");
    let assets_dir = data_dir.join("assets");
    let libraries_dir = data_dir.join("libraries");
    let instance_dir = ensure_instance_dir(&app, &instance_id).map_err(LaunchError::InstanceDir)?;
    ensure_transaction_ready(&instance_dir)?;

    let global_settings = system::load_settings(&app).map_err(LaunchError::Settings)?;

    let (mut command, _session_guard) =
        if instance.loader == ingot_minecraft::instance::ModLoaderType::Horizon {
            super::horizon::prepare_horizon_game_command(
                &app,
                &client,
                &instance,
                &instance_dir,
                &data_dir,
                &report_prog,
            )
            .await?
        } else {
            let prep = prepare_launch_environment(
                &NativePreparation::new(&app),
                &client,
                &instance,
                &cache_dir,
                &assets_dir,
                &libraries_dir,
                report_prog.clone(),
            )
            .await?;

            let context = CommandContext {
                instance: &instance,
                settings: &global_settings,
                instance_dir: &instance_dir,
                cache_dir: &cache_dir,
                assets_dir: &assets_dir,
                libraries_dir: &libraries_dir,
                quick_play: quick_play.as_ref(),
            };
            (
                prepare_game_command(&client, &prep, context, &report_prog).await?,
                None,
            )
        };

    for (key, value) in &extra_env {
        command.env(key, value);
    }

    // Run pre-launch synchronization
    if let Err(e) = crate::app::sync::sync_before_launch(&app, &instance, &instance_dir) {
        eprintln!("[Launcher] Warning: Pre-launch sync failed: {e}");
    }

    report_prog("Starting Game", 10, 10, "Launching Minecraft process...");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    process_manager.clear_instance_logs(&instance_id);
    let launch_banner = format!(
        "[Ingot] Launching {} ({} {:?})...",
        instance.name, instance.game_version, instance.loader
    );
    let banner_line =
        process_manager.append_instance_log(&instance_id, &launch_banner, LogLevel::Launcher);
    crate::app::events::instance_log(
        &app,
        InstanceLogEvent {
            instance_id: instance_id.clone(),
            line: banner_line,
        },
    );

    let (mut child, pid) = spawn_and_commit_process(command, starting_guard, now).await?;
    crate::running::add(&app, crate::running::Kind::Game, &instance_id, pid, now);

    let mut reader_handles = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let pm_out = process_manager.clone();
        let app_out = app.clone();
        let id_out = instance_id.clone();
        reader_handles.push(tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(raw)) = lines.next_line().await {
                let line = pm_out.append_instance_log(&id_out, &raw, LogLevel::Info);
                crate::app::events::instance_log(
                    &app_out,
                    InstanceLogEvent {
                        instance_id: id_out.clone(),
                        line,
                    },
                );
            }
        }));
    }

    if let Some(stderr) = child.stderr.take() {
        let pm_err = process_manager.clone();
        let app_err = app.clone();
        let id_err = instance_id.clone();
        reader_handles.push(tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(raw)) = lines.next_line().await {
                let line = pm_err.append_instance_log(&id_err, &raw, LogLevel::Info);
                crate::app::events::instance_log(
                    &app_err,
                    InstanceLogEvent {
                        instance_id: id_err.clone(),
                        line,
                    },
                );
            }
        }));
    }

    on_status(InstanceStatusEvent {
        instance_id: instance_id.clone(),
        is_running: true,
        pid,
        started_at: now,
    });

    handle_launcher_behavior_on_launch(
        &app,
        global_settings.launcher_behavior.as_str(),
        &instance_id,
        launched_from_cli,
    );

    // Background task to monitor child exit
    let pm_clone = process_manager.clone();
    let app_clone = app.clone();
    let on_status_clone: StatusFn = Arc::new(on_status);
    let inst_clone = instance.clone();

    tokio::spawn(async move {
        let _guard = _session_guard;
        let (clean, exit_code) = match child.wait().await {
            Ok(status) => {
                println!(
                    "[Launcher] Minecraft process exited with status: {}",
                    status
                );
                (status.success(), status.code())
            }
            Err(e) => {
                eprintln!("[Launcher] Error waiting for Minecraft process: {e}");
                (false, None)
            }
        };
        for mut handle in reader_handles {
            if tokio::time::timeout(Duration::from_secs(1), &mut handle)
                .await
                .is_err()
            {
                handle.abort();
            }
        }
        let exit_msg = match exit_code {
            Some(0) => "[Ingot] Process exited normally (code 0).".to_string(),
            Some(code) => format!("[Ingot] Failed: Process exited with code {code}."),
            None => "[Ingot] Process terminated.".to_string(),
        };
        let exit_line = pm_clone.append_instance_log(&inst_clone.id, &exit_msg, LogLevel::Launcher);
        crate::app::events::instance_log(
            &app_clone,
            InstanceLogEvent {
                instance_id: inst_clone.id.clone(),
                line: exit_line,
            },
        );
        let ended = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(now);
        finish_game(
            &app_clone,
            &pm_clone,
            &inst_clone,
            pid,
            now,
            ended,
            clean,
            exit_code,
            launched_from_cli,
            &on_status_clone,
        )
        .await;
    });

    Ok(pid)
}

fn ensure_transaction_ready(instance_dir: &std::path::Path) -> Result<(), GameLaunchError> {
    let incomplete = ingot_version_change::detect_incomplete_transaction(instance_dir)
        .map_err(|source| GameLaunchError::TransactionInspection(Box::new(source)))?;
    if let Some(transaction) = incomplete {
        return Err(GameLaunchError::IncompleteTransaction(transaction.phase));
    }
    Ok(())
}

struct InstanceLaunchReservation {
    ownership: ingot_minecraft::launcher::process::StartingGuard,
    instance: InstanceConfig,
}

async fn reserve_instance_launch(
    repository: &ingot_core::metadata::MetadataRepository<InstanceConfig>,
    processes: &ProcessManager,
    id: &str,
) -> Result<InstanceLaunchReservation, LaunchError> {
    let ownership = processes.reserve_starting(id).await?;
    let instance = repository
        .load()?
        .into_iter()
        .find(|instance| instance.id == id)
        .ok_or_else(|| LaunchError::InstanceNotFound(id.to_owned()))?;
    Ok(InstanceLaunchReservation {
        ownership,
        instance,
    })
}

pub(crate) async fn launch_instance_with_events<R: Runtime>(
    app: AppHandle<R>,
    instance_id: String,
    quick_play: Option<QuickPlayOptions>,
) -> Result<u32, GameLaunchError> {
    launch_instance_with_options(
        app,
        instance_id,
        LaunchSessionOptions {
            quick_play,
            ..Default::default()
        },
    )
    .await
}

pub(crate) async fn launch_instance_with_options<R: Runtime>(
    app: AppHandle<R>,
    instance_id: String,
    options: LaunchSessionOptions,
) -> Result<u32, GameLaunchError> {
    crate::app::lifecycle::adopt_running_games(&app).await;
    let pm = crate::app::state(&app).games.clone();
    let client = crate::app::state(&app).http.clone();

    let app_prog = app.clone();
    let on_prog = move |ev: LaunchProgressEvent| {
        crate::app::events::launch_progress(&app_prog, ev);
    };

    let app_stat = app.clone();
    let on_status = move |ev: InstanceStatusEvent| {
        crate::app::events::instance_status(&app_stat, ev);
    };

    launch_minecraft_with_options(app, pm, client, instance_id, options, on_prog, on_status).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_core::metadata::MetadataRepository;
    use ingot_minecraft::instance::{prepare_instance, ModLoaderType};

    #[test]
    fn transaction_inspection_preserves_decode_cause_and_blocks_launch() {
        use std::error::Error;
        let directory = tempfile::tempdir().unwrap();
        let backup = ingot_version_change::models::backup_dir(directory.path());
        std::fs::create_dir_all(&backup).unwrap();
        std::fs::write(backup.join("journal.json"), b"malformed").unwrap();
        let error = ensure_transaction_ready(directory.path()).unwrap_err();
        assert!(matches!(error, GameLaunchError::TransactionInspection(_)));
        let source = error.source().unwrap();
        assert!(source.is::<Box<ingot_version_change::VersionChangeError>>());
        assert!(source.source().unwrap().is::<serde_json::Error>());
    }

    #[test]
    fn incomplete_transaction_retains_its_phase() {
        let directory = tempfile::tempdir().unwrap();
        let backup = ingot_version_change::models::backup_dir(directory.path());
        std::fs::create_dir_all(&backup).unwrap();
        let journal = ingot_version_change::journal::Journal {
            phase: ingot_version_change::TransactionPhase::Swapping,
            ..Default::default()
        };
        std::fs::write(
            backup.join("journal.json"),
            serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            ensure_transaction_ready(directory.path()),
            Err(GameLaunchError::IncompleteTransaction(
                ingot_version_change::TransactionPhase::Swapping
            ))
        ));
    }

    #[test]
    fn incomplete_transaction_maps_to_incomplete_recovery_api_error() {
        let directory = tempfile::tempdir().unwrap();
        let backup = ingot_version_change::models::backup_dir(directory.path());
        std::fs::create_dir_all(&backup).unwrap();
        let journal = ingot_version_change::journal::Journal {
            phase: ingot_version_change::TransactionPhase::Swapping,
            ..Default::default()
        };
        std::fs::write(
            backup.join("journal.json"),
            serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        let error = ensure_transaction_ready(directory.path()).unwrap_err();
        let api_error = crate::ipc::ApiError::from(error);
        assert_eq!(
            api_error,
            crate::ipc::ApiError::IncompleteRecovery {
                message: "Cannot launch instance: version change transaction is in incomplete state (Swapping). Please recover or revert it first.".to_string(),
            }
        );
    }

    #[test]
    fn transaction_inspection_maps_to_incomplete_recovery_api_error() {
        let directory = tempfile::tempdir().unwrap();
        let backup = ingot_version_change::models::backup_dir(directory.path());
        std::fs::create_dir_all(&backup).unwrap();
        std::fs::write(backup.join("journal.json"), b"malformed").unwrap();
        let error = ensure_transaction_ready(directory.path()).unwrap_err();
        let api_error = crate::ipc::ApiError::from(error);
        assert!(matches!(
            api_error,
            crate::ipc::ApiError::IncompleteRecovery { .. }
        ));
        assert!(api_error
            .to_string()
            .contains("Cannot inspect version change recovery state"));
    }

    #[tokio::test]
    async fn launch_reservation_reads_latest_metadata_and_releases_missing_targets() {
        let directory = tempfile::tempdir().unwrap();
        let repository = MetadataRepository::new(directory.path().join("instances.json"));
        let original = prepare_instance(
            "Fixture".into(),
            "1.20".into(),
            ModLoaderType::Vanilla,
            None,
        )
        .unwrap();
        let id = original.id.clone();
        repository
            .update(|records| {
                records.push(original);
                Ok(())
            })
            .unwrap();
        let processes = ProcessManager::new();
        let maintenance = processes.reserve_maintenance(&id).unwrap();
        assert!(matches!(
            reserve_instance_launch(&repository, &processes, &id).await,
            Err(LaunchError::AlreadyRunning)
        ));
        repository
            .update(|records| {
                records[0].game_version = "1.21".into();
                Ok(())
            })
            .unwrap();
        drop(maintenance);
        let launch = reserve_instance_launch(&repository, &processes, &id)
            .await
            .unwrap();
        assert_eq!(launch.instance.game_version, "1.21");
        drop(launch);
        repository
            .update(|records| {
                records.clear();
                Ok(())
            })
            .unwrap();
        assert!(matches!(
            reserve_instance_launch(&repository, &processes, &id).await,
            Err(LaunchError::InstanceNotFound(_))
        ));
        assert!(!processes.is_running(&id).await);
    }
}
