use super::*;
use crate::minecraft::metadata::MetadataRepository;
use crate::server::config::creation::{prepare_server_config, ServerCreation};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Clone)]
pub(crate) struct TransferService {
    pub(crate) repository: Arc<MetadataRepository<ServerConfig>>,
    pub(crate) servers_dir: PathBuf,
}
impl TransferService {
    fn prepare(&self, request: ServerCreation) -> Result<ServerConfig, TransferError> {
        let records = self.repository.load().map_err(config::ConfigError::from)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_secs();
        Ok(prepare_server_config(
            &records,
            request,
            uuid::Uuid::new_v4().to_string(),
            now,
        )?)
    }
    fn publish(&self, config: ServerConfig, staging: &Path) -> Result<ServerConfig, TransferError> {
        let destination = crate::app::safe_enclosed_filename(&self.servers_dir, &config.id)
            .ok_or(config::ConfigError::InvalidId)?;
        self.repository
            .publish(
                staging,
                &destination,
                config.clone(),
                |records, candidate| {
                    if records.iter().any(|record| record.id == candidate.id) {
                        return Err(crate::minecraft::metadata::MetadataError::Conflict(
                            "Server already exists".into(),
                        ));
                    }
                    config::ports::check_port(records, &candidate.id, candidate.port).map_err(
                        |error| {
                            crate::minecraft::metadata::MetadataError::Conflict(error.to_string())
                        },
                    )
                },
            )
            .map_err(config::ConfigError::from)?;
        Ok(config)
    }
    /// Creates a new server from an exported archive
    pub(crate) fn import(
        &self,
        archive: &Path,
        checkpoint: impl FnOnce(&ServerConfig) -> Result<(), TransferError>,
    ) -> Result<ServerConfig, TransferError> {
        let file = File::open(archive)?;
        let mut zip = zip::ZipArchive::new(file).map_err(TransferError::InvalidZip)?;
        let manifest: Manifest = {
            let mut entry = zip.by_name(MANIFEST).map_err(|error| match error {
                zip::result::ZipError::FileNotFound => TransferError::NotAnIngotExport,
                other => TransferError::Zip(other),
            })?;
            let mut raw = String::new();
            entry.read_to_string(&mut raw)?;
            serde_json::from_str(&raw).map_err(TransferError::InvalidManifest)?
        };

        if manifest.format != 1 {
            return Err(TransferError::UnsupportedManifestFormat(manifest.format));
        }

        // Keep names distinguishable when importing a server that already exists here
        let taken = self
            .repository
            .load()
            .map_err(config::ConfigError::from)?
            .iter()
            .any(|s| s.name == manifest.name);
        let name = if taken {
            format!("{} (imported)", manifest.name)
        } else {
            manifest.name.clone()
        };
        let created = self.prepare(ServerCreation {
            name,
            core: manifest.core.clone(),
            game_version: manifest.game_version.clone(),
            build_number: manifest.build_number.clone(),
            port: None,
            memory_min_mb: Some(manifest.memory_min_mb),
            memory_max_mb: Some(manifest.memory_max_mb),
        })?;
        self.repository
            .with_serialized(|| fs::create_dir_all(&self.servers_dir))
            .map_err(config::ConfigError::from)??;
        let servers_dir = &self.servers_dir;
        let staging = tempfile::tempdir_in(servers_dir)?;
        let result = (|| {
            crate::app::extract_zip_entries(&mut zip, staging.path(), "", Some(MANIFEST))?;
            set_port(staging.path(), created.port)?;
            checkpoint(&created)?;
            self.publish(created, staging.path())
        })();
        finish_staging(staging, result)
    }

    /// Copies a server (worlds, plugins, configs) into a new one, optionally on another
    /// Minecraft version so the original stays untouched as a fallback
    pub(crate) fn duplicate(
        &self,
        source: &ServerConfig,
        name: &str,
        game_version: Option<&str>,
        build_number: Option<String>,
    ) -> Result<ServerConfig, TransferError> {
        let version_changes = game_version.is_some_and(|v| v != source.game_version);
        let mut created = self.prepare(ServerCreation {
            name: name.to_string(),
            core: source.core.clone(),
            game_version: game_version.unwrap_or(&source.game_version).to_string(),
            build_number: if version_changes {
                build_number
            } else {
                source.build_number.clone()
            },
            port: None,
            memory_min_mb: Some(source.memory_min_mb),
            memory_max_mb: Some(source.memory_max_mb),
        })?;
        let from = crate::app::safe_enclosed_filename(&self.servers_dir, &source.id)
            .ok_or(config::ConfigError::InvalidId)?;
        let parent = &self.servers_dir;
        let staging = tempfile::tempdir_in(parent)?;
        let result = (|| {
            let to = staging.path();
            copy_dir(&from, to, version_changes)?;
            if version_changes {
                // Forge, NeoForge and Quilt install themselves again for the new version
                crate::server::installer::uninstall(to, &created.core)?;
            }
            set_port(to, created.port)?;

            created.jvm_args = source.jvm_args.clone();
            created.java_path = source.java_path.clone();
            created.sleep_enabled = source.sleep_enabled;
            created.idle_timeout_seconds = source.idle_timeout_seconds;
            self.publish(created, staging.path())
        })();
        finish_staging(staging, result)
    }
}

fn finish_staging(
    stage: tempfile::TempDir,
    result: Result<ServerConfig, TransferError>,
) -> Result<ServerConfig, TransferError> {
    match stage.close() {
        Ok(()) => result,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => result,
        Err(source) => Err(TransferError::StagingCleanup {
            primary: result.err().map(Box::new),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_cleanup_retains_original_failure() {
        let root = tempfile::tempdir().unwrap();
        let stage = tempfile::tempdir_in(root.path()).unwrap();
        let path = stage.path().to_path_buf();
        fs::remove_dir(&path).unwrap();
        fs::write(&path, b"unexpected replacement").unwrap();
        let result = finish_staging(
            stage,
            Err(std::io::Error::other("original copy failure").into()),
        );
        assert!(
            matches!(result, Err(TransferError::StagingCleanup { primary: Some(primary), .. }) if matches!(*primary, TransferError::Io(_)))
        );
        assert_eq!(fs::read(path).unwrap(), b"unexpected replacement");
    }
}
