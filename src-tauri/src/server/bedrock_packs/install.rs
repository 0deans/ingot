use super::{parse_manifest, set_pack_enabled, BedrockPackError, ParsedPackManifest};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
pub async fn install_bedrock_pack(
    server_dir: &Path,
    download_url: &str,
    filename: &str,
) -> Result<String, BedrockPackError> {
    let clean_stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("addon")
        .replace([' ', '(', ')', '[', ']', '+'], "_");

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?;

    let mut req = client.get(download_url);
    if download_url.contains("curseforge.com") {
        req = req.header("x-api-key", crate::minecraft::content::CURSEFORGE_API_KEY);
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(BedrockPackError::HttpStatus(resp.status()));
    }

    let bytes = resp.bytes().await?;

    let preparation_root = server_dir.to_path_buf();
    let prepared = tokio::task::spawn_blocking(move || {
        prepare_pack_bytes(&preparation_root, &bytes, &clean_stem)
    })
    .await??;
    let mut installed = Vec::new();
    for pack in prepared {
        let name = publish_pack(server_dir, pack)?;
        installed.push(name);
    }
    Ok(format!("Installed {}", installed.join(", ")))
}

fn prepare_pack_bytes(
    server_dir: &Path,
    bytes: &[u8],
    clean_stem: &str,
) -> Result<Vec<PreparedPack>, BedrockPackError> {
    let cursor = Cursor::new(bytes);
    let mut zip = ZipArchive::new(cursor)?;

    // Check if archive contains nested .mcpack files (.mcaddon bundle)
    let mut nested_packs: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();
        if name.ends_with(".mcpack") || name.ends_with(".zip") {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            nested_packs.push((name, buf));
        }
    }

    let mut installed_names = Vec::new();

    if !nested_packs.is_empty() {
        for (nested_name, nested_bytes) in nested_packs {
            let sub_stem = Path::new(&nested_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(clean_stem)
                .replace([' ', '(', ')', '[', ']', '+'], "_");

            let mut sub_zip = ZipArchive::new(Cursor::new(&nested_bytes))?;
            let extracted = prepare_single_pack(server_dir, &mut sub_zip, &sub_stem)?;
            installed_names.push(extracted);
        }
    } else {
        let extracted = prepare_single_pack(server_dir, &mut zip, clean_stem)?;
        installed_names.push(extracted);
    }

    Ok(installed_names)
}

struct PreparedPack {
    staging: tempfile::TempDir,
    destination: PathBuf,
    uuid: String,
    name: String,
    pack_type: String,
}

fn prepare_single_pack<R: Read + std::io::Seek>(
    server_dir: &Path,
    zip: &mut ZipArchive<R>,
    pack_name: &str,
) -> Result<PreparedPack, BedrockPackError> {
    // Find manifest to determine if behavior or resource pack
    let mut manifest_bytes = None;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_lowercase().replace('\\', "/");
        if name == "manifest.json" || name.ends_with("/manifest.json") {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            manifest_bytes = Some(buf);
            break;
        }
    }

    let Some(m_bytes) = manifest_bytes else {
        return Err(BedrockPackError::MissingManifest);
    };

    let manifest = parse_manifest(&m_bytes).map_err(BedrockPackError::ManifestJson)?;
    let ParsedPackManifest {
        uuid,
        name,
        pack_type,
        ..
    } = manifest.ok_or(BedrockPackError::InvalidManifest)?;

    let sub = if pack_type == "resource" {
        "resource_packs"
    } else {
        "behavior_packs"
    };
    let parent = server_dir.join(sub);
    let out_dir = crate::app::safe_enclosed_filename(&parent, pack_name)
        .ok_or(BedrockPackError::InvalidFolderName)?;
    if out_dir.exists() {
        return Err(BedrockPackError::AlreadyExists(pack_name.to_string()));
    }
    fs::create_dir_all(&parent)?;
    let staging = tempfile::tempdir_in(&parent)?;
    // Determine strip prefix if files are wrapped in a top folder
    let mut prefix = String::new();
    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        let name = file.name().replace('\\', "/");
        if name.ends_with("/manifest.json") {
            if let Some(p) = name.strip_suffix("manifest.json") {
                prefix = p.to_string();
                break;
            }
        }
    }

    crate::app::extract_zip_entries(zip, staging.path(), &prefix, None)?;
    Ok(PreparedPack {
        staging,
        destination: out_dir,
        uuid,
        name,
        pack_type,
    })
}

fn publish_pack(server_dir: &Path, pack: PreparedPack) -> Result<String, BedrockPackError> {
    publish_pack_with_cleanup(server_dir, pack, |path| fs::remove_dir_all(path))
}

fn publish_pack_with_cleanup(
    server_dir: &Path,
    pack: PreparedPack,
    cleanup: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<String, BedrockPackError> {
    let PreparedPack {
        staging,
        destination: out_dir,
        uuid,
        name,
        pack_type,
    } = pack;
    // Publication contains no await, so cancelling extraction cannot publish later.
    fs::rename(staging.path(), &out_dir)?;
    if let Err(error) = set_pack_enabled(server_dir, &uuid, &pack_type, true) {
        return match cleanup(&out_dir) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(BedrockPackError::CleanupFailed {
                primary: Box::new(error),
                cleanup,
                path: out_dir,
            }),
        };
    }
    Ok(format!("{name} ({pack_type})"))
}

#[cfg(test)]
mod integrity_tests {
    use super::*;
    use std::io::Write;

