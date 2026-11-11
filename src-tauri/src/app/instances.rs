//! Native path and managed-repository wiring for instance services.
use ingot_minecraft::instance::{self, InstanceConfig, InstanceError, ModLoaderType};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

pub(crate) fn ensure_instances_dir<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<PathBuf, InstanceError> {
    let data = app.path().app_data_dir().map_err(std::io::Error::other)?;
    instance::ensure_instances_directory(&super::state(app).instances, &data.join("instances"))
}

pub(crate) fn ensure_instance_dir<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<PathBuf, InstanceError> {
    let parent = ensure_instances_dir(app)?;
    instance::ensure_instance_directory(&super::state(app).instances, &parent, id)
}

pub(crate) struct ResolvedInstance {
    pub(crate) config: InstanceConfig,
    pub(crate) dir: PathBuf,
}

pub(crate) fn load_instances<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Vec<InstanceConfig>, InstanceError> {
    instance::load_instances(&super::state(app).instances)
}

/// Checks instance registration before touching the instance directory so missing
/// IDs never create orphan directories and metadata or filesystem failures propagate.
pub(crate) fn resolve_registered_instance<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<ResolvedInstance, InstanceError> {
    let config = load_instances(app)?
        .into_iter()
        .find(|instance| instance.id == id)
        .ok_or_else(|| InstanceError::NotFound(id.to_owned()))?;
    let dir = ensure_instance_dir(app, &config.id)?;
    Ok(ResolvedInstance { config, dir })
}

pub(crate) fn patch_instance<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    change: impl FnOnce(&mut InstanceConfig),
) -> Result<(), InstanceError> {
    instance::patch_instance(&super::state(app).instances, id, change)
}

pub(crate) async fn create_instance<R: Runtime>(
    app: &AppHandle<R>,
    name: String,
    game_version: String,
    loader: ModLoaderType,
    loader_version: Option<String>,
) -> Result<InstanceConfig, InstanceError> {
    let data = app.path().app_data_dir().map_err(std::io::Error::other)?;
    let repository = std::sync::Arc::clone(&super::state(app).instances);
    tokio::task::spawn_blocking(move || {
        instance::create_instance(
            &repository,
            &data.join("instances"),
            name,
            game_version,
            loader,
            loader_version,
        )
    })
    .await?
}

pub(crate) fn update_last_played<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    played_seconds: u64,
) -> Result<(), InstanceError> {
    instance::update_last_played(&super::state(app).instances, id, played_seconds)
}

pub(crate) fn duplicate_instance<R: Runtime>(
    app: &AppHandle<R>,
    source_id: &str,
    name: String,
    include_worlds: bool,
) -> Result<InstanceConfig, InstanceError> {
    let parent = ensure_instances_dir(app)?;
    instance::duplicate_instance(
        &super::state(app).instances,
        &parent,
        source_id,
        name,
        include_worlds,
    )
}

pub(crate) fn delete_instance<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<(), InstanceError> {
    let parent = ensure_instances_dir(app)?;
    instance::delete_instance(&super::state(app).instances, &parent, id)
}

pub(crate) fn update_instance<R: Runtime>(
    app: &AppHandle<R>,
    config: InstanceConfig,
) -> Result<(), InstanceError> {
    let parent = ensure_instances_dir(app)?;
    instance::update_instance(&super::state(app).instances, &parent, config)
}

/// The blocking worker owns maintenance even when its caller is cancelled.
pub(crate) async fn run_maintenance<T: Send + 'static>(
    ownership: ingot_minecraft::launcher::process::MaintenanceGuard,
    operation: impl FnOnce() -> Result<T, InstanceError> + Send + 'static,
) -> Result<T, InstanceError> {
    tokio::task::spawn_blocking(move || {
        let _ownership = ownership;
        operation()
    })
    .await?
}

pub(crate) async fn read_instances<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Vec<InstanceConfig>, InstanceError> {
    let repository = std::sync::Arc::clone(&super::state(app).instances);
    tokio::task::spawn_blocking(move || instance::load_instances(&repository)).await?
}

pub(crate) async fn read_worlds<R: Runtime>(
    app: &AppHandle<R>,
    id: String,
) -> Result<Vec<instance::InstanceWorldSummary>, InstanceError> {
    let handle = app.clone();
    tokio::task::spawn_blocking(move || {
        let resolved = resolve_registered_instance(&handle, &id)?;
        instance::get_instance_worlds(&resolved.dir)
    })
    .await?
}

pub(crate) async fn create_instance_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: String,
    location: instance::ShortcutLocation,
) -> Result<instance::ShortcutResult, InstanceError> {
    let handle = app.clone();
    tokio::task::spawn_blocking(move || {
        let ResolvedInstance {
            config: instance,
            dir: instance_dir,
        } = resolve_registered_instance(&handle, &instance_id)?;
        let target_exe = std::env::current_exe().map_err(instance::ShortcutError::Executable)?;

        let path = match location {
            instance::ShortcutLocation::Steam => {
                instance::shortcut::create_steam_shortcut(&instance_dir, &target_exe, &instance)?
            }
            instance::ShortcutLocation::Desktop | instance::ShortcutLocation::ApplicationsMenu => {
                let destination_dir = resolve_shortcut_directory(&handle, location)?;
                #[cfg(windows)]
                let destination_dir = if location == instance::ShortcutLocation::ApplicationsMenu {
                    if let Some(group) = &instance.group {
                        let trimmed = group.trim();
                        if trimmed.is_empty() {
                            destination_dir
                        } else {
                            let safe_group =
                                instance::shortcut::sanitize_shortcut_filename(trimmed);
                            destination_dir.join(safe_group)
                        }
                    } else {
                        destination_dir
                    }
                } else {
                    destination_dir
                };
                let icon_path = instance::shortcut::resolve_instance_icon(&instance_dir, &instance);
                instance::shortcut::create_platform_shortcut(
                    &destination_dir,
                    &target_exe,
                    &instance.id,
                    &instance.name,
                    icon_path.as_deref(),
                )?
            }
        };

        Ok(instance::ShortcutResult {
            path: path.to_string_lossy().into_owned(),
        })
    })
    .await?
}

pub(crate) fn resolve_shortcut_directory<R: Runtime>(
    app: &AppHandle<R>,
    location: instance::ShortcutLocation,
) -> Result<PathBuf, instance::ShortcutError> {
    match location {
        #[cfg(not(target_os = "android"))]
        instance::ShortcutLocation::Desktop => app
            .path()
            .desktop_dir()
            .map_err(|e| instance::ShortcutError::DirectoryNotFound(e.to_string())),
        #[cfg(target_os = "android")]
        instance::ShortcutLocation::Desktop => {
            let _ = app;
            Err(instance::ShortcutError::UnsupportedPlatform(
                std::env::consts::OS.to_string(),
            ))
        }
        instance::ShortcutLocation::ApplicationsMenu => {
            #[cfg(target_os = "macos")]
            {
                let home = app
                    .path()
                    .home_dir()
                    .map_err(|e| instance::ShortcutError::DirectoryNotFound(e.to_string()))?;
                instance::shortcut::resolve_shortcut_directory(&home, location)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let data = app
                    .path()
                    .data_dir()
                    .map_err(|e| instance::ShortcutError::DirectoryNotFound(e.to_string()))?;
                instance::shortcut::resolve_shortcut_directory(&data, location)
            }
        }
        instance::ShortcutLocation::Steam => Err(instance::ShortcutError::DirectoryNotFound(
            "Steam shortcuts are resolved through Steam userdata discovery".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests;
