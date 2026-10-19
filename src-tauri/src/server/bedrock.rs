use crate::server::config::ServerConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct DownloadLinksResponse {
    result: DownloadLinksResult,
}

#[derive(Debug, Deserialize)]
struct DownloadLinksResult {
    links: Vec<DownloadLinkEntry>,
}

#[derive(Debug, Deserialize)]
struct DownloadLinkEntry {
    #[serde(rename = "downloadType")]
    download_type: String,
    #[serde(rename = "downloadUrl")]
    download_url: String,
}

/// Resolves the Bedrock Dedicated Server binary executable name for this platform
pub fn get_bedrock_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "bedrock_server.exe"
    } else {
        "bedrock_server"
    }
}

pub fn get_bedrock_binary_path(server_dir: &Path) -> PathBuf {
    server_dir.join(get_bedrock_binary_name())
}

/// Extracts a clean version number from a Bedrock download URL
/// e.g. "https://www.minecraft.net/bedrockdedicatedserver/bin-win/bedrock-server-1.26.52.3.zip" -> "1.26.52.3"
fn extract_version_from_url(url: &str) -> Option<String> {
    let filename = url.rsplit('/').next()?;
    let stripped = filename
        .strip_prefix("bedrock-server-")?
        .strip_suffix(".zip")?;
    Some(stripped.to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum BedrockServerError {
    #[error("Network error during Bedrock server operation: {0}")]
    Network(#[from] reqwest::Error),

    #[error("I/O error during Bedrock server operation: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to download Bedrock server from {url}: HTTP {status}")]
    HttpStatus {
        url: String,
        status: reqwest::StatusCode,
    },

    #[error("Archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Task join error: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error("Bedrock archive extraction failed: {0}")]
    Archive(#[from] crate::app::ArchiveError),

    #[error("Invalid Bedrock executable path: {0}")]
    InvalidBinaryPath(PathBuf),

    #[error("Bedrock archive path is not valid Unicode: {0}")]
    InvalidArchivePath(PathBuf),

    #[error("Unsafe Bedrock destination: {0}")]
    UnsafeDestination(PathBuf),

    #[error("Bedrock staging root has no parent: {0}")]
    InvalidStagingRoot(PathBuf),

    #[error("Bedrock entry {path} is outside staging root {root}")]
    RelativePath {
        path: PathBuf,
        root: PathBuf,
        #[source]
        source: std::path::StripPrefixError,
    },

    #[error("Unsupported Bedrock archive entry: {0}")]
    UnsupportedEntry(PathBuf),

    #[error("Extracted Bedrock server files, but executable {0} was not found")]
    BinaryNotFound(PathBuf),
}

/// Fetches available Bedrock Dedicated Server versions from Mojang's API
pub async fn fetch_bedrock_versions(
    client: &reqwest::Client,
) -> Result<Vec<String>, BedrockServerError> {
    let url = "https://net-secondary.web.minecraft-services.net/api/v1.0/download/links";
    let res = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    let mut versions = Vec::new();

    if let Ok(response) = res {
        if let Ok(data) = response.json::<DownloadLinksResponse>().await {
            let win_target = if cfg!(target_os = "windows") {
                "serverBedrockWindows"
            } else {
                "serverBedrockLinux"
            };
            let preview_target = if cfg!(target_os = "windows") {
                "serverBedrockPreviewWindows"
            } else {
                "serverBedrockPreviewLinux"
            };

            for entry in &data.result.links {
                if entry.download_type == win_target {
                    if let Some(v) = extract_version_from_url(&entry.download_url) {
                        versions.push(v);
                    }
                } else if entry.download_type == preview_target {
                    if let Some(v) = extract_version_from_url(&entry.download_url) {
                        versions.push(format!("{v} (Preview)"));
                    }
                }
            }
        }
    }

    if versions.is_empty() {
        // Fallback versions if offline or API format changed
        versions.push("1.26.52.3".to_string());
        versions.push("1.26.60.29 (Preview)".to_string());
    }

    Ok(versions)
}

/// Resolves the download URL for a given version or default latest
async fn resolve_download_url(client: &reqwest::Client, version: &str) -> String {
    let is_preview = version.to_lowercase().contains("preview");
    let clean_ver = version.split_whitespace().next().unwrap_or(version).trim();

    // Query API first
    let api_url = "https://net-secondary.web.minecraft-services.net/api/v1.0/download/links";
    if let Ok(response) = client
        .get(api_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        if let Ok(data) = response.json::<DownloadLinksResponse>().await {
            let target_type = if cfg!(target_os = "windows") {
                if is_preview {
                    "serverBedrockPreviewWindows"
                } else {
                    "serverBedrockWindows"
                }
            } else {
                if is_preview {
                    "serverBedrockPreviewLinux"
                } else {
                    "serverBedrockLinux"
                }
            };

            for entry in data.result.links {
                if entry.download_type == target_type {
                    return entry.download_url;
                }
            }
        }
    }

    // Direct URL fallback based on OS
    if cfg!(target_os = "windows") {
        if is_preview {
            format!("https://www.minecraft.net/bedrockdedicatedserver/bin-win-preview/bedrock-server-{clean_ver}.zip")
        } else {
            format!("https://www.minecraft.net/bedrockdedicatedserver/bin-win/bedrock-server-{clean_ver}.zip")
        }
    } else {
        if is_preview {
            format!("https://www.minecraft.net/bedrockdedicatedserver/bin-linux-preview/bedrock-server-{clean_ver}.zip")
        } else {
            format!("https://www.minecraft.net/bedrockdedicatedserver/bin-linux/bedrock-server-{clean_ver}.zip")
        }
    }
}

/// Downloads and extracts the Bedrock Dedicated Server files into `server_dir` if needed
pub async fn ensure_bedrock_server_binary(
    client: &reqwest::Client,
    server_dir: &Path,
    version: &str,
) -> Result<PathBuf, BedrockServerError> {
    let bin_path = get_bedrock_binary_path(server_dir);
    if bin_path.exists() {
        if let Ok(meta) = std::fs::metadata(&bin_path) {
            if meta.len() > 1_000_000 {
                return Ok(bin_path);
            }
        }
    }

    std::fs::create_dir_all(server_dir)?;

    let download_url = resolve_download_url(client, version).await;

    let res = client
        .get(&download_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(BedrockServerError::HttpStatus {
            url: download_url,
            status: res.status(),
        });
    }

    let bytes = res.bytes().await?;

    let parent = server_dir.to_path_buf();
    let binary_name = bin_path
        .file_name()
        .ok_or_else(|| BedrockServerError::InvalidBinaryPath(bin_path.clone()))?
        .to_os_string();
    let PreparedArchive { staging, mut files } =
        tokio::task::spawn_blocking(move || prepare_archive(&parent, bytes.as_ref(), &binary_name))
            .await??;
    for (relative, file) in files.drain(..) {
        let name = relative
            .to_str()
            .ok_or_else(|| BedrockServerError::InvalidArchivePath(relative.clone()))?;
        let output = crate::app::safe_enclosed_path(server_dir, name)
            .ok_or_else(|| BedrockServerError::UnsafeDestination(server_dir.join(&relative)))?;
        let filename = output
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if output.exists()
            && matches!(
                filename,
                "server.properties" | "allowlist.json" | "whitelist.json" | "permissions.json"
            )
        {
            continue;
        }
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        file.persist(&output)
            .map_err(|error| BedrockServerError::Io(error.error))?;
    }
    drop(staging);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o755));
    }

    if !bin_path.exists() {
        return Err(BedrockServerError::BinaryNotFound(bin_path));
    }

    Ok(bin_path)
}

struct PreparedArchive {
    staging: tempfile::TempDir,
    files: Vec<(PathBuf, tempfile::NamedTempFile)>,
}

fn prepare_archive(
    parent: &Path,
    bytes: &[u8],
    binary_name: &std::ffi::OsStr,
) -> Result<PreparedArchive, BedrockServerError> {
    let staging = tempfile::tempdir_in(parent)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    crate::app::extract_zip_entries(&mut archive, staging.path(), "", None)?;
    let binary_path = staging.path().join(binary_name);
    if !binary_path.is_file() {
        return Err(BedrockServerError::BinaryNotFound(binary_path));
    }
    let mut prepared = Vec::new();
    prepare_files(staging.path(), staging.path(), &mut prepared)?;
    // Publish the executable last: a partial installation never looks ready.
    prepared.sort_by_key(|(relative, _)| relative.as_os_str() == binary_name);
    Ok(PreparedArchive {
        staging,
        files: prepared,
    })
}

fn prepare_files(
    root: &Path,
    directory: &Path,
    prepared: &mut Vec<(std::path::PathBuf, tempfile::NamedTempFile)>,
) -> Result<(), BedrockServerError> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            prepare_files(root, &entry.path(), prepared)?;
        } else if kind.is_file() {
            let mut source = std::fs::File::open(entry.path())?;
            // Tempfiles stay beside the staging root, outside the traversed tree.
            let parent = root
                .parent()
                .ok_or_else(|| BedrockServerError::InvalidStagingRoot(root.to_path_buf()))?;
            let mut output = tempfile::NamedTempFile::new_in(parent)?;
            std::io::copy(&mut source, &mut output)?;
            output.as_file().sync_all()?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|source| BedrockServerError::RelativePath {
                    path: path.clone(),
                    root: root.to_path_buf(),
                    source,
                })?
                .to_path_buf();
            prepared.push((relative, output));
        } else {
            return Err(BedrockServerError::UnsupportedEntry(entry.path()));
        }
    }
    Ok(())
}

