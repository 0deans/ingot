use std::path::Path;

use super::PlayitError;

pub const SECRET_FILE: &str = "secret.key";

/// Reads the saved playit secret key from `playit_dir/secret.key`.
///
/// Distinguishes:
/// - `Ok(None)`: Missing (the secret file legitimately does not exist).
/// - `Ok(Some(key))`: Present and non-empty.
/// - `Err(PlayitError::InvalidCredential)`: File exists but contains only whitespace or is empty.
/// - `Err(PlayitError::CredentialRead(err))`: Failed to read the file due to an I/O error.
pub fn read_secret(playit_dir: &Path) -> Result<Option<String>, PlayitError> {
    let secret_path = playit_dir.join(SECRET_FILE);
    match std::fs::read_to_string(&secret_path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return Err(PlayitError::InvalidCredential);
            }
            Ok(Some(trimmed.to_string()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(PlayitError::CredentialRead(err)),
    }
}

/// Persists a newly claimed or provided secret key before dependent work begins.
///
/// Stages the file beside `secret.key` and renames it to prevent partial writes.
pub fn write_secret(playit_dir: &Path, secret: &str) -> Result<(), PlayitError> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Err(PlayitError::InvalidCredential);
    }
    std::fs::create_dir_all(playit_dir).map_err(PlayitError::CredentialWrite)?;
    let secret_path = playit_dir.join(SECRET_FILE);
    ingot_core::write_file_atomic_with_parent_sync(&secret_path, trimmed.as_bytes(), true)
        .map_err(PlayitError::CredentialWrite)?;
    Ok(())
}

/// Deletes the saved secret key when invalidated by the daemon.
///
/// Missing file is acceptable (`Ok(())`), but any other I/O failure is returned honestly.
pub fn delete_secret(playit_dir: &Path) -> Result<(), PlayitError> {
    let secret_path = playit_dir.join(SECRET_FILE);
    match std::fs::remove_file(&secret_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(PlayitError::CredentialDelete(err)),
    }
}
