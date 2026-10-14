//! Installation routines for individual content files and full modpack instances.
use super::{
    curseforge::{CurseForgeClient, CURSEFORGE_API_KEY},
    error::ContentError,
};
use crate::minecraft::instance::{self, InstanceConfig, ModLoaderType};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read, Seek},
    path::Path,
};
use tauri::Runtime;
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct ModrinthIndex {
    pub(crate) game: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) dependencies: Option<HashMap<String, String>>,
    pub(crate) files: Option<Vec<ModrinthIndexFile>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModrinthIndexFile {
    pub(crate) path: String,
    pub(crate) downloads: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeManifest {
    pub(crate) minecraft: CurseForgeManifestMinecraft,
    pub(crate) name: Option<String>,
    pub(crate) files: Option<Vec<CurseForgeManifestFile>>,
    pub(crate) overrides: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeManifestMinecraft {
    pub(crate) version: String,
    pub(crate) mod_loaders: Option<Vec<CurseForgeManifestModLoader>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeManifestModLoader {
    pub(crate) id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeManifestFile {
    #[serde(rename = "projectID")]
    pub(crate) project_id: u32,
    #[serde(rename = "fileID")]
    pub(crate) file_id: u32,
}

/// Maps content project type string to the appropriate target subfolder within an instance.
pub(crate) fn target_subfolder_for_project_type(project_type: &str) -> &'static str {
    match project_type.to_lowercase().as_str() {
        "mod" => "mods",
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        _ => "mods",
    }
}

/// Parses the loader type and optional loader version from Modrinth dependencies.
pub(crate) fn parse_modrinth_loader(
    deps: &HashMap<String, String>,
) -> (ModLoaderType, Option<String>) {
    if let Some(lv) = deps.get("fabric-loader") {
        (ModLoaderType::Fabric, Some(lv.clone()))
    } else if let Some(lv) = deps.get("forge") {
        (ModLoaderType::Forge, Some(lv.clone()))
    } else if let Some(lv) = deps.get("neoforge") {
        (ModLoaderType::NeoForge, Some(lv.clone()))
    } else if let Some(lv) = deps.get("quilt-loader") {
        (ModLoaderType::Quilt, Some(lv.clone()))
    } else {
        (ModLoaderType::Vanilla, None)
    }
}

/// Parses the loader type and optional loader version from CurseForge manifest loaders.
pub(crate) fn parse_curseforge_loader(
    loaders: &[CurseForgeManifestModLoader],
) -> (ModLoaderType, Option<String>) {
    let Some(primary) = loaders.first() else {
        return (ModLoaderType::Vanilla, None);
    };

    let id = primary.id.to_lowercase();
    if let Some(ver) = id.strip_prefix("fabric-") {
        (ModLoaderType::Fabric, Some(ver.to_string()))
    } else if let Some(ver) = id.strip_prefix("forge-") {
        (ModLoaderType::Forge, Some(ver.to_string()))
    } else if let Some(ver) = id.strip_prefix("neoforge-") {
        (ModLoaderType::NeoForge, Some(ver.to_string()))
    } else if let Some(ver) = id.strip_prefix("quilt-") {
        (ModLoaderType::Quilt, Some(ver.to_string()))
    } else {
        (ModLoaderType::Vanilla, None)
    }
}

/// Constructs a fallback CDN URL for a CurseForge file if none was directly provided.
pub(crate) fn construct_curseforge_cdn_url(file_id: u32, file_name: &str) -> String {
    let id_str = file_id.to_string();
    let (part1, part2) = if id_str.len() > 4 {
        let split = id_str.len() - 3;
        (&id_str[..split], &id_str[split..])
    } else {
        ("0", id_str.as_str())
    };
    format!("https://edge.forgecdn.net/files/{part1}/{part2}/{file_name}")
}

/// Extracts overrides directory from a modpack archive to the target instance directory.
pub(crate) fn extract_zip_overrides<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    prefixes: &[&str],
    dest_dir: &Path,
) -> Result<(), std::io::Error> {
    for i in 0..zip.len() {
        let mut file = match zip.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let file_name = file.name().to_string();
        let subpath = prefixes
            .iter()
            .find_map(|prefix| file_name.strip_prefix(*prefix));

        let Some(subpath) = subpath else {
            continue;
        };

        let Some(out_path) = crate::app::safe_enclosed_path(dest_dir, subpath) else {
            continue;
        };

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = match fs::File::create(&out_path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let _ = std::io::copy(&mut file, &mut outfile);
        }
    }
    Ok(())
}

/// Installs a single content file (mod, resource pack, or shader) into an instance directory.
pub async fn install_content_file<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    project_type: &str,
    download_url: &str,
    filename: &str,
) -> Result<String, ContentError> {
    let instance_dir =
        instance::get_instance_dir(app, instance_id).map_err(ContentError::Instance)?;

    let clean_name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ContentError::InvalidFilename(filename.to_string()))?;

    let subfolder = target_subfolder_for_project_type(project_type);
    let target_dir = instance_dir.join(subfolder);
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).map_err(ContentError::Io)?;
    }

    let target_file = crate::app::safe_enclosed_filename(&target_dir, clean_name)
        .ok_or_else(|| ContentError::InvalidFilename(filename.to_string()))?;
    let tmp_file = target_dir.join(format!("{clean_name}.tmp.{}", std::process::id()));

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(ContentError::ClientInit)?;

    let mut req = client.get(download_url);
    if download_url.contains("curseforge.com") {
        req = req.header("x-api-key", CURSEFORGE_API_KEY);
    }

    let resp = req.send().await.map_err(ContentError::Transport)?;
    if !resp.status().is_success() {
        return Err(ContentError::HttpStatus(resp.status()));
    }

    let bytes = resp.bytes().await.map_err(ContentError::Response)?;
    fs::write(&tmp_file, &bytes).map_err(ContentError::Io)?;
    fs::rename(&tmp_file, &target_file).map_err(ContentError::Io)?;

    Ok(format!("Installed {clean_name} into {subfolder}"))
}

