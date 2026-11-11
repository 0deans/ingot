use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

/// Returns `true` if `metadata` represents a symbolic link or, on Windows, a reparse point
/// (such as a junction).
pub fn is_link_or_reparse_point(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Syncs `staged` contents to disk and atomically persists it over `destination`, optionally
/// syncing the parent directory on Unix.
pub fn persist_staged_file(
    staged: NamedTempFile,
    destination: &Path,
    sync_parent: bool,
) -> io::Result<()> {
    staged.as_file().sync_all()?;
    staged.persist(destination).map_err(|error| error.error)?;
    if sync_parent {
        #[cfg(unix)]
        if let Some(parent) = destination.parent() {
            fs::File::open(parent)?.sync_all()?;
        }
    }
    Ok(())
}

/// Atomically writes `contents` to `destination` via a staged temporary file in the same
/// parent directory, syncing file contents before persisting.
pub fn write_file_atomic(destination: &Path, contents: &[u8]) -> io::Result<()> {
    write_file_atomic_with_parent_sync(destination, contents, false)
}

/// Atomically writes `contents` to `destination` via a staged temporary file, syncing file
/// contents and optionally syncing the parent directory on Unix before returning.
pub fn write_file_atomic_with_parent_sync(
    destination: &Path,
    contents: &[u8],
    sync_parent: bool,
) -> io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent directory",
        )
    })?;
    let mut staged = NamedTempFile::new_in(parent)?;
    staged.write_all(contents)?;
    persist_staged_file(staged, destination, sync_parent)
}

/// Atomically copies `source` to `destination` via a staged temporary file in `destination`'s
/// parent directory, syncing file contents before persisting.
pub fn copy_file_atomic(source: &Path, destination: &Path) -> io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent directory",
        )
    })?;
    let mut staged = NamedTempFile::new_in(parent)?;
    let mut input = fs::File::open(source)?;
    io::copy(&mut input, &mut staged)?;
    persist_staged_file(staged, destination, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_file_atomic_creates_and_overwrites_without_leaving_tempfiles() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("data.json");

        write_file_atomic(&target, br#"{"v":1}"#).unwrap();
        assert_eq!(fs::read(&target).unwrap(), br#"{"v":1}"#);

        write_file_atomic_with_parent_sync(&target, br#"{"v":2}"#, true).unwrap();
        assert_eq!(fs::read(&target).unwrap(), br#"{"v":2}"#);

        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn write_file_atomic_fails_without_parent() {
        let result = write_file_atomic(Path::new(""), b"abc");
        assert!(result.is_err());
    }

    #[test]
    fn copy_file_atomic_preserves_existing_destination_on_missing_source() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.txt");
        let dest = dir.path().join("dest.txt");
        fs::write(&dest, b"keep-me").unwrap();

        let err = copy_file_atomic(&missing, &dest).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert_eq!(fs::read(&dest).unwrap(), b"keep-me");
    }

    #[test]
    fn copy_file_atomic_replaces_existing_file_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        fs::write(&source, b"new-content").unwrap();
        fs::write(&dest, b"old-content").unwrap();

        copy_file_atomic(&source, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new-content");

        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn is_link_or_reparse_point_ignores_regular_files_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("regular.txt");
        fs::write(&file, b"ok").unwrap();

        assert!(!is_link_or_reparse_point(
            &fs::symlink_metadata(dir.path()).unwrap()
        ));
        assert!(!is_link_or_reparse_point(
            &fs::symlink_metadata(&file).unwrap()
        ));
    }

    #[cfg(unix)]
    #[test]
    fn is_link_or_reparse_point_detects_unix_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        fs::write(&target, b"ok").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(is_link_or_reparse_point(
            &fs::symlink_metadata(&link).unwrap()
        ));
    }
}
