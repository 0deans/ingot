use super::*;
use std::error::Error;
use std::io::{Cursor, Write};

fn archive_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn prepare_failure(parent: &Path, bytes: &[u8]) -> BedrockServerError {
    match prepare_archive(
        parent,
        bytes,
        std::ffi::OsStr::new(get_bedrock_binary_name()),
    ) {
        Err(error) => error,
        Ok(_) => panic!("archive preparation unexpectedly succeeded"),
    }
}

#[test]
fn unsafe_archive_preserves_error_and_existing_installation() {
    let temp = tempfile::tempdir().unwrap();
    let binary_path = get_bedrock_binary_path(temp.path());
    std::fs::write(&binary_path, b"original executable").unwrap();
    let bytes = archive_with(&[("../escaped", b"unsafe")]);

    let error = prepare_failure(temp.path(), &bytes);

    assert!(matches!(
        error,
        BedrockServerError::Archive(crate::app::ArchiveError::UnsafeEntry)
    ));
    assert!(error.source().unwrap().is::<crate::app::ArchiveError>());
    assert_eq!(std::fs::read(binary_path).unwrap(), b"original executable");
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn missing_executable_is_distinct_and_does_not_publish_files() {
    let temp = tempfile::tempdir().unwrap();
    let bytes = archive_with(&[("server.properties", b"new properties")]);

    let error = prepare_failure(temp.path(), &bytes);

    match error {
        BedrockServerError::BinaryNotFound(path) => {
            assert_eq!(path.file_name().unwrap(), get_bedrock_binary_name());
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn malformed_archive_preserves_zip_cause() {
    let temp = tempfile::tempdir().unwrap();
    let error = prepare_failure(temp.path(), b"not a zip archive");

    assert!(matches!(error, BedrockServerError::Zip(_)));
    assert!(error.source().unwrap().is::<zip::result::ZipError>());
}

#[test]
fn archive_write_failure_preserves_io_cause() {
    let temp = tempfile::tempdir().unwrap();
    let bytes = archive_with(&[("config/options", b"first"), ("config\\options", b"second")]);

    let error = prepare_failure(temp.path(), &bytes);

    match &error {
        BedrockServerError::Archive(crate::app::ArchiveError::Io(_)) => {}
        other => panic!("unexpected error: {other:?}"),
    }
    let archive_error = error.source().unwrap();
    assert!(archive_error.is::<crate::app::ArchiveError>());
    assert!(archive_error.source().unwrap().is::<std::io::Error>());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn prepared_files_keep_executable_last_and_leave_installation_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let bytes = archive_with(&[
        (get_bedrock_binary_name(), b"executable"),
        ("config/options", b"options"),
        ("server.properties", b"properties"),
    ]);

    let prepared = prepare_archive(
        temp.path(),
        &bytes,
        std::ffi::OsStr::new(get_bedrock_binary_name()),
    )
    .unwrap();

    assert_eq!(prepared.files.len(), 3);
    assert_eq!(
        prepared.files.last().unwrap().0,
        PathBuf::from(get_bedrock_binary_name())
    );
    assert!(!get_bedrock_binary_path(temp.path()).exists());
    assert!(!temp.path().join("server.properties").exists());
    drop(prepared);
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn staging_prefix_failure_preserves_paths_and_cause() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let other = temp.path().join("other");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let path = other.join("file");
    std::fs::write(&path, b"content").unwrap();

    let error = prepare_files(&root, &other, &mut Vec::new()).unwrap_err();

    match &error {
        BedrockServerError::RelativePath {
            path: actual,
            root: actual_root,
            ..
        } => {
            assert_eq!(actual, &path);
            assert_eq!(actual_root, &root);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(error.source().unwrap().is::<std::path::StripPrefixError>());
    assert_eq!(std::fs::read(path).unwrap(), b"content");
}
