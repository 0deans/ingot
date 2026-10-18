use crate::minecraft::{
    content::{self, ContentError},
    instance::{self, InstanceConfig},
};
use tauri::{AppHandle, Runtime};

pub(crate) fn service<R: Runtime>(app: &AppHandle<R>) -> content::service::ContentService {
    content::service::ContentService::new(super::state(app).http.clone())
}

pub(crate) async fn install_content_file<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    project_type: &str,
    download_url: &str,
    filename: &str,
) -> Result<String, ContentError> {
    let handle = app.clone();
    let id = instance_id.to_owned();
    let directory = tokio::task::spawn_blocking(move || instance::get_instance_dir(&handle, &id))
        .await
        .map_err(ContentError::Task)?
        .map_err(ContentError::Instance)?;
    content::install::install_content_file(
        &super::state(app).http,
        &directory,
        project_type,
        download_url,
        filename,
    )
    .await
}

pub(crate) async fn install_modpack_instance<R: Runtime>(
    app: &AppHandle<R>,
    name: &str,
    source: &str,
    download_url: &str,
    filename: &str,
) -> Result<InstanceConfig, ContentError> {
    let handle = app.clone();
    let root = tokio::task::spawn_blocking(move || instance::get_instances_dir(&handle))
        .await
        .map_err(ContentError::Task)?
        .map_err(ContentError::Instance)?;
    let state = super::state(app);
    let installer = content::install::ModpackInstaller {
        client: state.http.clone(),
        instances: std::sync::Arc::clone(&state.instances),
        root,
        curseforge_base: content::curseforge::DEFAULT_CURSEFORGE_BASE.into(),
    };
    installer
        .install(name, source, download_url, filename)
        .await
}
