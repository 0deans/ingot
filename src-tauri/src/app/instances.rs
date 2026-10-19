//! Native path and managed-repository wiring for instance services.
use crate::minecraft::instance::{self, InstanceConfig, InstanceError, ModLoaderType};
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

pub(crate) fn load_instances<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Vec<InstanceConfig>, InstanceError> {
    instance::load_instances(&super::state(app).instances)
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
    ownership: crate::minecraft::launcher::process::MaintenanceGuard,
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
        let directory = ensure_instance_dir(&handle, &id)?;
        instance::get_instance_worlds(&directory)
    })
    .await?
}

#[cfg(test)]
mod tests;