/// Applies Bedrock server configuration properties
pub fn configure_bedrock_properties(
    server_dir: &Path,
    config: &ServerConfig,
) -> Result<(), std::io::Error> {
    let props_path = server_dir.join("server.properties");
    let mut props_map = std::collections::HashMap::new();

    if props_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&props_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.is_empty() {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    props_map.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }

    // Set configured properties
    props_map.insert("server-name".to_string(), config.name.clone());
    props_map.insert("server-port".to_string(), config.port.to_string());
    props_map.insert(
        "server-portv6".to_string(),
        (config.port.saturating_add(1)).to_string(),
    );

    // Defaults if missing
    props_map
        .entry("gamemode".to_string())
        .or_insert_with(|| "survival".to_string());
    props_map
        .entry("difficulty".to_string())
        .or_insert_with(|| "easy".to_string());
    props_map
        .entry("max-players".to_string())
        .or_insert_with(|| "10".to_string());
    props_map
        .entry("online-mode".to_string())
        .or_insert_with(|| "true".to_string());
    props_map
        .entry("white-list".to_string())
        .or_insert_with(|| "false".to_string());
    props_map
        .entry("allow-cheats".to_string())
        .or_insert_with(|| "false".to_string());

    let mut out = String::from("# Bedrock server properties configured by Ingot\n");
    for (k, v) in props_map {
        out.push_str(&format!("{k}={v}\n"));
    }

    std::fs::write(props_path, out)
}

#[cfg(test)]
#[path = "bedrock/tests.rs"]
mod tests;
