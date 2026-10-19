//! Native launcher paths and import repository publication.
use crate::minecraft::importer::{
    discovery::{self, DiscoveryRoots},
    execute::execute_import_instance,
    DetectedLauncher, ImportError, ImportInstanceOptions, ImportReport, ImportableInstance,
};
use crate::minecraft::metadata::MetadataError;
use std::path::PathBuf;
use tauri::{Manager, Runtime};

fn roots<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<DiscoveryRoots, ImportError> {
    let app_data = match std::env::var_os("APPDATA") {
        Some(path) => Some(PathBuf::from(path)),
        None => {
            let directory = app
                .path()
                .app_data_dir()
                .map_err(|error| ImportError::NativePath(std::io::Error::other(error)))?;
            directory.parent().map(|parent| parent.to_path_buf())
        }
    };
    let home = match std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        Some(path) => Some(PathBuf::from(path)),
        None => Some(
            app.path()
                .home_dir()
                .map_err(|error| ImportError::NativePath(std::io::Error::other(error)))?,
        ),
    };
    Ok(DiscoveryRoots { app_data, home })
}

pub(crate) async fn get_detected_launchers<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<DetectedLauncher>, ImportError> {
    let roots = roots(app)?;
    tokio::task::spawn_blocking(move || discovery::get_detected_launchers(&roots))
        .await
        .map_err(ImportError::Task)
}

pub(crate) async fn get_launcher_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
    launcher_id: String,
    custom_path: Option<String>,
) -> Result<Vec<ImportableInstance>, ImportError> {
    let roots = roots(app)?;
    tokio::task::spawn_blocking(move || {
        discovery::get_launcher_instances(&roots, &launcher_id, custom_path.as_deref())
    })
    .await?
}

pub(crate) async fn detect_custom_instance(
    path: String,
) -> Result<ImportableInstance, ImportError> {
    tokio::task::spawn_blocking(move || discovery::detect_custom_instance(&path)).await?
}

pub(crate) async fn import_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    options: ImportInstanceOptions,
) -> Result<ImportReport, ImportError> {
    let parent =
        crate::app::instances::ensure_instances_dir(app).map_err(ImportError::InstanceDir)?;
    let app = app.clone();
    tokio::task::spawn_blocking(move || {
        execute_import_instance(&parent, options, |stage, instance| {
            let destination = parent.join(&instance.id);
            crate::app::state(&app).instances.publish(
                stage,
                &destination,
                instance,
                |records, candidate| {
                    if records.iter().any(|record| record.id == candidate.id) {
                        return Err(MetadataError::Conflict("Instance already exists".into()));
                    }
                    Ok(())
                },
            )
        })
    })
    .await?
}
