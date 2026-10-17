use super::{error::ConfigError, models::ServerConfig};
use crate::minecraft::metadata::{DeletionFiles, MetadataRepository};
use std::path::Path;

pub(crate) fn delete_server_record(
    repository: &MetadataRepository<ServerConfig>,
    server_id: &str,
    directory: Option<&Path>,
) -> Result<(), ConfigError> {
    if let Some(directory) = directory {
        let parent = directory.parent().ok_or(ConfigError::InvalidId)?;
        let expected =
            crate::app::safe_enclosed_filename(parent, server_id).ok_or(ConfigError::InvalidId)?;
        if expected != directory {
            return Err(ConfigError::InvalidId);
        }
    }
    let files = match directory {
        Some(_) => DeletionFiles::Remove,
        None => DeletionFiles::Retain,
    };
    repository.delete_record(server_id, files)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn server(id: &str) -> ServerConfig {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": "Server", "core": "paper", "gameVersion": "1.20.1",
            "port": 25565, "memoryMinMb": 1024, "memoryMaxMb": 2048, "createdAt": 0
        }))
        .unwrap()
    }

    #[test]
    fn deletion_preserves_files_when_requested_and_retains_other_servers() {
        for remove_files in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let directory = root.path().join("servers");
            let target = directory.join("remove");
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join("world"), b"save").unwrap();
            let repository = MetadataRepository::with_deletion_directory(
                root.path().join("servers.json"),
                directory,
                |server: &ServerConfig| &server.id,
            );
            repository
                .update(|servers| {
                    servers.extend([server("remove"), server("keep")]);
                    Ok(())
                })
                .unwrap();
            delete_server_record(
                &repository,
                "remove",
                remove_files.then_some(target.as_path()),
            )
            .unwrap();
            let records = repository.load().unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].id, "keep");
            assert_eq!(target.exists(), !remove_files);
        }
    }

    #[test]
    fn invalid_directory_cannot_delete_metadata_or_files() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("servers");
        let repository = MetadataRepository::with_deletion_directory(
            root.path().join("servers.json"),
            directory.clone(),
            |server: &ServerConfig| &server.id,
        );
        repository
            .update(|servers| {
                servers.push(server("keep"));
                Ok(())
            })
            .unwrap();
        let result = delete_server_record(&repository, "keep", Some(&directory.join("other")));
        assert!(matches!(result, Err(ConfigError::InvalidId)));
        assert_eq!(repository.load().unwrap().len(), 1);
    }
}
