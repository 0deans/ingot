use std::fs;
use std::path::PathBuf;

use tauri::{Manager, Runtime};

use ingot_artifacts::sources::SourceClient;
use ingot_content::instance_export::{
    export_instance, preview_export, ExportOptions, ExportPreview, ExportReport,
    InstanceExportError, SourceClientResolver,
};

pub(crate) async fn preview_instance_export<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
) -> Result<ExportPreview, InstanceExportError> {
    let instances = crate::app::instances::load_instances(app)
        .map_err(|e| InstanceExportError::NotFound(e.to_string()))?;
    let config = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| InstanceExportError::NotFound(instance_id.clone()))?;
    let dir = crate::app::instances::ensure_instance_dir(app, &instance_id)
        .map_err(|e| InstanceExportError::NotFound(e.to_string()))?;

    tokio::task::spawn_blocking(move || preview_export(&dir, &config)).await?
}

pub(crate) async fn export_instance_archive<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: String,
    options: ExportOptions,
    dest: Option<String>,
) -> Result<ExportReport, InstanceExportError> {
    let guard = crate::app::state(app)
        .games
        .reserve_maintenance(&instance_id)
        .map_err(|_| InstanceExportError::InstanceRunning)?;

    let instances = crate::app::instances::load_instances(app)
        .map_err(|e| InstanceExportError::NotFound(e.to_string()))?;
    let config = instances
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| InstanceExportError::NotFound(instance_id.clone()))?;
    let dir = crate::app::instances::ensure_instance_dir(app, &instance_id)
        .map_err(|e| InstanceExportError::NotFound(e.to_string()))?;

    let dest_path = match dest {
        Some(p) => PathBuf::from(p),
        None => {
            let cache_dir = app
                .path()
                .app_cache_dir()
                .map_err(|e| InstanceExportError::io(&dir, std::io::Error::other(e.to_string())))?
                .join("exports");
            fs::create_dir_all(&cache_dir).map_err(|e| InstanceExportError::io(&cache_dir, e))?;

            let sanitized_name: String = options
                .name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let ext = options.format.file_extension();
            cache_dir.join(format!("{sanitized_name}-{}.{ext}", options.version.trim()))
        }
    };

    let state = crate::app::state(app);
    let client = SourceClient::new(&state.http);
    let resolver = SourceClientResolver { client };

    let report = export_instance(&dir, &config, options, &dest_path, &resolver).await?;

    drop(guard);
    Ok(report)
}
