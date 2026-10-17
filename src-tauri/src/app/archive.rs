use std::io::{Read, Seek};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("Archive contains an unsafe path or unsupported link")]
    UnsafeEntry,
    #[error("Archive entry could not be read")]
    Zip(#[from] zip::result::ZipError),
    #[error("Archive extraction failed")]
    Io(#[from] std::io::Error),
}

/// Extract only into an exclusively owned staging tree. Symlinks and special
/// entries are unsupported; existing links/junctions are rejected at each write.
pub(crate) fn extract_zip_entries<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    destination: &Path,
    strip_prefix: &str,
    skip: Option<&str>,
) -> Result<(), ArchiveError> {
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().replace('\\', "/");
        if skip == Some(name.as_str()) {
            continue;
        }
        if let Some(mode) = entry.unix_mode() {
            let kind = mode & 0o170000;
            if kind != 0 && kind != 0o100000 && kind != 0o040000 {
                return Err(ArchiveError::UnsafeEntry);
            }
        }
        // Validate the original name too: stripping a prefix must not conceal
        // traversal or a Windows device path in an archive entry.
        let source_name = name.trim_end_matches('/');
        super::paths::safe_enclosed_path(destination, source_name)
            .ok_or(ArchiveError::UnsafeEntry)?;
        let Some(relative) = name.strip_prefix(strip_prefix) else {
            continue;
        };
        let relative = relative.trim_end_matches('/');
        if relative.is_empty() {
            continue;
        }
        let output = super::paths::safe_enclosed_path(destination, relative)
            .ok_or(ArchiveError::UnsafeEntry)?;
        if entry.is_dir() {
            std::fs::create_dir_all(output)?;
        } else {
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?;
            std::io::copy(&mut entry, &mut file)?;
            file.sync_all()?;
            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(mode & 0o777))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn zip_with(name: &str) -> zip::ZipArchive<Cursor<Vec<u8>>> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"content").unwrap();
        zip::ZipArchive::new(writer.finish().unwrap()).unwrap()
    }

    #[test]
    fn rejects_portable_unsafe_archive_paths() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "../escape",
            "..\\escape",
            "C:\\escape",
            "/escape",
            "CON.txt",
            "file:stream",
        ] {
            let mut archive = zip_with(name);
            assert!(matches!(
                extract_zip_entries(&mut archive, temp.path(), "", None),
                Err(ArchiveError::UnsafeEntry)
            ));
        }
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    #[test]
    fn refuses_overwriting_existing_file_and_extracts_valid_tree() {
        let temp = tempfile::tempdir().unwrap();
        let mut archive = zip_with("wrapped/config/options.txt");
        extract_zip_entries(&mut archive, temp.path(), "wrapped/", None).unwrap();
        let target = temp.path().join("config/options.txt");
        assert_eq!(std::fs::read(&target).unwrap(), b"content");
        let mut second = zip_with("config/options.txt");
        assert!(extract_zip_entries(&mut second, temp.path(), "", None).is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"content");
    }

    #[test]
    fn rejects_archive_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_symlink(
                "linked",
                "../outside",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        let mut archive = zip::ZipArchive::new(writer.finish().unwrap()).unwrap();
        assert!(matches!(
            extract_zip_entries(&mut archive, temp.path(), "", None),
            Err(ArchiveError::UnsafeEntry)
        ));
    }
}
