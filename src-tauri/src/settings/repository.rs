use std::{fs, io::Write, path::PathBuf, sync::Mutex};

use tempfile::NamedTempFile;

use super::{LauncherSettings, SettingsError};

/// One application-owned store serializes read/modify/write operations.
pub(crate) struct SettingsRepository {
    path: PathBuf,
    lock: Mutex<()>,
}

impl SettingsRepository {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    pub(crate) fn load(&self) -> Result<LauncherSettings, SettingsError> {
        let _guard = self.lock.lock().map_err(|_| SettingsError::LockPoisoned)?;
        self.load_unlocked()
    }

    pub(crate) fn update<T>(
        &self,
        change: impl FnOnce(&mut LauncherSettings) -> Result<T, SettingsError>,
    ) -> Result<T, SettingsError> {
        self.update_with(change, |file, path| {
            file.persist(path).map(|_| ()).map_err(|error| error.error)
        })
    }

    // The commit boundary permits deterministic failure injection without native
    // permissions or dependence on the user's filesystem configuration.
    fn update_with<T>(
        &self,
        change: impl FnOnce(&mut LauncherSettings) -> Result<T, SettingsError>,
        commit: impl FnOnce(NamedTempFile, &std::path::Path) -> std::io::Result<()>,
    ) -> Result<T, SettingsError> {
        let _guard = self.lock.lock().map_err(|_| SettingsError::LockPoisoned)?;
        let mut settings = self.load_unlocked()?;
        let result = change(&mut settings)?;
        settings.validate()?;
        let bytes = serde_json::to_vec_pretty(&settings)?;

        let parent = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|source| self.io_error("create directory for", source))?;
        let mut staged =
            NamedTempFile::new_in(parent).map_err(|source| self.io_error("stage", source))?;
        staged
            .write_all(&bytes)
            .map_err(|source| self.io_error("write", source))?;
        staged
            .as_file()
            .sync_all()
            .map_err(|source| self.io_error("flush", source))?;
        commit(staged, &self.path).map_err(|source| self.io_error("replace", source))?;
        Ok(result)
    }

    fn load_unlocked(&self) -> Result<LauncherSettings, SettingsError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LauncherSettings::default());
            }
            Err(source) => return Err(self.io_error("read", source)),
        };
        let settings: LauncherSettings =
            serde_json::from_slice(&bytes).map_err(|source| SettingsError::Decode {
                path: self.path.clone(),
                source,
            })?;
        settings.validate()?;
        Ok(settings)
    }

    fn io_error(&self, operation: &'static str, source: std::io::Error) -> SettingsError {
        SettingsError::Io {
            operation,
            path: self.path.clone(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_commit_preserves_previous_settings_and_cleans_staging() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let repository = SettingsRepository::new(path.clone());
        repository.update(|_| Ok(())).unwrap();
        let previous = fs::read(&path).unwrap();

        let error = repository
            .update_with(
                |settings| {
                    settings.window.width = 1280;
                    Ok(())
                },
                |_, _| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            SettingsError::Io {
                operation: "replace",
                ..
            }
        ));
        assert_eq!(fs::read(&path).unwrap(), previous);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        repository
            .update(|settings| {
                settings.window.width = 1280;
                Ok(())
            })
            .unwrap();
        assert_eq!(repository.load().unwrap().window.width, 1280);
    }
}
