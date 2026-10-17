use super::{parse_manifest, set_pack_enabled};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
pub async fn install_bedrock_pack(
    server_dir: &Path,
    download_url: &str,
    filename: &str,
) -> Result<String, String> {
    let clean_stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("addon")
        .replace([' ', '(', ')', '[', ']', '+'], "_");

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.get(download_url);
    if download_url.contains("curseforge.com") {
        req = req.header("x-api-key", crate::minecraft::content::CURSEFORGE_API_KEY);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Download failed with status: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;

    let preparation_root = server_dir.to_path_buf();
    let prepared = tokio::task::spawn_blocking(move || {
        prepare_pack_bytes(&preparation_root, &bytes, &clean_stem)
    })
    .await
    .map_err(|error| format!("Pack extraction task failed: {error}"))??;
    let mut installed = Vec::new();
    for pack in prepared {
        let name = publish_pack(server_dir, pack).map_err(|error| {
            format!(
                "{error}; previously installed packs: {}",
                installed.join(", ")
            )
        })?;
        installed.push(name);
    }
    Ok(format!("Installed {}", installed.join(", ")))
}

fn prepare_pack_bytes(
    server_dir: &Path,
    bytes: &[u8],
    clean_stem: &str,
) -> Result<Vec<PreparedPack>, String> {
    let cursor = Cursor::new(bytes);
    let mut zip = ZipArchive::new(cursor).map_err(|e| format!("Invalid archive file: {e}"))?;

    // Check if archive contains nested .mcpack files (.mcaddon bundle)
    let mut nested_packs: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|error| error.to_string())?;
        let name = file.name().to_string();
        if name.ends_with(".mcpack") || name.ends_with(".zip") {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|error| error.to_string())?;
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

            let mut sub_zip =
                ZipArchive::new(Cursor::new(&nested_bytes)).map_err(|error| error.to_string())?;
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
) -> Result<PreparedPack, String> {
    // Find manifest to determine if behavior or resource pack
    let mut manifest_bytes = None;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|error| error.to_string())?;
        let name = file.name().to_lowercase().replace('\\', "/");
        if name == "manifest.json" || name.ends_with("/manifest.json") {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|error| error.to_string())?;
            manifest_bytes = Some(buf);
            break;
        }
    }

    let Some(m_bytes) = manifest_bytes else {
        return Err("No manifest.json found in pack archive".to_string());
    };

    let (uuid, name, _, _, pack_type) =
        parse_manifest(&m_bytes).ok_or_else(|| "Failed to parse pack manifest.json".to_string())?;

    let sub = if pack_type == "resource" {
        "resource_packs"
    } else {
        "behavior_packs"
    };
    let parent = server_dir.join(sub);
    let out_dir = crate::app::safe_enclosed_filename(&parent, pack_name)
        .ok_or_else(|| "Invalid pack folder name or linked destination".to_string())?;
    if out_dir.exists() {
        return Err(
            "Pack folder already exists; remove it explicitly before installing".to_string(),
        );
    }
    fs::create_dir_all(&parent).map_err(|error| error.to_string())?;
    let staging = tempfile::tempdir_in(&parent).map_err(|error| error.to_string())?;
    // Determine strip prefix if files are wrapped in a top folder
    let mut prefix = String::new();
    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(|error| error.to_string())?;
        let name = file.name().replace('\\', "/");
        if name.ends_with("/manifest.json") {
            if let Some(p) = name.strip_suffix("manifest.json") {
                prefix = p.to_string();
                break;
            }
        }
    }

    crate::app::extract_zip_entries(zip, staging.path(), &prefix, None)
        .map_err(|error| error.to_string())?;
    Ok(PreparedPack {
        staging,
        destination: out_dir,
        uuid,
        name,
        pack_type,
    })
}

fn publish_pack(server_dir: &Path, pack: PreparedPack) -> Result<String, String> {
    let PreparedPack {
        staging,
        destination: out_dir,
        uuid,
        name,
        pack_type,
    } = pack;
    // Publication contains no await, so cancelling extraction cannot publish later.
    fs::rename(staging.path(), &out_dir).map_err(|error| error.to_string())?;
    if let Err(error) = set_pack_enabled(server_dir, &uuid, &pack_type, true) {
        return match fs::remove_dir_all(&out_dir) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!(
                "{error}; removing unpublished pack failed: {cleanup}; retained at {}",
                out_dir.display()
            )),
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
}
