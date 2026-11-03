use crate::minecraft::instance::InstanceConfig;
use crate::minecraft::worlds::models::{
    validate_world_folder_name, WorldBackupInfo, WorldDimension, WorldResetResult, WorldSummary,
};
use crate::minecraft::worlds::{dimensions, discovery, transfer, WorldError};
use std::path::PathBuf;
use tauri::{Manager, Runtime};

fn instance_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
) -> Result<PathBuf, WorldError> {
    crate::app::instances::ensure_instance_dir(app, instance_id).map_err(WorldError::InstanceDir)
}

fn load_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<InstanceConfig>, WorldError> {
    crate::app::instances::load_instances(app).map_err(WorldError::LoadInstances)
}

pub(crate) async fn get_all_worlds<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<WorldSummary>, WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let instances = load_instances(&app)?;
        let mut all_worlds = Vec::new();
        for instance in instances {
            if let Ok(dir) = instance_dir(&app, &instance.id) {
                if let Ok(worlds) =
                    discovery::get_instance_worlds(&dir, &instance.id, &instance.name)
                {
                    all_worlds.extend(worlds);
                }
            }
        }
        all_worlds.sort_by_key(|w| std::cmp::Reverse(w.last_played));
        Ok(all_worlds)
    })
    .await?
}

pub(crate) async fn get_instance_worlds_full<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
) -> Result<Vec<WorldSummary>, WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let instances = load_instances(&app)?;
        let instance = instances
            .into_iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| WorldError::InstanceNotFound(instance_id.clone()))?;
        let dir = instance_dir(&app, &instance_id)?;
        discovery::get_instance_worlds(&dir, &instance_id, &instance.name)
    })
    .await?
}

pub(crate) async fn reset_world_dimension<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: String,
    dimension: WorldDimension,
) -> Result<WorldResetResult, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    validate_world_folder_name(&folder_name)?;
    let app = app.clone();

    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let dir = instance_dir(&app, &instance_id)?;
        let world_dir = dir.join("saves").join(&folder_name);
        dimensions::reset_dimension(&world_dir, dimension)
    })
    .await?
}

pub(crate) async fn export_world<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: String,
    dest: Option<String>,
) -> Result<String, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    validate_world_folder_name(&folder_name)?;
    let app = app.clone();

    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let dir = instance_dir(&app, &instance_id)?;
        let world_dir = dir.join("saves").join(&folder_name);

        let dest_path = match dest {
            Some(p) => PathBuf::from(p),
            None => {
                let cache_dir = app
                    .path()
                    .app_cache_dir()
                    .map_err(|e| WorldError::Archive(e.to_string()))?
                    .join("exports");
                std::fs::create_dir_all(&cache_dir)?;
                cache_dir.join(format!("{folder_name}.zip"))
            }
        };

        transfer::export_world(&world_dir, &dest_path)?;
        Ok(dest_path.to_string_lossy().to_string())
    })
    .await?
}

pub(crate) async fn backup_world<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: String,
) -> Result<WorldBackupInfo, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    validate_world_folder_name(&folder_name)?;
    let app = app.clone();

    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let dir = instance_dir(&app, &instance_id)?;
        transfer::backup_world(&dir, &folder_name)
    })
    .await?
}

pub(crate) async fn list_world_backups<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    folder_name: Option<String>,
) -> Result<Vec<WorldBackupInfo>, WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let dir = instance_dir(&app, &instance_id)?;
        transfer::list_world_backups(&dir, folder_name.as_deref())
    })
    .await?
}

pub(crate) async fn delete_world_backup<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    file_name: String,
) -> Result<(), WorldError> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let dir = instance_dir(&app, &instance_id)?;
        transfer::delete_world_backup(&dir, &file_name)
    })
    .await?
}

pub(crate) async fn import_world<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    zip_path: String,
) -> Result<WorldSummary, WorldError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| WorldError::InstanceRunning)?;

    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let instances = load_instances(&app)?;
        let instance = instances
            .into_iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| WorldError::InstanceNotFound(instance_id.clone()))?;

        let dir = instance_dir(&app, &instance_id)?;
        let saves_dir = dir.join("saves");
        let source_path = PathBuf::from(zip_path);

        let final_folder_name = transfer::import_world(&saves_dir, &source_path)?;
        let worlds = discovery::get_instance_worlds(&dir, &instance_id, &instance.name)?;
        worlds
            .into_iter()
            .find(|w| w.folder_name == final_folder_name)
            .ok_or_else(|| WorldError::NotFound(final_folder_name))
    })
    .await?
}

pub(crate) fn resolve_world_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    folder_name: &str,
) -> Result<PathBuf, WorldError> {
    validate_world_folder_name(folder_name)?;
    let dir = instance_dir(app, instance_id)?;
    let world_dir = dir.join("saves").join(folder_name);
    if !world_dir.is_dir() {
        return Err(WorldError::NotFound(folder_name.to_string()));
    }
    Ok(world_dir)
}
