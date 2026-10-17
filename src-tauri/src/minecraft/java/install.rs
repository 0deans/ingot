use super::discovery::{find_binaries, find_binary, validate_binary, JavaProbe};
use super::extract::extract_runtime;
use super::{JavaError, JavaProgressFn};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy)]
pub(crate) enum ArchiveFormat {
    Zip,
    TarGzip,
}

pub(crate) struct RuntimeRequest {
    pub data_dir: PathBuf,
    pub major_version: u32,
    pub custom_path: Option<PathBuf>,
    pub system_roots: Vec<PathBuf>,
    pub download_url: String,
    pub archive_format: ArchiveFormat,
}

pub(crate) async fn ensure_java_runtime(
    request: RuntimeRequest,
    client: &reqwest::Client,
    probe: Arc<dyn JavaProbe>,
    progress: Option<JavaProgressFn>,
) -> Result<PathBuf, JavaError> {
    let request = Arc::new(request);
    let scan_request = request.clone();
    let scan_probe = probe.clone();
    let existing =
        tokio::task::spawn_blocking(move || find_existing(&scan_request, scan_probe.as_ref()))
            .await??;
    if let Some(binary) = existing {
        return Ok(binary);
    }

    let parent = request.data_dir.join("runtimes");
    tokio::fs::create_dir_all(&parent).await?;
    let stage = tempfile::tempdir_in(&parent)?;
    let archive_path = stage.path().join("download");
    download_archive(client, &request.download_url, &archive_path, &progress).await?;
    if let Some(callback) = &progress {
        callback(100, 100, "Extracting Java runtime...");
    }
    let format = request.archive_format;
    let major = request.major_version;
    let (stage, relative) = tokio::task::spawn_blocking(move || {
        let extraction = stage.path().join("runtime");
        std::fs::create_dir(&extraction)?;
        extract_runtime(&archive_path, &extraction, format)?;
        let binary = find_binary(&extraction)?.ok_or(JavaError::MissingBinary)?;
        let binary = validate_binary(binary, major, probe.as_ref())?;
        let relative = binary
            .strip_prefix(&extraction)
            .map_err(|_| JavaError::InvalidLocation)?
            .to_path_buf();
        Ok::<_, JavaError>((stage, relative))
    })
    .await??;
    let destination = runtime_dir(&request);
    // Never remove an incomplete non-empty installation or replace a runtime
    // installed by another caller. Publication contains no cancellation point.
    match std::fs::remove_dir(&destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    std::fs::rename(stage.path().join("runtime"), &destination)?;
    Ok(destination.join(relative))
}

fn runtime_dir(request: &RuntimeRequest) -> PathBuf {
    request
        .data_dir
        .join("runtimes")
        .join(format!("java-{}", request.major_version))
}

fn find_existing(
    request: &RuntimeRequest,
    probe: &dyn JavaProbe,
) -> Result<Option<PathBuf>, JavaError> {
    if let Some(custom) = &request.custom_path {
        let binary = if custom.is_file() {
            custom.clone()
        } else {
            find_binary(custom)?.ok_or(JavaError::MissingBinary)?
        };
        return validate_binary(binary, request.major_version, probe).map(Some);
    }
    if let Some(binary) = find_binary(&runtime_dir(request))? {
        return validate_binary(binary, request.major_version, probe).map(Some);
    }
    for root in &request.system_roots {
        for binary in find_binaries(root)? {
            match validate_binary(binary, request.major_version, probe) {
                Ok(binary) => return Ok(Some(binary)),
                Err(JavaError::VersionMismatch { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
    }
    Ok(None)
}

async fn download_archive(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    progress: &Option<JavaProgressFn>,
) -> Result<(), JavaError> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    let total = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(destination).await?;
    let mut current = 0;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        current += chunk.len() as u64;
        if let Some(callback) = progress {
            callback(current, total, "Downloading Java runtime...");
        }
    }
    file.sync_all().await?;
    Ok(())
}
