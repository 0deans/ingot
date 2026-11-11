use ingot_artifacts::java::{JavaError, JavaProgressFn};
use std::path::PathBuf;
use tauri::{Manager, Runtime};

pub(crate) async fn ensure_java_runtime<R: Runtime>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    major_version: u32,
    custom_path: Option<&str>,
    progress: Option<JavaProgressFn>,
) -> Result<PathBuf, JavaError> {
    let data_dir = app.path().app_data_dir().map_err(std::io::Error::other)?;
    ingot_artifacts::java::ensure_java_runtime_in_dir(
        data_dir,
        client,
        major_version,
        custom_path,
        progress,
    )
    .await
}
