use super::bootstrap::{is_rootfs_installed, ALPINE_BRANCH, ALPINE_VERSION, BASE_MARKER};
use super::SandboxError;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;
use tar::Archive;
use tokio::io::AsyncWriteExt;

const RECEIPT: &str = ".rootfs-publication.json";
const STAGE: &str = "rootfs.next";
const PREVIOUS: &str = "rootfs.previous";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: u32,
    had_previous: bool,
    phase: Phase,
}

#[derive(Serialize, Deserialize)]
enum Phase {
    Prepared,
    Committed,
}

pub(super) async fn install<F>(
    client: &reqwest::Client,
    sandbox: &Path,
    endpoint: &str,
    progress: &F,
) -> Result<(), SandboxError>
where
    F: Fn(&str, f32) + Send + Sync,
{
    progress("Downloading Linux sandbox...", 0.05);
    let temporary = tempfile::NamedTempFile::new_in(sandbox)?;
    let mut file = tokio::fs::File::from_std(temporary.reopen()?);
    let mut response = client
        .get(endpoint)
        .timeout(Duration::from_secs(5 * 60))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(SandboxError::DownloadStatus {
            status: response.status(),
        });
    }
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.sync_all().await?;
    drop(file);
    progress("Extracting sandbox filesystem...", 0.20);
    let sandbox = sandbox.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let result = extract_and_publish(&sandbox, temporary.path());
        match (result, temporary.close()) {
            (result, Ok(())) => result,
            (Ok(()), Err(error)) => Err(error.into()),
            (Err(primary), Err(source)) => Err(SandboxError::Cleanup {
                primary: Box::new(primary),
                source,
            }),
        }
    })
    .await??;
    Ok(())
}

fn extract_and_publish(sandbox: &Path, archive_path: &Path) -> Result<(), SandboxError> {
    recover(sandbox)?;
    let stage = sandbox.join(STAGE);
    remove_directory(&stage)?;
    fs::create_dir(&stage)?;
    let prepared = prepare_stage(&stage, archive_path);
    if let Err(primary) = prepared {
        return match remove_directory(&stage) {
            Ok(()) => Err(primary),
            Err(source) => Err(SandboxError::Cleanup {
                primary: Box::new(primary),
                source,
            }),
        };
    }
    publish(sandbox).map_err(|error| SandboxError::PublicationPending(Box::new(error)))
}

fn prepare_stage(stage: &Path, archive_path: &Path) -> Result<(), SandboxError> {
    let mut archive = Archive::new(GzDecoder::new(File::open(archive_path)?));
    archive.unpack(stage)?;
    let apk =
        crate::app::safe_enclosed_path(stage, "sbin/apk").ok_or(SandboxError::InvalidRootfs)?;
    let repositories = crate::app::safe_enclosed_path(stage, "etc/apk/repositories")
        .ok_or(SandboxError::InvalidRootfs)?;
    let resolv = crate::app::safe_enclosed_path(stage, "etc/resolv.conf")
        .ok_or(SandboxError::InvalidRootfs)?;
    let hosts =
        crate::app::safe_enclosed_path(stage, "etc/hosts").ok_or(SandboxError::InvalidRootfs)?;
    let marker =
        crate::app::safe_enclosed_path(stage, BASE_MARKER).ok_or(SandboxError::InvalidRootfs)?;
    if !apk.is_file() || !stage.join("etc/apk").is_dir() {
        return Err(SandboxError::InvalidRootfs);
    }
    // Android does not provide a host resolv.conf for the guest.
    fs::write(resolv, "nameserver 1.1.1.1\nnameserver 8.8.8.8\n")?;
    fs::write(hosts, "127.0.0.1 localhost\n::1 localhost ip6-localhost\n")?;
    fs::write(repositories, format!(
        "https://dl-cdn.alpinelinux.org/alpine/{ALPINE_BRANCH}/main\nhttps://dl-cdn.alpinelinux.org/alpine/{ALPINE_BRANCH}/community\n"
    ))?;
    fs::write(marker, ALPINE_VERSION)?;
    Ok(())
}

