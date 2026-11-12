//! FTB version JSON manifest parser and plan generator.
use super::curseforge_resolver::{self, CurseForgeEndpoints};
use super::support::{self, Integrity, PreparedModpack};
use crate::content::{
    error::ContentError,
    serde_helpers::{deserialize_opt_u64_or_string, deserialize_u32_or_string},
};
use ingot_minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::{path::Path, sync::Arc};

#[derive(Debug, Deserialize)]
pub(crate) struct FtbVersionManifest {
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) targets: Vec<FtbVersionTarget>,
    #[serde(default)]
    pub(crate) files: Vec<FtbVersionFile>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct FtbVersionTarget {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) r#type: String,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct FtbVersionFile {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) path: String,
    pub(crate) url: Option<String>,
    pub(crate) sha1: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    pub(crate) size: Option<u64>,
    #[serde(default)]
    pub(crate) serveronly: bool,
    pub(crate) curseforge: Option<FtbCurseForgeRef>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct FtbCurseForgeRef {
    #[serde(deserialize_with = "deserialize_u32_or_string")]
    pub(crate) project: u32,
    #[serde(deserialize_with = "deserialize_u32_or_string")]
    pub(crate) file: u32,
}

pub(crate) fn parse_ftb_targets(
    targets: &[FtbVersionTarget],
) -> (String, ModLoaderType, Option<String>) {
    let mut game_version = "1.20.1".to_string();
    let mut loader = ModLoaderType::Vanilla;
    let mut loader_version = None;

    for target in targets {
        let t_type = target.r#type.to_lowercase();
        let t_name = target.name.to_lowercase();
        if t_type == "game" || t_name == "minecraft" {
            if let Some(v) = &target.version {
                game_version = v.clone();
            }
        } else if t_type == "modloader" {
            loader = match t_name.as_str() {
                "forge" => ModLoaderType::Forge,
                "neoforge" => ModLoaderType::NeoForge,
                "fabric" => ModLoaderType::Fabric,
                "quilt" => ModLoaderType::Quilt,
                _ => ModLoaderType::Vanilla,
            };
            loader_version = target.version.clone();
        }
    }

    (game_version, loader, loader_version)
}

pub(crate) fn plan_ftb(
    root: &Path,
    bytes: &[u8],
    requested_name: &str,
) -> Result<(PreparedModpack, Vec<FtbVersionFile>), ContentError> {
    let manifest: FtbVersionManifest = serde_json::from_slice(bytes).map_err(ContentError::Json)?;

    let (game_version, loader, loader_version) = parse_ftb_targets(&manifest.targets);

    let stage = tempfile::Builder::new()
        .prefix(".ftb-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;

    let fallback_name = manifest.name.unwrap_or_else(|| "FTB Modpack".to_string());
    let name = support::resolve_pack_name(requested_name, fallback_name);

    Ok((
        PreparedModpack {
            stage: Arc::new(stage),
            name,
            game_version,
            loader,
            loader_version,
        },
        manifest.files,
    ))
}

pub(super) async fn download_ftb_files(
    client: &reqwest::Client,
    endpoints: CurseForgeEndpoints<'_>,
    stage: &Arc<tempfile::TempDir>,
    files: Vec<FtbVersionFile>,
    on_progress: Option<&ingot_minecraft::launcher::progress::ProgressCallback>,
) -> Result<(), ContentError> {
    let client_files: Vec<_> = files.into_iter().filter(|f| !f.serveronly).collect();
    let total = client_files.len() as u64;
    for (idx, file) in client_files.into_iter().enumerate() {
        let label = if !file.name.trim().is_empty() {
            file.name.clone()
        } else {
            format!("CurseForge file {}", idx + 1)
        };
        if let Some(report) = on_progress {
            report(
                "Installing Modpack",
                idx as u64,
                total,
                &format!("Downloading {} ({}/{})", label, idx + 1, total),
            );
        }
        let clean_path = file
            .path
            .trim()
            .trim_start_matches("./")
            .trim_start_matches('/');

        if let Some(url) = file.url.filter(|s| !s.trim().is_empty()) {
            let relative = if clean_path.is_empty() {
                file.name.clone()
            } else {
                format!("{}/{}", clean_path.trim_end_matches('/'), file.name)
            };
            support::validate_stage_path(stage.path(), &relative)?;
            let mut hashes = std::collections::HashMap::new();
            if let Some(sha1) = file.sha1.filter(|s| !s.trim().is_empty()) {
                hashes.insert("sha1".to_string(), sha1);
            }
            let integrity = Integrity {
                hashes,
                size: file.size.filter(|&s| s > 0),
            };
            support::write_download(client, &url, stage, &relative, integrity).await?;
        } else if let Some(cf) = file.curseforge {
            let resolved = curseforge_resolver::resolve_curseforge_file_with_cdn(
                client, endpoints, cf.project, cf.file,
            )
            .await?;
            let fname = if !file.name.trim().is_empty() {
                file.name.as_str()
            } else {
                resolved.file_name.as_str()
            };
            let relative = if clean_path.is_empty() {
                format!("mods/{fname}")
            } else {
                format!("{}/{fname}", clean_path.trim_end_matches('/'))
            };
            support::validate_stage_path(stage.path(), &relative)?;
            support::write_download(
                client,
                &resolved.download_url,
                stage,
                &relative,
                resolved.integrity,
            )
            .await?;
        }
    }
    support::report_finalizing(on_progress, total);
    Ok(())
}
