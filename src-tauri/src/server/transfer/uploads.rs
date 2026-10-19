use super::{ServerConfig, TransferError, TransferService};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

const MAX_UPLOADS: usize = 1024;

#[derive(Default)]
pub(crate) struct UploadManager {
    entries: Mutex<HashMap<String, Arc<AsyncMutex<UploadState>>>>,
}

#[derive(Default)]
pub(crate) enum UploadState {
    #[default]
    Receiving,
    Imported(ServerConfig),
    Finished(ServerConfig),
}

impl UploadManager {
    pub(crate) async fn claim(
        &self,
        id: &str,
    ) -> Result<OwnedMutexGuard<UploadState>, TransferError> {
        if id.is_empty()
            || id.len() > 64
            || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(TransferError::InvalidUploadId);
        }
        let entry = {
            let mut entries = self
                .entries
                .lock()
                .map_err(|_| std::io::Error::other("Upload ownership lock poisoned"))?;
            if !entries.contains_key(id) && entries.len() >= MAX_UPLOADS {
                entries.retain(|_, entry| {
                    if Arc::strong_count(entry) != 1 {
                        return true;
                    }
                    match entry.try_lock() {
                        Ok(state) => matches!(*state, UploadState::Imported(_)),
                        Err(_) => true,
                    }
                });
            }
            if !entries.contains_key(id) && entries.len() >= MAX_UPLOADS {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "Upload session limit reached",
                )
                .into());
            }
            entries.entry(id.to_string()).or_default().clone()
        };
        Ok(entry.lock_owned().await)
    }
}

pub(crate) fn write_chunk(
    ownership: OwnedMutexGuard<UploadState>,
    path: &Path,
    bytes: &[u8],
    first: bool,
) -> Result<(), TransferError> {
    validate_upload_paths(path)?;
    if matches!(
        *ownership,
        UploadState::Imported(_) | UploadState::Finished(_)
    ) {
        return Err(TransferError::UploadFinalized);
    }
    match fs::metadata(path.with_extension("import.json")) {
        Ok(_) => return Err(TransferError::UploadFinalized),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let parent = path.parent().ok_or(TransferError::MissingParent)?;
    fs::create_dir_all(parent)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(!first)
        .truncate(first)
        .open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

pub(crate) fn finalize(
    mut ownership: OwnedMutexGuard<UploadState>,
    service: &TransferService,
    archive: &Path,
) -> Result<ServerConfig, TransferError> {
    validate_upload_paths(archive)?;
    let created = match &*ownership {
        UploadState::Receiving => recover_or_import(service, archive)?,
        UploadState::Imported(server) | UploadState::Finished(server) => server.clone(),
    };
    // The worker owns both publication and this checkpoint even when its caller
    // is cancelled. Retrying a failed cleanup must not publish a second server.
    *ownership = UploadState::Imported(created.clone());
    match fs::remove_file(archive) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(TransferError::UploadCleanup {
                server: Box::new(created),
                source,
            })
        }
    }
    let receipt = archive.with_extension("import.json");
    match fs::remove_file(&receipt) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(TransferError::UploadCleanup {
                server: Box::new(created),
                source,
            })
        }
    }
    *ownership = UploadState::Finished(created.clone());
    Ok(created)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportReceipt {
    version: u32,
    server_id: String,
}

fn recover_or_import(
    service: &TransferService,
    archive: &Path,
) -> Result<ServerConfig, TransferError> {
    let path = archive.with_extension("import.json");
    let receipt = match fs::read(&path) {
        Ok(bytes) => Some(serde_json::from_slice::<ImportReceipt>(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if let Some(receipt) = receipt {
        let identity_valid = uuid::Uuid::parse_str(&receipt.server_id)
            .is_ok_and(|id| id.to_string() == receipt.server_id);
        if receipt.version != 1 || !identity_valid {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported import receipt version",
            )
            .into());
        }
        // Loading first also resolves an interrupted directory publication.
        let records = service
            .repository
            .load()
            .map_err(crate::server::config::ConfigError::from)?;
        if let Some(server) = records
            .into_iter()
            .find(|server| server.id == receipt.server_id)
        {
            let directory = crate::app::safe_enclosed_filename(&service.servers_dir, &server.id)
                .ok_or(TransferError::UnsupportedEntry)?;
            if !directory.is_dir() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Imported server directory is missing",
                )
                .into());
            }
            return Ok(server);
        }
    }
    service.import(archive, |server| {
        let parent = path.parent().ok_or(TransferError::MissingParent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        let receipt = ImportReceipt {
            version: 1,
            server_id: server.id.clone(),
        };
        temporary.write_all(&serde_json::to_vec(&receipt)?)?;
        temporary.as_file().sync_all()?;
        temporary.persist(&path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })
}

fn validate_upload_paths(archive: &Path) -> Result<(), TransferError> {
    let imports = archive.parent().ok_or(TransferError::MissingParent)?;
    let cache = imports.parent().ok_or(TransferError::MissingParent)?;
    let directory = imports
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(TransferError::InvalidPathEncoding)?;
    crate::app::safe_enclosed_filename(cache, directory).ok_or(TransferError::UnsupportedEntry)?;
    for path in [archive.to_path_buf(), archive.with_extension("import.json")] {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(TransferError::InvalidPathEncoding)?;
        crate::app::safe_enclosed_filename(imports, filename)
            .ok_or(TransferError::UnsupportedEntry)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
