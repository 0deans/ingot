//! Shared download, integrity verification, containment, and staging support for modpack installers.
use crate::content::error::ContentError;
use ingot_artifacts::sources::CURSEFORGE_API_KEY;
use ingot_minecraft::instance::ModLoaderType;
use ingot_minecraft::launcher::progress::ProgressCallback;
use serde::Deserialize;
use sha1::Digest;
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

pub(crate) struct PreparedModpack {
    pub(crate) stage: Arc<tempfile::TempDir>,
    pub(crate) name: String,
    pub(crate) game_version: String,
    pub(crate) loader: ModLoaderType,
    pub(crate) loader_version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Integrity {
    #[serde(default)]
    pub(super) hashes: HashMap<String, String>,
    #[serde(rename = "fileSize")]
    pub(super) size: Option<u64>,
}

pub(super) fn resolve_pack_name(requested_name: &str, fallback_name: String) -> String {
    let trimmed = requested_name.trim();
    if trimmed.is_empty() {
        fallback_name
    } else {
        trimmed.to_owned()
    }
}

pub(super) fn validate_stage_path(stage: &Path, relative: &str) -> Result<PathBuf, ContentError> {
    ingot_core::safe_enclosed_path(stage, relative)
        .ok_or_else(|| ContentError::UnsafePath(relative.to_owned()))
}

pub(super) fn validate_stage_filename(
    stage: &Path,
    filename: &str,
) -> Result<PathBuf, ContentError> {
    ingot_core::safe_enclosed_filename(stage, filename)
        .ok_or_else(|| ContentError::InvalidFilename(filename.to_owned()))
}

pub(super) fn is_invalid_checksum_error(error: &std::io::Error) -> bool {
    error
        .to_string()
        .to_ascii_lowercase()
        .contains("invalid checksum")
}

pub(super) fn read_zip_json_manifest<T: serde::de::DeserializeOwned>(
    zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    name: &'static str,
) -> Result<T, ContentError> {
    let mut file = zip
        .by_name(name)
        .map_err(|_| ContentError::MissingManifest(name))?;
    let mut content = Vec::new();
    if let Err(error) = file.read_to_end(&mut content) {
        if !is_invalid_checksum_error(&error) {
            return Err(ContentError::Io(error));
        }
    }
    serde_json::from_slice(&content).map_err(ContentError::Json)
}

pub(super) fn cleanup_stage(stage: Arc<tempfile::TempDir>, primary: ContentError) -> ContentError {
    let stage = match Arc::try_unwrap(stage) {
        Ok(stage) => stage,
        Err(_) => {
            return ContentError::Cleanup {
                primary: Box::new(primary),
                cleanup: std::io::Error::other("staging is still owned by a writer"),
            }
        }
    };
    if let Err(cleanup) = stage.close() {
        return ContentError::Cleanup {
            primary: Box::new(primary),
            cleanup,
        };
    }
    primary
}

pub(super) fn report_finalizing(on_progress: Option<&ProgressCallback>, total: u64) {
    if let Some(report) = on_progress {
        report(
            "Installing Modpack",
            total,
            total.max(1),
            "Finalizing modpack installation...",
        );
    }
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn verify(bytes: &[u8], integrity: &Integrity) -> Result<(), ContentError> {
    if integrity
        .size
        .is_some_and(|size| size != bytes.len() as u64)
    {
        return Err(ContentError::Integrity("size mismatch"));
    }
    for (algorithm, expected) in &integrity.hashes {
        let actual = match algorithm.as_str() {
            "sha1" => hex_digest(sha1::Sha1::digest(bytes)),
            "sha256" => hex_digest(sha2::Sha256::digest(bytes)),
            "sha512" => hex_digest(sha2::Sha512::digest(bytes)),
            "md5" => hex_digest(md5::Md5::digest(bytes)),
            _ => return Err(ContentError::Integrity("unsupported hash algorithm")),
        };
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(ContentError::Integrity("checksum mismatch"));
        }
    }
    Ok(())
}

pub(super) async fn write_download(
    client: &reqwest::Client,
    url: &str,
    stage: &Arc<tempfile::TempDir>,
    relative: &str,
    integrity: Integrity,
) -> Result<(), ContentError> {
    let destination = validate_stage_path(stage.path(), relative)?;
    let bytes = fetch(client, url).await?;
    let stage = Arc::clone(stage);
    tokio::task::spawn_blocking(move || {
        let _stage = stage;
        verify(&bytes, &integrity)?;
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(ContentError::Io)?;
        }
        std::fs::write(destination, bytes).map_err(ContentError::Io)
    })
    .await
    .map_err(ContentError::Task)?
}

pub(super) async fn fetch_with_progress(
    client: &reqwest::Client,
    url: &str,
    phase: &str,
    label: &str,
    on_progress: Option<&ProgressCallback>,
) -> Result<Vec<u8>, ContentError> {
    let mut request = client.get(url);
    let parsed =
        reqwest::Url::parse(url).map_err(|_| ContentError::MissingField("valid download URL"))?;
    if parsed
        .host_str()
        .is_some_and(|host| host == "curseforge.com" || host.ends_with(".curseforge.com"))
    {
        request = request.header("x-api-key", CURSEFORGE_API_KEY);
    }
    let mut response = request.send().await.map_err(ContentError::Transport)?;
    if !response.status().is_success() {
        return Err(ContentError::HttpStatus(response.status()));
    }
    let total_bytes = response.content_length().unwrap_or(0);
    let mut bytes = if total_bytes > 0 && total_bytes <= (1024 * 1024 * 1024) {
        Vec::with_capacity(total_bytes as usize)
    } else {
        Vec::new()
    };
    let mut downloaded: u64 = 0;
    while let Some(chunk) = response.chunk().await.map_err(ContentError::Response)? {
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        bytes.extend_from_slice(&chunk);
        if let Some(report) = on_progress {
            if total_bytes > 0 {
                report(
                    phase,
                    downloaded,
                    total_bytes,
                    &format!(
                        "{label} ({:.1} MB / {:.1} MB)",
                        downloaded as f64 / 1_048_576.0,
                        total_bytes as f64 / 1_048_576.0
                    ),
                );
            } else {
                report(
                    phase,
                    0,
                    1,
                    &format!("{label} ({:.1} MB)", downloaded as f64 / 1_048_576.0),
                );
            }
        }
    }
    Ok(bytes)
}

pub(super) async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, ContentError> {
    fetch_with_progress(client, url, "", "", None).await
}
