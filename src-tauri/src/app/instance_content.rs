use tauri::{AppHandle, Runtime};

use crate::minecraft::instance_content::{
    listing, mutation, ContentCategory, ContentListResult, InstanceContentError,
};

pub(crate) async fn list_content<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
) -> Result<ContentListResult, InstanceContentError> {
    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    tokio::task::spawn_blocking(move || listing::list_content(&instance_dir, category))
        .await
        .map_err(|e| InstanceContentError::InvalidOperation(e.to_string()))?
}

pub(crate) async fn set_content_enabled<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    file_name: String,
    enabled: bool,
) -> Result<String, InstanceContentError> {
    super::lifecycle::adopt_running_games(app).await;
    if super::state(app).games.is_running(instance_id).await {
        return Err(InstanceContentError::InstanceRunning(
            instance_id.to_string(),
        ));
    }

    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    tokio::task::spawn_blocking(move || {
        mutation::set_enabled(&instance_dir, category, &file_name, enabled)
    })
    .await
    .map_err(|e| InstanceContentError::InvalidOperation(e.to_string()))?
}

pub(crate) async fn set_content_enabled_bulk<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    file_names: Vec<String>,
    enabled: bool,
) -> Result<Vec<String>, InstanceContentError> {
    super::lifecycle::adopt_running_games(app).await;
    if super::state(app).games.is_running(instance_id).await {
        return Err(InstanceContentError::InstanceRunning(
            instance_id.to_string(),
        ));
    }

    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    tokio::task::spawn_blocking(move || {
        mutation::set_enabled_bulk(&instance_dir, category, &file_names, enabled)
    })
    .await
    .map_err(|e| InstanceContentError::InvalidOperation(e.to_string()))?
}

pub(crate) async fn delete_content<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    file_names: Vec<String>,
) -> Result<Vec<String>, InstanceContentError> {
    super::lifecycle::adopt_running_games(app).await;
    if super::state(app).games.is_running(instance_id).await {
        return Err(InstanceContentError::InstanceRunning(
            instance_id.to_string(),
        ));
    }

    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    tokio::task::spawn_blocking(move || {
        mutation::delete_content(&instance_dir, category, &file_names)
    })
    .await
    .map_err(|e| InstanceContentError::InvalidOperation(e.to_string()))?
}

pub(crate) async fn open_content_folder<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    world: Option<String>,
) -> Result<(), InstanceContentError> {
    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    let path = match category {
        ContentCategory::DataPack => {
            if let Some(w) = world {
                instance_dir.join("saves").join(w).join("datapacks")
            } else {
                instance_dir.join("saves")
            }
        }
        _ => instance_dir.join(category.default_subfolder()),
    };

    let _ = std::fs::create_dir_all(&path);
    super::platform::open_directory(&path)
        .map_err(|e| InstanceContentError::InvalidOperation(e.to_string()))?;

    Ok(())
}

pub(crate) async fn identify_content<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
) -> Result<crate::minecraft::instance_content::IdentifyResult, InstanceContentError> {
    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    let config = super::instances::load_instances(app)
        .map_err(|e| InstanceContentError::Metadata(e.to_string()))?
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    let game_version = config.game_version;
    let loader_id = crate::version_change::targets::loader_id(&config.loader);
    let http = super::state(app).http.clone();

    crate::minecraft::instance_content::identify_content(
        &instance_dir,
        category,
        &game_version,
        &loader_id,
        &http,
    )
    .await
}

pub(crate) async fn check_content_updates<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
) -> Result<crate::minecraft::instance_content::UpdateCheckResult, InstanceContentError> {
    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    let config = super::instances::load_instances(app)
        .map_err(|e| InstanceContentError::Metadata(e.to_string()))?
        .into_iter()
        .find(|i| i.id == instance_id)
        .ok_or_else(|| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    let game_version = config.game_version;
    let loader_id = crate::version_change::targets::loader_id(&config.loader);
    let http = super::state(app).http.clone();

    crate::minecraft::instance_content::check_updates(
        &instance_dir,
        category,
        &game_version,
        &loader_id,
        &http,
    )
    .await
}

pub(crate) async fn apply_content_updates<R: Runtime>(
    app: &AppHandle<R>,
    instance_id: &str,
    category: ContentCategory,
    selections: Vec<crate::minecraft::instance_content::ContentUpdateCandidate>,
) -> Result<crate::minecraft::instance_content::ContentApplyReport, InstanceContentError> {
    super::lifecycle::adopt_running_games(app).await;
    if super::state(app).games.is_running(instance_id).await {
        return Err(InstanceContentError::InstanceRunning(
            instance_id.to_string(),
        ));
    }

    let instance_dir = super::instances::ensure_instance_dir(app, instance_id)
        .map_err(|_| InstanceContentError::InstanceNotFound(instance_id.to_string()))?;

    let http = super::state(app).http.clone();

    crate::minecraft::instance_content::apply_updates(&instance_dir, category, selections, &http)
        .await
}