    fn pack_archive(extra: &str) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("manifest.json", options).unwrap();
        writer.write_all(br#"{"header":{"name":"Pack","uuid":"test-id","version":[1,0,0]},"modules":[{"type":"resources"}]}"#).unwrap();
        writer.start_file(extra, options).unwrap();
        writer.write_all(b"pack data").unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn manifest_archive(manifest: &[u8]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(manifest).unwrap();
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn unsafe_pack_never_publishes_or_enables() {
        let temp = tempfile::tempdir().unwrap();
        let result = prepare_pack_bytes(temp.path(), &pack_archive("../escaped"), "pack");
        assert!(result.is_err());
        assert!(!temp.path().join("resource_packs/pack").exists());
        assert!(!temp
            .path()
            .join("worlds/Bedrock level/world_resource_packs.json")
            .exists());
        assert!(!temp.path().join("escaped").exists());
    }

    #[test]
    fn dropping_prepared_pack_removes_staging_without_installing() {
        let temp = tempfile::tempdir().unwrap();
        let packs =
            prepare_pack_bytes(temp.path(), &pack_archive("textures/test.txt"), "pack").unwrap();
        assert!(!temp.path().join("resource_packs/pack").exists());
        drop(packs);
        assert_eq!(
            fs::read_dir(temp.path().join("resource_packs"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn corrupt_world_references_block_activation_and_preserve_original_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let world = temp.path().join("worlds/Bedrock level");
        fs::create_dir_all(&world).unwrap();
        let refs = world.join("world_resource_packs.json");
        fs::write(&refs, b"invalid references").unwrap();
        let mut packs =
            prepare_pack_bytes(temp.path(), &pack_archive("textures/test.txt"), "pack").unwrap();
        assert!(publish_pack(temp.path(), packs.remove(0)).is_err());
        assert_eq!(fs::read(refs).unwrap(), b"invalid references");
        assert!(!temp.path().join("resource_packs/pack").exists());
    }

    #[test]
    fn failed_publication_cleanup_preserves_both_causes_and_pack_material() {
        use std::error::Error;

        let temp = tempfile::tempdir().unwrap();
        let world = temp.path().join("worlds/Bedrock level");
        fs::create_dir_all(&world).unwrap();
        let refs = world.join("world_resource_packs.json");
        fs::write(&refs, b"invalid references").unwrap();
        let mut packs =
            prepare_pack_bytes(temp.path(), &pack_archive("textures/test.txt"), "pack").unwrap();
        let result = publish_pack_with_cleanup(temp.path(), packs.remove(0), |_| {
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        });
        let error = result.unwrap_err();
        let primary_source = error
            .source()
            .unwrap()
            .downcast_ref::<Box<BedrockPackError>>()
            .unwrap();
        assert!(matches!(primary_source.as_ref(), BedrockPackError::Json(_)));
        let json_source = primary_source.source().unwrap();
        assert!(json_source.downcast_ref::<serde_json::Error>().is_some());
        let BedrockPackError::CleanupFailed {
            primary,
            cleanup,
            path,
        } = error
        else {
            panic!("expected failed rollback cleanup");
        };
        assert!(matches!(*primary, BedrockPackError::Json(_)));
        assert_eq!(cleanup.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(path, temp.path().join("resource_packs/pack"));
        assert_eq!(
            fs::read(path.join("textures/test.txt")).unwrap(),
            b"pack data"
        );
        assert!(path.join("manifest.json").is_file());
        assert_eq!(fs::read(refs).unwrap(), b"invalid references");
    }

    #[test]
    fn malformed_pack_manifest_retains_json_cause_without_publication() {
        use std::error::Error;

        let temp = tempfile::tempdir().unwrap();
        let bytes = manifest_archive(br#"{"header": "#);
        let result = prepare_pack_bytes(temp.path(), &bytes, "pack");
        let error = result.err().unwrap();
        assert!(matches!(error, BedrockPackError::ManifestJson(_)));
        let source = error.source().unwrap();
        let json = source.downcast_ref::<serde_json::Error>().unwrap();
        assert!(json.is_eof());
        assert!(!temp.path().join("behavior_packs").exists());
        assert!(!temp.path().join("resource_packs").exists());
        assert!(!temp.path().join("worlds").exists());
    }

    #[test]
    fn missing_manifest_identity_remains_validation_failure() {
        let temp = tempfile::tempdir().unwrap();
        let bytes = manifest_archive(br#"{"header":{"name":"Pack"}}"#);
        let result = prepare_pack_bytes(temp.path(), &bytes, "pack");
        assert!(matches!(result, Err(BedrockPackError::InvalidManifest)));
        assert!(!temp.path().join("behavior_packs").exists());
    }

    #[test]
    fn optional_scan_keeps_valid_packs_when_other_manifests_are_invalid() {
        let temp = tempfile::tempdir().unwrap();
        let mut packs =
            prepare_pack_bytes(temp.path(), &pack_archive("textures/test.txt"), "valid").unwrap();
        publish_pack(temp.path(), packs.remove(0)).unwrap();
        let invalid = temp.path().join("resource_packs/invalid");
        fs::create_dir_all(&invalid).unwrap();
        fs::write(invalid.join("manifest.json"), b"invalid manifest").unwrap();
        let missing_identity = temp.path().join("resource_packs/missing_identity");
        fs::create_dir_all(&missing_identity).unwrap();
        fs::write(missing_identity.join("manifest.json"), br#"{}"#).unwrap();

        let installed = super::super::list_installed_packs(temp.path()).unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, "test-id");
        assert!(installed[0].enabled);
    }
}
