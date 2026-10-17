//! Native skin cache and save dialog. Cache failures are nonfatal because the provider is authoritative.
use base64::Engine;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tauri::Manager;

#[derive(thiserror::Error)]
pub(crate) enum DownloadError {
    #[error("Skin URL is empty")]
    EmptyUrl,
    #[error("Invalid skin data URL")]
    InvalidDataUrl,
    #[error("Invalid base64 skin data: {0}")]
    Decode(#[from] base64::DecodeError),
    #[error("Native path resolution failed: {0}")]
    Path(#[source] tauri::Error),
    #[error("Skin request failed")]
    Transport(#[source] reqwest::Error),
    #[error("Skin response failed")]
    Body(#[source] reqwest::Error),
    #[error("Skin provider returned HTTP {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("Skin file I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid destination file path")]
    InvalidDestination,
    #[error("Skin task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

impl std::fmt::Debug for DownloadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DownloadError")
            .field("message", &self.to_string())
            .finish_non_exhaustive()
    }
}

fn cache_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, DownloadError> {
    app.path()
        .app_cache_dir()
        .or_else(|_| app.path().app_data_dir())
        .map(|path| path.join("skins"))
        .map_err(DownloadError::Path)
}

async fn skin_bytes(
    client: &reqwest::Client,
    cache: &Path,
    url: &str,
) -> Result<Vec<u8>, DownloadError> {
    if url.trim().is_empty() {
        return Err(DownloadError::EmptyUrl);
    }
    if url.starts_with("data:") {
        let (prefix, encoded) = url.split_once(',').ok_or(DownloadError::InvalidDataUrl)?;
        if prefix != "data:image/png;base64" {
            return Err(DownloadError::InvalidDataUrl);
        }
        return Ok(base64::engine::general_purpose::STANDARD.decode(encoded.trim())?);
    }
    let hash = uuid::Uuid::new_v3(&uuid::Uuid::NAMESPACE_URL, url.as_bytes());
    let cached = cache.join(format!("{hash:x}.png"));
    let read_path = cached.clone();
    match tokio::task::spawn_blocking(move || fs::read(read_path)).await? {
        Ok(bytes) => return Ok(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => eprintln!("Skin cache read failed; fetching from provider: {error}"),
    }
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| DownloadError::Transport(error.without_url()))?;
    if !response.status().is_success() {
        return Err(DownloadError::HttpStatus(response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| DownloadError::Body(error.without_url()))?
        .to_vec();
    let cache_bytes = bytes.clone();
    let cache = cache.to_owned();
    if let Err(error) = tokio::task::spawn_blocking(move || {
        fs::create_dir_all(cache)?;
        publish_skin_file(&cached, &cache_bytes)
    })
    .await?
    {
        eprintln!("Skin fetched, but optional cache write failed: {error}");
    }
    Ok(bytes)
}

pub(crate) async fn get_skin_data_url<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    skin_url: String,
) -> Result<String, DownloadError> {
    let bytes = skin_bytes(&crate::app::state(&app).http, &cache_dir(&app)?, &skin_url).await?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

pub(crate) async fn save_skin_to_downloads<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
    skin_url: String,
) -> Result<String, DownloadError> {
    let bytes = skin_bytes(&crate::app::state(&app).http, &cache_dir(&app)?, &skin_url).await?;
    let directory = app
        .path()
        .download_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(DownloadError::Path)?;
    let default_name = download_filename(&username);
    let file_path = tokio::task::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        app.dialog()
            .file()
            .set_title("Save Skin As")
            .set_directory(directory)
            .set_file_name(default_name)
            .add_filter("PNG Image (*.png)", &["png"])
            .blocking_save_file()
    })
    .await?;
    let target = match file_path {
        Some(path) => path
            .into_path()
            .map_err(|_| DownloadError::InvalidDestination)?,
        None => return Ok(String::new()),
    };
    let destination = target.clone();
    tokio::task::spawn_blocking(move || publish_skin_file(&destination, &bytes)).await??;
    Ok(target.to_string_lossy().into_owned())
}

fn download_filename(username: &str) -> String {
    let clean = username
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if clean.is_empty() {
        return "minecraft-skin.png".into();
    }
    format!("{clean}-skin.png")
}

fn publish_skin_file(destination: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = destination
        .parent()
        .ok_or_else(|| std::io::Error::other("missing skin destination parent"))?;
    let mut stage = tempfile::NamedTempFile::new_in(parent)?;
    stage.write_all(bytes)?;
    stage.as_file().sync_all()?;
    stage.persist(destination).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn failed_skin_transport_does_not_expose_signed_url() {
        let cache = tempfile::tempdir().unwrap();
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let error = skin_bytes(
            &client,
            cache.path(),
            "http://127.0.0.1:0/skin?token=private-signature",
        )
        .await
        .unwrap_err();
        assert!(matches!(error, DownloadError::Transport(_)));
        assert!(!format!("{error:?} {error}").contains("private-signature"));
        let DownloadError::Transport(source) = error else {
            unreachable!()
        };
        assert!(source.url().is_none());
    }
    #[tokio::test]
    async fn invalid_or_missing_data_fails_without_cache_effects() {
        let cache = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        assert!(matches!(
            skin_bytes(&client, cache.path(), "").await,
            Err(DownloadError::EmptyUrl)
        ));
        assert!(matches!(
            skin_bytes(&client, cache.path(), "data:image/png;base64").await,
            Err(DownloadError::InvalidDataUrl)
        ));
        assert!(matches!(
            skin_bytes(&client, cache.path(), "data:text/plain;base64,YWJj").await,
            Err(DownloadError::InvalidDataUrl)
        ));
        assert_eq!(
            skin_bytes(&client, cache.path(), "data:image/png;base64,YWJj")
                .await
                .unwrap(),
            b"abc"
        );
        assert_eq!(fs::read_dir(cache.path()).unwrap().count(), 0);
    }
    #[test]
    fn save_failure_keeps_destination_and_removes_staging() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("destination.png");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("existing"), b"keep").unwrap();
        assert!(publish_skin_file(&destination, b"replacement").is_err());
        assert_eq!(fs::read(destination.join("existing")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
