//! Technic Platform package planner (Solder manifest and direct zip).
use super::{extract_zip_overrides, pack::PreparedModpack};
use crate::content::error::ContentError;
use crate::importer::version_id::parse_vanilla_version_id;
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::{
    fs,
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
};
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
pub(crate) struct SolderBuildManifest {
    pub(crate) minecraft: Option<String>,
    #[serde(default)]
    pub(crate) mods: Vec<SolderMod>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct SolderMod {
    pub(crate) name: String,
    pub(crate) md5: Option<String>,
    pub(crate) url: String,
}

#[derive(Deserialize)]
struct VersionJson {
    id: Option<String>,
}

fn read_version_id_from_jar(jar_path: &Path) -> Option<String> {
    let file = fs::File::open(jar_path).ok()?;
    let mut zip = ZipArchive::new(file).ok()?;
    let mut entry = zip.by_name("version.json").ok()?;
    let mut content = Vec::new();
    if let Err(err) = entry.read_to_end(&mut content) {
        if !err
            .to_string()
            .to_ascii_lowercase()
            .contains("invalid checksum")
        {
            return None;
        }
    }
    let vj = serde_json::from_slice::<VersionJson>(&content).ok()?;
    vj.id.filter(|s| !s.trim().is_empty())
}

pub(crate) fn detect_loader_from_dir(
    dir: &Path,
    default_game_version: Option<&str>,
) -> (String, ModLoaderType, Option<String>) {
    let bin_version = dir.join("bin").join("version.json");
    let root_version = dir.join("version.json");

    for candidate in [&bin_version, &root_version] {
        if candidate.exists() {
            if let Ok(content) = fs::read_to_string(candidate) {
                if let Ok(vj) = serde_json::from_str::<VersionJson>(&content) {
                    if let Some(id) = vj.id.filter(|s| !s.trim().is_empty()) {
                        let (gv, ldr, lv) = parse_vanilla_version_id(&id);
                        let final_gv = if gv.is_empty() {
                            default_game_version.unwrap_or("1.20.1").to_string()
                        } else {
                            gv
                        };
                        return (final_gv, ldr, lv);
                    }
                }
            }
        }
    }

    let modpack_jar = dir.join("bin").join("modpack.jar");
    if modpack_jar.exists() {
        if let Some(id) = read_version_id_from_jar(&modpack_jar) {
            let (gv, ldr, lv) = parse_vanilla_version_id(&id);
            let final_gv = if gv.is_empty() {
                default_game_version.unwrap_or("1.20.1").to_string()
            } else {
                gv
            };
            return (final_gv, ldr, lv);
        }
    }

    let has_mods = dir
        .join("mods")
        .read_dir()
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("jar"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    let gv = default_game_version.unwrap_or("1.20.1").to_string();
    if has_mods {
        (gv, ModLoaderType::Forge, None)
    } else {
        (gv, ModLoaderType::Vanilla, None)
    }
}

pub(crate) fn plan_technic_solder(
    root: &Path,
    manifest: SolderBuildManifest,
    requested_name: &str,
) -> Result<(PreparedModpack, Vec<SolderMod>), ContentError> {
    let stage = tempfile::Builder::new()
        .prefix(".technic-solder-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;

    let game_version = manifest
        .minecraft
        .clone()
        .unwrap_or_else(|| "1.20.1".to_string());

    let name = if requested_name.trim().is_empty() {
        "Technic Modpack".to_string()
    } else {
        requested_name.trim().to_owned()
    };

    Ok((
        PreparedModpack {
            stage: Arc::new(stage),
            name,
            game_version,
            loader: ModLoaderType::Forge,
            loader_version: None,
        },
        manifest.mods,
    ))
}

pub(crate) fn plan_technic_zip(
    root: &Path,
    bytes: Vec<u8>,
    requested_name: &str,
    default_game_version: Option<&str>,
    on_progress: Option<&crate::minecraft::launcher::progress::ProgressCallback>,
) -> Result<PreparedModpack, ContentError> {
    if let Some(report) = on_progress {
        report("Installing Modpack", 0, 1, "Extracting modpack archive...");
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(ContentError::Zip)?;
    let stage = tempfile::Builder::new()
        .prefix(".technic-zip-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;

    extract_zip_overrides(&mut zip, &[""], stage.path()).map_err(ContentError::Io)?;

    let (game_version, loader, loader_version) =
        detect_loader_from_dir(stage.path(), default_game_version);

    if let Some(report) = on_progress {
        report("Installing Modpack", 1, 1, "Extracting modpack archive...");
    }

    let name = if requested_name.trim().is_empty() {
        "Technic Modpack".to_string()
    } else {
        requested_name.trim().to_owned()
    };

    Ok(PreparedModpack {
        stage: Arc::new(stage),
        name,
        game_version,
        loader,
        loader_version,
    })
}

pub(crate) async fn download_solder_mods(
    client: &reqwest::Client,
    stage: &Arc<tempfile::TempDir>,
    mods: Vec<SolderMod>,
    default_game_version: &str,
    on_progress: Option<&crate::minecraft::launcher::progress::ProgressCallback>,
) -> Result<(String, ModLoaderType, Option<String>), ContentError> {
    let total = mods.len() as u64;
    for (idx, smod) in mods.into_iter().enumerate() {
        if let Some(report) = on_progress {
            report(
                "Installing Modpack",
                idx as u64,
                total,
                &format!("Downloading {} ({}/{})", smod.name, idx + 1, total),
            );
        }
        let bytes = super::pack::fetch(client, &smod.url).await?;
        let mut hashes = std::collections::HashMap::new();
        if let Some(md5) = smod.md5.filter(|s| !s.trim().is_empty()) {
            hashes.insert("md5".to_string(), md5);
        }
        let integrity = super::pack::Integrity { hashes, size: None };
        super::pack::verify(&bytes, &integrity)?;

        let stage_clone = Arc::clone(stage);
        tokio::task::spawn_blocking(move || {
            let _stage = stage_clone;
            let mut mod_zip =
                zip::ZipArchive::new(Cursor::new(bytes)).map_err(ContentError::Zip)?;
            extract_zip_overrides(&mut mod_zip, &[""], _stage.path()).map_err(ContentError::Io)
        })
        .await
        .map_err(ContentError::Task)??;
    }
    if let Some(report) = on_progress {
        report(
            "Installing Modpack",
            total,
            total.max(1),
            "Finalizing modpack installation...",
        );
    }
    Ok(detect_loader_from_dir(
        stage.path(),
        Some(default_game_version),
    ))
}
