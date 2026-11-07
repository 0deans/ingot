//! ATLauncher modpack planner (manifest and zip archive).
use super::{extract_zip_overrides, pack::PreparedModpack};
use crate::minecraft::{content::error::ContentError, instance::ModLoaderType};
use serde::Deserialize;
use std::{io::Cursor, path::Path, sync::Arc};
use zip::ZipArchive;

pub(crate) fn format_cdn_url(base: &str, relative: &str) -> String {
    let base_trimmed = base.trim_end_matches('/');
    let rel_trimmed = relative.trim_start_matches('/');
    let encoded_path = rel_trimmed
        .split('/')
        .map(|segment| {
            let mut encoded = String::with_capacity(segment.len());
            for b in segment.as_bytes() {
                match b {
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        encoded.push(*b as char);
                    }
                    _ => {
                        use std::fmt::Write;
                        let _ = write!(encoded, "%{:02X}", b);
                    }
                }
            }
            encoded
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("{base_trimmed}/{encoded_path}")
}

fn default_client() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub(crate) struct AtLauncherVersionManifest {
    pub(crate) minecraft: Option<String>,
    pub(crate) loader: Option<AtLauncherLoader>,
    #[serde(default)]
    pub(crate) mods: Vec<AtLauncherMod>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AtLauncherLoader {
    pub(crate) r#type: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) metadata: Option<AtLauncherLoaderMetadata>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AtLauncherLoaderMetadata {
    pub(crate) version: Option<String>,
    pub(crate) forge: Option<String>,
    pub(crate) loader: Option<String>,
    #[serde(rename = "rawVersion")]
    pub(crate) raw_version: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AtLauncherMod {
    pub(crate) name: String,
    pub(crate) file: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) download: Option<String>,
    pub(crate) md5: Option<String>,
    pub(crate) sha1: Option<String>,
    pub(crate) filesize: Option<u64>,
    #[serde(
        default,
        deserialize_with = "crate::minecraft::content::ftb::deserialize_opt_u32_or_string"
    )]
    pub(crate) curse_id: Option<u32>,
    #[serde(
        default,
        deserialize_with = "crate::minecraft::content::ftb::deserialize_opt_u32_or_string"
    )]
    pub(crate) curse_file_id: Option<u32>,
    #[serde(default)]
    pub(crate) optional: bool,
    #[serde(default)]
    pub(crate) selected: bool,
    #[serde(default)]
    pub(crate) server: bool,
    #[serde(default = "default_client")]
    pub(crate) client: bool,
    #[serde(rename = "type")]
    pub(crate) mod_type: Option<String>,
    pub(crate) subfolder: Option<String>,
}

pub(crate) fn target_subfolder_for_mod(m: &AtLauncherMod) -> &'static str {
    if let Some(sub) = m.subfolder.as_deref() {
        let s = sub.trim().trim_start_matches('/').trim_end_matches('/');
        if s.eq_ignore_ascii_case("mods") {
            return "mods";
        } else if s.eq_ignore_ascii_case("resourcepacks") {
            return "resourcepacks";
        } else if s.eq_ignore_ascii_case("shaderpacks") {
            return "shaderpacks";
        } else if s.eq_ignore_ascii_case("config") {
            return "config";
        }
    }
    match m
        .mod_type
        .as_deref()
        .unwrap_or("mods")
        .to_lowercase()
        .as_str()
    {
        "resourcepack" | "resourcepacks" => "resourcepacks",
        "shader" | "shaders" | "shaderpack" | "shaderpacks" => "shaderpacks",
        "config" | "configs" => "config",
        _ => "mods",
    }
}

pub(crate) fn extract_loader(
    loader: Option<&AtLauncherLoader>,
    game_version: &str,
) -> (ModLoaderType, Option<String>) {
    let Some(ldr) = loader else {
        return (ModLoaderType::Forge, None);
    };

    let loader_type = match ldr.r#type.as_deref().unwrap_or("").to_lowercase().as_str() {
        "fabric" => ModLoaderType::Fabric,
        "quilt" => ModLoaderType::Quilt,
        "neoforge" => ModLoaderType::NeoForge,
        "forge" => ModLoaderType::Forge,
        _ => ModLoaderType::Vanilla,
    };

    let raw_ver = ldr
        .metadata
        .as_ref()
        .and_then(|m| {
            m.version
                .as_deref()
                .or(m.forge.as_deref())
                .or(m.loader.as_deref())
                .or(m.raw_version.as_deref())
        })
        .or(ldr.version.as_deref());

    let loader_version = raw_ver.and_then(|v| {
        let cleaned = crate::minecraft::loader::normalize_loader_version(game_version, v);
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    });

    (loader_type, loader_version)
}