/// Installs a full modpack archive (.mrpack or CurseForge zip) into a new instance.
pub async fn install_modpack_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: &str,
    source: &str,
    download_url: &str,
    filename: &str,
) -> Result<InstanceConfig, ContentError> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(ContentError::ClientInit)?;

    let mut req = client.get(download_url);
    if download_url.contains("curseforge.com") {
        req = req.header("x-api-key", CURSEFORGE_API_KEY);
    }

    let resp = req.send().await.map_err(ContentError::Transport)?;
    if !resp.status().is_success() {
        return Err(ContentError::HttpStatus(resp.status()));
    }

    let pack_bytes = resp.bytes().await.map_err(ContentError::Response)?;
    let cursor = Cursor::new(pack_bytes);
    let mut zip = ZipArchive::new(cursor).map_err(ContentError::Zip)?;

    let src = source.trim().to_lowercase();
    let is_modrinth = src == "modrinth"
        || filename.ends_with(".mrpack")
        || zip.by_name("modrinth.index.json").is_ok();

    if is_modrinth {
        install_modrinth_pack(app, &client, &mut zip, name).await
    } else {
        install_curseforge_pack(app, &client, &mut zip, name).await
    }
}

async fn install_modrinth_pack<R: Runtime, S: Read + Seek>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    zip: &mut ZipArchive<S>,
    name: &str,
) -> Result<InstanceConfig, ContentError> {
    let index_data = {
        let mut file = zip
            .by_name("modrinth.index.json")
            .map_err(|_| ContentError::MissingManifest("modrinth.index.json"))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(ContentError::Io)?;
        content
    };

    let index: ModrinthIndex = serde_json::from_str(&index_data).map_err(ContentError::Json)?;
    let deps = index.dependencies.unwrap_or_default();
    let game_version = deps
        .get("minecraft")
        .cloned()
        .unwrap_or_else(|| "1.20.1".into());
    let (loader, loader_version) = parse_modrinth_loader(&deps);

    let inst_name = if !name.trim().is_empty() {
        name.trim().to_string()
    } else {
        index.name.unwrap_or_else(|| "Modpack Instance".into())
    };

    let instance = instance::create_instance(app, inst_name, game_version, loader, loader_version)
        .map_err(ContentError::Instance)?;
    let inst_dir = instance::get_instance_dir(app, &instance.id).map_err(ContentError::Instance)?;

    let _ = extract_zip_overrides(zip, &["overrides/", "client-overrides/"], &inst_dir);

    if let Some(files) = index.files {
        let mods_dir = inst_dir.join("mods");
        let _ = fs::create_dir_all(&mods_dir);

        for f in files {
            if let Some(dl_url) = f.downloads.first() {
                let Some(dest) = crate::app::safe_enclosed_path(&inst_dir, &f.path) else {
                    continue;
                };
                if let Some(p) = dest.parent() {
                    let _ = fs::create_dir_all(p);
                }
                if let Ok(dl_resp) = client.get(dl_url).send().await {
                    if dl_resp.status().is_success() {
                        if let Ok(b) = dl_resp.bytes().await {
                            let _ = fs::write(&dest, &b);
                        }
                    }
                }
            }
        }
    }

    Ok(instance)
}

