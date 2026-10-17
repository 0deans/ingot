use super::exit::{finish_game, handle_launcher_behavior_on_launch};
use super::preparation::NativePreparation;
use crate::minecraft::instance::{get_instance_dir, InstanceConfig};
use crate::minecraft::launcher::models::{
    InstanceStatusEvent, LaunchError, LaunchProgressEvent, QuickPlayOptions, StatusFn,
};
use crate::minecraft::launcher::prepare::prepare_launch_environment;
use crate::minecraft::launcher::process::ProcessManager;
use crate::minecraft::launcher::progress::make_progress_reporter;
use crate::minecraft::launcher::service::{
    prepare_game_command, spawn_and_commit_process, CommandContext,
};
use crate::system;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};

pub async fn launch_minecraft<R: Runtime, FProg, FStatus>(
    app: AppHandle<R>,
    process_manager: ProcessManager,
    client: reqwest::Client,
    instance_id: String,
    quick_play: Option<QuickPlayOptions>,
    on_progress: FProg,
    on_status: FStatus,
) -> Result<u32, LaunchError>
where
    FProg: Fn(LaunchProgressEvent) + Send + Sync + 'static,
    FStatus: Fn(InstanceStatusEvent) + Send + Sync + 'static,
{
    let reservation = reserve_instance_launch(
        &crate::app::state(&app).instances,
        &process_manager,
        &instance_id,
    )
    .await?;
    let instance = reservation.instance;
    let starting_guard = reservation.ownership;

    let report_prog = make_progress_reporter(instance_id.clone(), on_progress);

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| LaunchError::AppDataDir(format!("Failed to get app data directory: {e}")))?;

    let cache_dir = data_dir.join("cache");
    let assets_dir = data_dir.join("assets");
    let libraries_dir = data_dir.join("libraries");
    let instance_dir = get_instance_dir(&app, &instance_id).map_err(LaunchError::InstanceDir)?;
    match crate::version_change::detect_incomplete_transaction(&instance_dir) {
        Ok(Some(incomplete)) => {
            return Err(LaunchError::IncompleteTransaction(format!(
                "{:?}",
                incomplete.phase
            )));
        }
        Err(err) => {
            return Err(LaunchError::IncompleteTransaction(format!(
                "{err}. Please recover or inspect the instance backup first."
            )));
        }
        Ok(None) => {}
    }

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

    let global_settings =
        system::load_settings(&app).map_err(|error| LaunchError::Settings(error.to_string()))?;
    let context = CommandContext {
        instance: &instance,
        settings: &global_settings,
        instance_dir: &instance_dir,
        cache_dir: &cache_dir,
        assets_dir: &assets_dir,
        libraries_dir: &libraries_dir,
        quick_play: quick_play.as_ref(),
    };
    let command = prepare_game_command(&client, &prep, context, &report_prog).await?;

    // Run pre-launch synchronization
    if let Err(e) = crate::app::sync::sync_before_launch(&app, &instance, &instance_dir) {
        eprintln!("[Launcher] Warning: Pre-launch sync failed: {e}");
    }

    report_prog("Starting Game", 10, 10, "Launching Minecraft process...");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let (mut child, pid) = spawn_and_commit_process(command, starting_guard, now).await?;
    crate::running::add(&app, crate::running::Kind::Game, &instance_id, pid, now);

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
    );

    // Background task to monitor child exit
    let pm_clone = process_manager.clone();
    let app_clone = app.clone();
    let on_status_clone: StatusFn = Arc::new(on_status);
    let inst_clone = instance.clone();

    tokio::spawn(async move {
        let clean = match child.wait().await {
            Ok(status) => {
                println!(
                    "[Launcher] Minecraft process exited with status: {}",
                    status
                );
                status.success()
            }
            Err(e) => {
                eprintln!("[Launcher] Error waiting for Minecraft process: {e}");
                false
            }
        };
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
            &on_status_clone,
        )
        .await;
    });

    Ok(pid)
}

struct InstanceLaunchReservation {
    ownership: crate::minecraft::launcher::process::StartingGuard,
    instance: InstanceConfig,
}

async fn reserve_instance_launch(
    repository: &crate::minecraft::metadata::MetadataRepository<InstanceConfig>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::{
        instance::{prepare_instance, ModLoaderType},
        metadata::MetadataRepository,
    };

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