fn publish(sandbox: &Path) -> Result<(), SandboxError> {
    validate_paths(sandbox)?;
    let rootfs = sandbox.join("rootfs");
    let mut receipt = Receipt {
        version: 1,
        had_previous: rootfs.try_exists()?,
        phase: Phase::Prepared,
    };
    save_receipt(sandbox, &receipt)?;
    if receipt.had_previous {
        fs::rename(&rootfs, sandbox.join(PREVIOUS))?;
        sync_directory(sandbox)?;
    }
    fs::rename(sandbox.join(STAGE), &rootfs)?;
    sync_directory(sandbox)?;
    receipt.phase = Phase::Committed;
    save_receipt(sandbox, &receipt)?;
    recover(sandbox)
}

/// Prepared receipts roll back; committed receipts finish cleanup before acknowledgement.
pub(super) fn recover(sandbox: &Path) -> Result<(), SandboxError> {
    validate_paths(sandbox)?;
    let bytes = match fs::read(sandbox.join(RECEIPT)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if sandbox.join(PREVIOUS).try_exists()? {
                return Err(SandboxError::RecoveryBlocked(
                    "backup exists without receipt",
                ));
            }
            remove_directory(&sandbox.join(STAGE))?;
            sync_directory(sandbox)?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let receipt: Receipt = serde_json::from_slice(&bytes).map_err(SandboxError::Receipt)?;
    if receipt.version != 1 {
        return Err(SandboxError::RecoveryBlocked("unknown receipt version"));
    }
    let rootfs = sandbox.join("rootfs");
    let previous = sandbox.join(PREVIOUS);
    match receipt.phase {
        Phase::Prepared => {
            if receipt.had_previous {
                if previous.try_exists()? {
                    remove_directory(&rootfs)?;
                    fs::rename(&previous, &rootfs)?;
                    sync_directory(sandbox)?;
                } else if !rootfs.try_exists()? {
                    return Err(SandboxError::RecoveryBlocked("original rootfs is missing"));
                }
            } else {
                if previous.try_exists()? {
                    return Err(SandboxError::RecoveryBlocked("unexpected original rootfs"));
                }
                remove_directory(&rootfs)?;
            }
            remove_directory(&sandbox.join(STAGE))?;
        }
        Phase::Committed => {
            if !is_rootfs_installed(&rootfs)? {
                return Err(SandboxError::RecoveryBlocked(
                    "committed rootfs is incomplete",
                ));
            }
            remove_directory(&previous)?;
            remove_directory(&sandbox.join(STAGE))?;
        }
    }
    sync_directory(sandbox)?;
    fs::remove_file(sandbox.join(RECEIPT))?;
    sync_directory(sandbox)?;
    Ok(())
}

fn save_receipt(sandbox: &Path, receipt: &Receipt) -> Result<(), SandboxError> {
    let mut file = tempfile::NamedTempFile::new_in(sandbox)?;
    serde_json::to_writer(&mut file, receipt).map_err(SandboxError::Receipt)?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(sandbox.join(RECEIPT))
        .map_err(|error| error.error)?;
    sync_directory(sandbox)?;
    Ok(())
}

fn validate_paths(sandbox: &Path) -> Result<(), SandboxError> {
    for path in [
        sandbox.to_path_buf(),
        sandbox.join("rootfs"),
        sandbox.join(STAGE),
        sandbox.join(PREVIOUS),
        sandbox.join(RECEIPT),
    ] {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                let linked = metadata.file_type().is_symlink();
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    linked || metadata.file_attributes() & 0x400 != 0
                };
                if linked {
                    return Err(SandboxError::RecoveryBlocked("linked publication path"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn remove_directory(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests;