async fn install_curseforge_pack<R: Runtime, S: Read + Seek>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    zip: &mut ZipArchive<S>,
    name: &str,
) -> Result<InstanceConfig, ContentError> {
    let manifest_data = {
        let mut file = zip
            .by_name("manifest.json")
            .map_err(|_| ContentError::MissingManifest("manifest.json"))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(ContentError::Io)?;
        content
    };

    let manifest: CurseForgeManifest =
        serde_json::from_str(&manifest_data).map_err(ContentError::Json)?;

    let game_version = manifest.minecraft.version;
    let loaders = manifest.minecraft.mod_loaders.unwrap_or_default();
    let (loader, loader_version) = parse_curseforge_loader(&loaders);

    let inst_name = if !name.trim().is_empty() {
        name.trim().to_string()
    } else {
        manifest.name.unwrap_or_else(|| "CurseForge Modpack".into())
    };

    let instance = instance::create_instance(app, inst_name, game_version, loader, loader_version)
        .map_err(ContentError::Instance)?;
    let inst_dir = instance::get_instance_dir(app, &instance.id).map_err(ContentError::Instance)?;

    let overrides_folder = manifest.overrides.unwrap_or_else(|| "overrides".into());
    let prefix = format!("{overrides_folder}/");
    let _ = extract_zip_overrides(zip, &[&prefix], &inst_dir);

    if let Some(files) = manifest.files {
        let mods_dir = inst_dir.join("mods");
        let _ = fs::create_dir_all(&mods_dir);
        let cf_client = CurseForgeClient::with_endpoint(
            client.clone(),
            super::curseforge::DEFAULT_CURSEFORGE_BASE,
            CURSEFORGE_API_KEY,
        );

        for f in files {
            if let Ok(item) = cf_client.get_file(f.project_id, f.file_id).await {
                let dl_url = item
                    .download_url
                    .filter(|u| !u.is_empty())
                    .unwrap_or_else(|| construct_curseforge_cdn_url(item.id, &item.file_name));

                let Some(dest) = crate::app::safe_enclosed_filename(&mods_dir, &item.file_name) else {
                    continue;
                };
                if let Ok(dl_resp) = client.get(&dl_url).send().await {
                    if dl_resp.status().is_success() {
                        if let Ok(b) = dl_resp.bytes().await {
                            let _ = fs::write(&dest, &b);
                        }
                    }
                }
            }
        }
    }

    Ok(instance)
}
