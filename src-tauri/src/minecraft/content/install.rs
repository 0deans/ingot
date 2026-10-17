//! Installation routines for individual content files and full modpack instances.
use super::error::ContentError;
use crate::minecraft::instance::{self, InstanceConfig, ModLoaderType};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    io::{Read, Seek},
    path::Path,
};
use tauri::Runtime;
use zip::ZipArchive;

mod pack;
#[cfg(test)]
mod pack_tests;

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
    #[serde(flatten)]
    integrity: pack::Integrity,
    pub(super) env: Option<HashMap<String, String>>,
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
        let mut file = zip.by_index(i).map_err(std::io::Error::other)?;

        let file_name = file.name().to_string();
        let subpath = prefixes
            .iter()
            .find_map(|prefix| file_name.strip_prefix(*prefix));

        let Some(subpath) = subpath else {
            continue;
        };
        let subpath = subpath.trim_end_matches('/');
        if subpath.is_empty() {
            continue;
        }
        if let Some(mode) = file.unix_mode() {
            let kind = mode & 0o170000;
            if kind != 0 && kind != 0o100000 && kind != 0o040000 {
                return Err(std::io::Error::other(
                    "Modpack overrides must contain plain files or directories",
                ));
            }
        }
        let out_path = crate::app::safe_enclosed_path(dest_dir, subpath)
            .ok_or_else(|| std::io::Error::other("Unsafe modpack override path"))?;

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut outfile)?;
            outfile.sync_all()?;
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

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(ContentError::ClientInit)?;

    let bytes = pack::fetch(&client, download_url).await?;
    tokio::task::spawn_blocking(move || publish_content_file(&target_file, &bytes))
        .await
        .map_err(ContentError::Task)??;

    Ok(format!("Installed {clean_name} into {subfolder}"))
}

fn publish_content_file(destination: &Path, bytes: &[u8]) -> Result<(), ContentError> {
    use std::io::Write;
    let directory = destination
        .parent()
        .ok_or(ContentError::MissingField("destination directory"))?;
    let mut staged = tempfile::NamedTempFile::new_in(directory).map_err(ContentError::Io)?;
    staged.write_all(bytes).map_err(ContentError::Io)?;
    staged.as_file().sync_all().map_err(ContentError::Io)?;
    staged
        .persist(destination)
        .map_err(|error| ContentError::Io(error.error))?;
    Ok(())
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
    let bytes = pack::fetch(&client, download_url).await?;
    let root = instance::get_instances_dir(app).map_err(ContentError::Instance)?;
    let prepared = pack::prepare_modpack(
        &client,
        super::curseforge::DEFAULT_CURSEFORGE_BASE,
        &root,
        bytes,
        source,
        filename,
        name,
    )
    .await?;
    let config = instance::prepare_instance(
        prepared.name,
        prepared.game_version,
        prepared.loader,
        prepared.loader_version,
    )
    .map_err(ContentError::Instance)?;
    publish_modpack(
        &crate::app::state(app).instances,
        &root,
        config,
        prepared.stage.path(),
    )
}

fn publish_modpack(
    repository: &crate::minecraft::metadata::MetadataRepository<InstanceConfig>,
    parent: &Path,
    config: InstanceConfig,
    staging: &Path,
) -> Result<InstanceConfig, ContentError> {
    let destination = crate::app::safe_enclosed_filename(parent, &config.id)
        .ok_or_else(|| ContentError::InvalidFilename(config.id.clone()))?;
    repository
        .publish(
            staging,
            &destination,
            config.clone(),
            |records, candidate| {
                if records.iter().any(|record| record.id == candidate.id) {
                    return Err(crate::minecraft::metadata::MetadataError::Conflict(
                        "Instance already exists".into(),
                    ));
                }
                Ok(())
            },
        )
        .map_err(ContentError::Metadata)?;
    Ok(config)
}
