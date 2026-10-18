use super::*;
use crate::ipc::error::ApiError;
use crate::version_change::models::{ContentKind, Folder, Target};
use crate::version_change::sources::Scope;
use std::collections::HashMap;
use std::io;

fn test_target(root: &Path) -> Target {
    Target {
        root: root.to_path_buf(),
        folders: vec![Folder {
            kind: ContentKind::Mod,
            dir: "mods",
            scope: Scope {
                curseforge: false,
                ..Default::default()
            },
        }],
        worlds: Vec::new(),
        configs: Vec::new(),
        core_files: Vec::new(),
        hangar: HashMap::new(),
    }
}

#[test]
fn missing_optional_directory_remains_acceptable() {
    let temp = tempfile::tempdir().unwrap();
    let target = test_target(temp.path());

    // "mods" directory does not exist on disk
    let result = scan(&target);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert!(files.is_empty());
}

#[test]
fn unreadable_directory_reports_failure() {
    let temp = tempfile::tempdir().unwrap();
    let target = test_target(temp.path());

    let result = scan_with(
        &target,
        |path| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("unreadable directory: {}", path.display()),
            ))
        },
        |_| Ok(Vec::new()),
    );

    assert!(matches!(
        result,
        Err(VersionChangeError::ScanDirectory { ref path, ref source })
            if path == &temp.path().join("mods") && source.kind() == io::ErrorKind::PermissionDenied
    ));

    // Verify IPC category maps to storage_failure
    let api_err = ApiError::from(result.err().unwrap());
    assert!(matches!(api_err, ApiError::StorageFailure { .. }));
}

#[test]
fn directory_entry_failure_reports_failure() {
    let temp = tempfile::tempdir().unwrap();
    let target = test_target(temp.path());

    let result = scan_with(
        &target,
        |_| Ok(vec![Err(io::Error::other("corrupt entry inspection"))]),
        |_| Ok(Vec::new()),
    );

    assert!(matches!(
        result,
        Err(VersionChangeError::ScanDirectory { ref source, .. })
            if source.to_string().contains("corrupt entry inspection")
    ));
}

#[test]
fn file_read_failure_reports_failure() {
    let temp = tempfile::tempdir().unwrap();
    let target = test_target(temp.path());

    let result = scan_with(
        &target,
        |_| {
            Ok(vec![Ok(DiscoveredEntry {
                path: temp.path().join("mods/broken.jar"),
                file_name: "broken.jar".to_string(),
                is_dir: false,
            })])
        },
        |path| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("cannot read file: {}", path.display()),
            ))
        },
    );

    assert!(matches!(
        result,
        Err(VersionChangeError::ScanFile { ref path, ref source })
            if path == &temp.path().join("mods/broken.jar") && source.kind() == io::ErrorKind::PermissionDenied
    ));

    let api_err = ApiError::from(result.err().unwrap());
    assert!(matches!(api_err, ApiError::StorageFailure { .. }));
}

#[test]
fn irrelevant_files_and_directories_are_intentionally_skipped() {
    let temp = tempfile::tempdir().unwrap();
    let mods_dir = temp.path().join("mods");
    fs::create_dir_all(&mods_dir).unwrap();

    // 1. Subdirectory inside mods/
    fs::create_dir(mods_dir.join("subfolder")).unwrap();
    // 2. Dotfile / hidden file
    fs::write(mods_dir.join(".DS_Store"), b"hidden").unwrap();
    // 3. Non-matching file extensions for ContentKind::Mod (.jar)
    fs::write(mods_dir.join("readme.txt"), b"information").unwrap();
    fs::write(mods_dir.join("disabled.jar.disabled"), b"turned off").unwrap();
    fs::write(mods_dir.join("settings.json"), b"{}").unwrap();
    // 4. Valid mod jar
    fs::write(mods_dir.join("valid_mod.jar"), b"some jar bytes").unwrap();

    let target = test_target(temp.path());
    let result = scan(&target).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].file_name, "valid_mod.jar");
    assert_eq!(result[0].folder, 0);
    assert_eq!(result[0].sha1, sha1_hex(b"some jar bytes"));
}

#[test]
fn malformed_filename_reports_failure() {
    let temp = tempfile::tempdir().unwrap();
    let target = test_target(temp.path());

    // Filename containing illegal path traversal/characters
    let result = scan_with(
        &target,
        |_| {
            Ok(vec![Ok(DiscoveredEntry {
                path: temp.path().join("mods/../escaped.jar"),
                file_name: "../escaped.jar".to_string(),
                is_dir: false,
            })])
        },
        |_| Ok(b"mod content".to_vec()),
    );

    assert!(matches!(
        result,
        Err(VersionChangeError::InvalidFileName(ref name)) if name == "../escaped.jar"
    ));

    let api_err = ApiError::from(result.err().unwrap());
    assert!(matches!(api_err, ApiError::InvalidInput { .. }));
}

#[test]
fn world_size_calculation_reports_failure_on_unreadable_data() {
    let temp = tempfile::tempdir().unwrap();
    let mut target = test_target(temp.path());
    target.worlds = vec!["saves/CorruptWorld".to_string()];

    let result = list_worlds_with(&target, |path| {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("unreadable world directory: {}", path.display()),
        ))
    });

    assert!(matches!(
        result,
        Err(VersionChangeError::ScanDirectory { ref path, ref source })
            if path == &temp.path().join("saves/CorruptWorld") && source.kind() == io::ErrorKind::PermissionDenied
    ));
}

#[test]
fn dir_size_typed_measures_correctly_and_handles_missing_file() {
    let temp = tempfile::tempdir().unwrap();
    let world_dir = temp.path().join("world");
    fs::create_dir(&world_dir).unwrap();

    fs::write(world_dir.join("level.dat"), b"12345678").unwrap(); // 8 bytes
    fs::create_dir(world_dir.join("region")).unwrap();
    fs::write(world_dir.join("region/r.0.0.mca"), b"1234").unwrap(); // 4 bytes

    let measured = dir_size_typed(&world_dir).unwrap();
    assert_eq!(measured, 12);

    let missing = dir_size_typed(&temp.path().join("nonexistent")).unwrap();
    assert_eq!(missing, 0);
}
