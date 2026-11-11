//! FTB version JSON manifest parser and plan generator.
use super::pack::PreparedModpack;
use crate::content::{
    curseforge::CURSEFORGE_API_KEY,
    error::ContentError,
    ftb::{deserialize_opt_u64_or_string, deserialize_u32_or_string},
};
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::{path::Path, sync::Arc};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FtbVersionManifest {
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    pub(crate) id: Option<u64>,
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
#[allow(dead_code)]
pub(crate) struct FtbVersionFile {
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    pub(crate) id: Option<u64>,
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
    let name = if requested_name.trim().is_empty() {
        fallback_name
    } else {
        requested_name.trim().to_owned()
    };

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

pub(crate) async fn download_ftb_files(
    client: &reqwest::Client,
    curseforge_base: &str,
    stage: &Arc<tempfile::TempDir>,
    files: Vec<FtbVersionFile>,
    on_progress: Option<&crate::minecraft::launcher::progress::ProgressCallback>,
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
            if crate::core::safe_enclosed_path(stage.path(), &relative).is_none() {
                return Err(ContentError::UnsafePath(relative));
            }
            let mut hashes = std::collections::HashMap::new();
            if let Some(sha1) = file.sha1.filter(|s| !s.trim().is_empty()) {
                hashes.insert("sha1".to_string(), sha1);
            }
            let integrity = super::pack::Integrity {
                hashes,
                size: file.size.filter(|&s| s > 0),
            };
            super::pack::write_download(client, &url, stage, &relative, integrity).await?;
        } else if let Some(cf) = file.curseforge {
            let url = format!(
                "{}/mods/{}/files/{}",
                curseforge_base.trim_end_matches('/'),
                cf.project,
                cf.file
            );
            let response = client
                .get(url)
                .header("x-api-key", CURSEFORGE_API_KEY)
                .send()
                .await
                .map_err(ContentError::Transport)?;
            if !response.status().is_success() {
                return Err(ContentError::HttpStatus(response.status()));
            }
            let item = response
                .json::<super::pack::CurseForgeResponse>()
                .await
                .map_err(ContentError::Response)?
                .data;
            let fname = if !file.name.trim().is_empty() {
                file.name.as_str()
            } else {
                item.file_name.as_str()
            };
            let relative = if clean_path.is_empty() {
                format!("mods/{}", fname)
            } else {
                format!("{}/{}", clean_path.trim_end_matches('/'), fname)
            };
            if crate::core::safe_enclosed_path(stage.path(), &relative).is_none() {
                return Err(ContentError::UnsafePath(relative));
            }
            let download = item
                .download_url
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| super::construct_curseforge_cdn_url(item.id, &item.file_name));
            let mut integrity = super::pack::Integrity {
                size: item.file_length,
                hashes: std::collections::HashMap::new(),
            };
            for hash in item.hashes {
                let algorithm = match hash.algo {
                    1 => "sha1",
                    2 => "md5",
                    _ => return Err(ContentError::Integrity("unsupported hash algorithm")),
                };
                integrity.hashes.insert(algorithm.into(), hash.value);
            }
            super::pack::write_download(client, &download, stage, &relative, integrity).await?;
        }
    }
    if let Some(report) = on_progress {
        report(
            "Installing Modpack",
            total,
            total.max(1),
            "Finalizing modpack installation...",
        );
    }
    Ok(())
}