pub(crate) fn plan_atlauncher_manifest(
    root: &Path,
    manifest: AtLauncherVersionManifest,
    requested_name: &str,
) -> Result<(PreparedModpack, Vec<AtLauncherMod>), ContentError> {
    let stage = tempfile::Builder::new()
        .prefix(".atlauncher-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;

    let game_version = manifest.minecraft.unwrap_or_else(|| "1.20.1".to_string());

    let (loader, loader_version) = extract_loader(manifest.loader.as_ref(), &game_version);

    let name = if requested_name.trim().is_empty() {
        "ATLauncher Modpack".to_string()
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
        manifest.mods,
    ))
}

pub(crate) fn plan_atlauncher_zip(
    root: &Path,
    bytes: Vec<u8>,
    requested_name: &str,
) -> Result<PreparedModpack, ContentError> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(ContentError::Zip)?;
    let stage = tempfile::Builder::new()
        .prefix(".atlauncher-zip-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;

    extract_zip_overrides(&mut zip, &[""], stage.path()).map_err(ContentError::Io)?;

    let name = if requested_name.trim().is_empty() {
        "ATLauncher Modpack".to_string()
    } else {
        requested_name.trim().to_owned()
    };

    Ok(PreparedModpack {
        stage: Arc::new(stage),
        name,
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Forge,
        loader_version: None,
    })
}

pub(crate) async fn download_atlauncher_mods(
    client: &reqwest::Client,
    curseforge_base: &str,
    atlauncher_base: &str,
    stage: &Arc<tempfile::TempDir>,
    mods: Vec<AtLauncherMod>,
    configs_zip_url: Option<&str>,
    on_progress: Option<&crate::minecraft::launcher::progress::ProgressCallback>,
) -> Result<(), ContentError> {
    let client_mods: Vec<_> = mods
        .into_iter()
        .filter(|m| !(m.optional && !m.selected) && !(m.server && !m.client))
        .collect();
    let total = client_mods.len() as u64;
    for (idx, m) in client_mods.into_iter().enumerate() {
        if let Some(report) = on_progress {
            report(
                "Installing Modpack",
                idx as u64,
                total,
                &format!("Downloading {} ({}/{})", m.name, idx + 1, total),
            );
        }
        let subfolder = target_subfolder_for_mod(&m);
        let dl_mode = m.download.as_deref().unwrap_or("").to_lowercase();
        let is_curse = dl_mode == "curse" || dl_mode == "curseforge";

        if !is_curse && m.url.as_deref().is_some_and(|u| !u.trim().is_empty()) {
            let raw_url = m.url.as_deref().unwrap_or("");
            let download_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://")
            {
                raw_url.to_string()
            } else {
                format_cdn_url(atlauncher_base, raw_url)
            };
            let filename = m.file.as_deref().unwrap_or(m.name.as_str());
            let destination = format!("{subfolder}/{filename}");
            if crate::app::safe_enclosed_path(stage.path(), &destination).is_none() {
                return Err(ContentError::UnsafePath(destination));
            }
            let mut hashes = std::collections::HashMap::new();
            if let Some(sha1) = m.sha1.filter(|s| !s.trim().is_empty()) {
                hashes.insert("sha1".to_string(), sha1);
            }
            if let Some(md5) = m.md5.filter(|s| !s.trim().is_empty()) {
                hashes.insert("md5".to_string(), md5);
            }
            let integrity = super::pack::Integrity {
                hashes,
                size: m.filesize.filter(|&s| s > 0),
            };
            super::pack::write_download(client, &download_url, stage, &destination, integrity)
                .await?;
        } else if let (Some(curse_id), Some(curse_file_id)) = (m.curse_id, m.curse_file_id) {
            let url = format!(
                "{}/mods/{}/files/{}",
                curseforge_base.trim_end_matches('/'),
                curse_id,
                curse_file_id
            );
            let response = client
                .get(url)
                .header(
                    "x-api-key",
                    crate::minecraft::content::curseforge::CURSEFORGE_API_KEY,
                )
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
            let fname = m.file.as_deref().unwrap_or(item.file_name.as_str());
            let destination = format!("{subfolder}/{fname}");
            if crate::app::safe_enclosed_path(stage.path(), &destination).is_none() {
                return Err(ContentError::UnsafePath(destination));
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
            super::pack::write_download(client, &download, stage, &destination, integrity).await?;
        }
    }

    if let Some(zip_url) = configs_zip_url {
        if let Some(report) = on_progress {
            report(
                "Installing Modpack",
                total,
                total.max(1),
                "Extracting modpack configuration...",
            );
        }
        if let Ok(res) = client.get(zip_url).send().await {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes().await {
                    let stage_clone = Arc::clone(stage);
                    tokio::task::spawn_blocking(move || {
                        let mut zip = ZipArchive::new(Cursor::new(bytes.to_vec()))
                            .map_err(ContentError::Zip)?;
                        extract_zip_overrides(&mut zip, &[""], stage_clone.path())
                            .map_err(ContentError::Io)
                    })
                    .await
                    .map_err(ContentError::Task)??;
                }
            }
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
