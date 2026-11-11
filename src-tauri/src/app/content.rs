use ingot_content::content::{self, ContentError};
use ingot_minecraft::instance::InstanceConfig;
use ingot_minecraft::launcher::progress::make_progress_reporter;
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
    let (instance, directory) = tokio::task::spawn_blocking(move || {
        let parent = crate::app::instances::ensure_instances_dir(&handle)?;
        let dir = ingot_minecraft::instance::ensure_instance_directory(
            &super::state(&handle).instances,
            &parent,
            &id,
        )?;
        let instances = crate::app::instances::load_instances(&handle)?;
        let inst = instances
            .into_iter()
            .find(|i| i.id == id)
            .ok_or_else(|| ingot_minecraft::instance::InstanceError::NotFound(id))?;
        Ok::<_, ingot_minecraft::instance::InstanceError>((inst, dir))
    })
    .await
    .map_err(ContentError::Task)?
    .map_err(ContentError::Instance)?;

    if project_type.eq_ignore_ascii_case("mod") && !instance.loader.supports_mods() {
        return Err(ContentError::UnsupportedLoader {
            instance: instance.name,
            loader: instance.loader,
            category: "mods",
        });
    }

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
    let root =
        tokio::task::spawn_blocking(move || crate::app::instances::ensure_instances_dir(&handle))
            .await
            .map_err(ContentError::Task)?
            .map_err(ContentError::Instance)?;
    let state = super::state(app);
    let tracking_id = format!("installing:{}", name.trim());
    let app_prog = app.clone();
    let on_progress = make_progress_reporter(tracking_id, move |ev| {
        crate::app::events::launch_progress(&app_prog, ev);
    });
    let installer = content::install::ModpackInstaller {
        client: state.http.clone(),
        instances: std::sync::Arc::clone(&state.instances),
        root,
        curseforge_base: content::curseforge::DEFAULT_CURSEFORGE_BASE.into(),
        on_progress: Some(on_progress),
    };
    installer
        .install(name, source, download_url, filename)
        .await
}
