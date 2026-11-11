use super::{
    construct_curseforge_cdn_url, extract_zip_overrides, parse_curseforge_loader,
    parse_modrinth_loader, CurseForgeManifest, ModrinthIndex,
};
use crate::content::{curseforge::CURSEFORGE_API_KEY, error::ContentError};
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use sha1::Digest;
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
};

pub(crate) struct PreparedModpack {
    pub(crate) stage: Arc<tempfile::TempDir>,
    pub(crate) name: String,
    pub(crate) game_version: String,
    pub(crate) loader: ModLoaderType,
    pub(crate) loader_version: Option<String>,
}

enum Downloads {
    Modrinth(Vec<super::ModrinthIndexFile>),
    CurseForge(Vec<super::CurseForgeManifestFile>),
    Ftb(Vec<super::ftb_pack::FtbVersionFile>),
    TechnicSolder(Vec<super::technic_pack::SolderMod>),
    AtLauncher(Vec<super::atlauncher_pack::AtLauncherMod>),
    None,
}

struct Plan {
    prepared: PreparedModpack,
    downloads: Downloads,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Integrity {
    #[serde(default)]
    pub(super) hashes: HashMap<String, String>,
    #[serde(rename = "fileSize")]
    pub(super) size: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeDownload {
    pub(super) id: u32,
    pub(super) file_name: String,
    pub(super) download_url: Option<String>,
    pub(super) file_length: Option<u64>,
    #[serde(default)]
    pub(super) hashes: Vec<CurseForgeHash>,
}

#[derive(Deserialize)]
pub(super) struct CurseForgeHash {
    pub(super) algo: u8,
    pub(super) value: String,
}

#[derive(Deserialize)]
pub(super) struct CurseForgeResponse {
    pub(super) data: CurseForgeDownload,
}

fn read_manifest<T: serde::de::DeserializeOwned>(
    zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    name: &'static str,
) -> Result<T, ContentError> {
    let mut file = zip
        .by_name(name)
        .map_err(|_| ContentError::MissingManifest(name))?;
    let mut content = Vec::new();
    if let Err(error) = file.read_to_end(&mut content) {
        if !error
            .to_string()
            .to_ascii_lowercase()
            .contains("invalid checksum")
        {
            return Err(ContentError::Io(error));
        }
    }
    serde_json::from_slice(&content).map_err(ContentError::Json)
}

fn plan(
    root: &Path,
    bytes: Vec<u8>,
    source: &str,
    filename: &str,
    requested_name: &str,
    on_progress: Option<&crate::minecraft::launcher::progress::ProgressCallback>,
) -> Result<Plan, ContentError> {
    let src = source.trim().to_lowercase();

    if src == "ftb" || filename.starts_with("ftb-") {
        let (prepared, files) = super::ftb_pack::plan_ftb(root, &bytes, requested_name)?;
        return Ok(Plan {
            prepared,
            downloads: Downloads::Ftb(files),
        });
    }

    if src == "technic" {
        if let Ok(manifest) =
            serde_json::from_slice::<super::technic_pack::SolderBuildManifest>(&bytes)
        {
            if manifest.minecraft.is_some() || !manifest.mods.is_empty() {
                let (prepared, mods) =
                    super::technic_pack::plan_technic_solder(root, manifest, requested_name)?;
                return Ok(Plan {
                    prepared,
                    downloads: Downloads::TechnicSolder(mods),
                });
            }
        }
        let default_mc = filename
            .rsplit_once("-mc")
            .map(|(_, after)| after.strip_suffix(".zip").unwrap_or(after))
            .filter(|s| !s.trim().is_empty());
        let prepared = super::technic_pack::plan_technic_zip(
            root,
            bytes,
            requested_name,
            default_mc,
            on_progress,
        )?;
        return Ok(Plan {
            prepared,
            downloads: Downloads::None,
        });
    }

    if src == "atlauncher" || src == "atl" {
        if let Ok(manifest) =
            serde_json::from_slice::<super::atlauncher_pack::AtLauncherVersionManifest>(&bytes)
        {
            if manifest.minecraft.is_some()
                || manifest.loader.is_some()
                || !manifest.mods.is_empty()
            {
                let (prepared, mods) = super::atlauncher_pack::plan_atlauncher_manifest(
                    root,
                    manifest,
                    requested_name,
                )?;
                return Ok(Plan {
                    prepared,
                    downloads: Downloads::AtLauncher(mods),
                });
            }
        }
        let prepared = super::atlauncher_pack::plan_atlauncher_zip(root, bytes, requested_name)?;
        return Ok(Plan {
            prepared,
            downloads: Downloads::None,
        });
    }

    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(ContentError::Zip)?;
    let is_modrinth = src == "modrinth"
        || filename.ends_with(".mrpack")
        || zip.by_name("modrinth.index.json").is_ok();
    let stage = tempfile::Builder::new()
        .prefix(".modpack-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;
    let (fallback_name, game_version, loader, loader_version, downloads) = if is_modrinth {
        let index: ModrinthIndex = read_manifest(&mut zip, "modrinth.index.json")?;
        let dependencies = index.dependencies.unwrap_or_default();
        let game_version = dependencies
            .get("minecraft")
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .ok_or(ContentError::MissingField("minecraft"))?;
        let (loader, loader_version) = parse_modrinth_loader(&dependencies);
        extract_zip_overrides(&mut zip, &["overrides/", "client-overrides/"], stage.path())
            .map_err(ContentError::Io)?;
        (
            index.name.unwrap_or_else(|| "Modpack Instance".into()),
            game_version,
            loader,
            loader_version,
            Downloads::Modrinth(index.files.unwrap_or_default()),
        )
    } else {
        let manifest: CurseForgeManifest = read_manifest(&mut zip, "manifest.json")?;
        if manifest.minecraft.version.trim().is_empty() {
            return Err(ContentError::MissingField("minecraft.version"));
        }
        let (loader, loader_version) =
            parse_curseforge_loader(&manifest.minecraft.mod_loaders.unwrap_or_default());
        let folder = manifest.overrides.unwrap_or_else(|| "overrides".into());
        if crate::core::safe_enclosed_path(stage.path(), &folder).is_none() {
            return Err(ContentError::UnsafePath(folder));
        }
        extract_zip_overrides(&mut zip, &[&format!("{folder}/")], stage.path())
            .map_err(ContentError::Io)?;
        (
            manifest.name.unwrap_or_else(|| "CurseForge Modpack".into()),
            manifest.minecraft.version,
            loader,
            loader_version,
            Downloads::CurseForge(manifest.files.unwrap_or_default()),
        )
    };
    let name = if requested_name.trim().is_empty() {
        fallback_name
    } else {
        requested_name.trim().to_owned()
    };
    Ok(Plan {
        prepared: PreparedModpack {
            stage: Arc::new(stage),
            name,
            game_version,
            loader,
            loader_version,
        },
        downloads,
    })
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn verify(bytes: &[u8], integrity: &Integrity) -> Result<(), ContentError> {
    if integrity
        .size
        .is_some_and(|size| size != bytes.len() as u64)
    {
        return Err(ContentError::Integrity("size mismatch"));
    }
    for (algorithm, expected) in &integrity.hashes {
        let actual = match algorithm.as_str() {
            "sha1" => hex_digest(sha1::Sha1::digest(bytes)),
            "sha256" => hex_digest(sha2::Sha256::digest(bytes)),
            "sha512" => hex_digest(sha2::Sha512::digest(bytes)),
            "md5" => hex_digest(md5::Md5::digest(bytes)),
            _ => return Err(ContentError::Integrity("unsupported hash algorithm")),
        };
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(ContentError::Integrity("checksum mismatch"));
        }
    }
    Ok(())
}

pub(super) async fn write_download(
    client: &reqwest::Client,
    url: &str,
    stage: &Arc<tempfile::TempDir>,
    relative: &str,
    integrity: Integrity,
) -> Result<(), ContentError> {
    let destination = crate::core::safe_enclosed_path(stage.path(), relative)
        .ok_or_else(|| ContentError::UnsafePath(relative.into()))?;
    let bytes = fetch(client, url).await?;
    let stage = Arc::clone(stage);
    tokio::task::spawn_blocking(move || {
        let _stage = stage;
        verify(&bytes, &integrity)?;
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(ContentError::Io)?;
        }
        std::fs::write(destination, bytes).map_err(ContentError::Io)
    })
    .await
    .map_err(ContentError::Task)?
}

pub(super) async fn fetch_with_progress(
    client: &reqwest::Client,
    url: &str,
    phase: &str,
    label: &str,
    on_progress: Option<&crate::minecraft::launcher::progress::ProgressCallback>,
) -> Result<Vec<u8>, ContentError> {
    let mut request = client.get(url);
    let parsed =
        reqwest::Url::parse(url).map_err(|_| ContentError::MissingField("valid download URL"))?;
    if parsed
        .host_str()
        .is_some_and(|host| host == "curseforge.com" || host.ends_with(".curseforge.com"))
    {
        request = request.header("x-api-key", CURSEFORGE_API_KEY);
    }
    let mut response = request.send().await.map_err(ContentError::Transport)?;
    if !response.status().is_success() {
        return Err(ContentError::HttpStatus(response.status()));
    }
    let total_bytes = response.content_length().unwrap_or(0);
    let mut bytes = if total_bytes > 0 && total_bytes <= (1024 * 1024 * 1024) {
        Vec::with_capacity(total_bytes as usize)
    } else {
        Vec::new()
    };
    let mut downloaded: u64 = 0;
    while let Some(chunk) = response.chunk().await.map_err(ContentError::Response)? {
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        bytes.extend_from_slice(&chunk);
        if let Some(report) = on_progress {
            if total_bytes > 0 {
                report(
                    phase,
                    downloaded,
                    total_bytes,
                    &format!(
                        "{label} ({:.1} MB / {:.1} MB)",
                        downloaded as f64 / 1_048_576.0,
                        total_bytes as f64 / 1_048_576.0
                    ),
                );
            } else {
                report(
                    phase,
                    0,
                    1,
                    &format!("{label} ({:.1} MB)", downloaded as f64 / 1_048_576.0),
                );
            }
        }
    }
    Ok(bytes)
}

pub(super) async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, ContentError> {
    fetch_with_progress(client, url, "", "", None).await
}

pub(crate) struct PrepareModpackParams<'a> {
    pub(crate) client: &'a reqwest::Client,
    pub(crate) curseforge_base: &'a str,
    pub(crate) root: &'a Path,
    pub(crate) bytes: Vec<u8>,
    pub(crate) source: &'a str,
    pub(crate) filename: &'a str,
    pub(crate) name: &'a str,
    pub(crate) manifest_url: Option<&'a str>,
    pub(crate) on_progress: Option<&'a crate::minecraft::launcher::progress::ProgressCallback>,
}

pub(crate) async fn prepare_modpack(
    params: PrepareModpackParams<'_>,
) -> Result<PreparedModpack, ContentError> {
    let client = params.client;
    let curseforge_base = params.curseforge_base;
    let manifest_url = params.manifest_url;
    let on_progress = params.on_progress;
    let root = params.root.to_owned();
    let source = params.source.to_owned();
    let filename = params.filename.to_owned();
    let name = params.name.to_owned();
    let bytes = params.bytes;
    let on_progress_plan = on_progress.cloned();
    let mut plan = tokio::task::spawn_blocking(move || {
        plan(
            &root,
            bytes,
            &source,
            &filename,
            &name,
            on_progress_plan.as_ref(),
        )
    })
    .await
    .map_err(ContentError::Task)??;
    let result = async {
        match plan.downloads {
            Downloads::Modrinth(files) => {
                let client_files: Vec<_> = files
                    .into_iter()
                    .filter(|file| {
                        !file
                            .env
                            .as_ref()
                            .and_then(|env| env.get("client"))
                            .is_some_and(|value| value == "unsupported")
                    })
                    .collect();
                let total = client_files.len() as u64;
                for (idx, file) in client_files.into_iter().enumerate() {
                    if let Some(report) = on_progress {
                        report(
                            "Installing Modpack",
                            idx as u64,
                            total,
                            &format!("Downloading {} ({}/{})", file.path, idx + 1, total),
                        );
                    }
                    let url = file
                        .downloads
                        .first()
                        .ok_or(ContentError::MissingField("downloads"))?;
                    write_download(
                        client,
                        url,
                        &plan.prepared.stage,
                        &file.path,
                        file.integrity,
                    )
                    .await?;
                }
                if let Some(report) = on_progress {
                    report(
                        "Installing Modpack",
                        total,
                        total.max(1),
                        "Finalizing modpack installation...",
                    );
                }
            }
            Downloads::CurseForge(files) => {
                let total = files.len() as u64;
                for (idx, file) in files.into_iter().enumerate() {
                    if let Some(report) = on_progress {
                        report(
                            "Installing Modpack",
                            idx as u64,
                            total,
                            &format!("Downloading mod {}/{}...", idx + 1, total),
                        );
                    }
                    let url = format!(
                        "{}/mods/{}/files/{}",
                        curseforge_base.trim_end_matches('/'),
                        file.project_id,
                        file.file_id
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
                        .json::<CurseForgeResponse>()
                        .await
                        .map_err(ContentError::Response)?
                        .data;
                    let destination = format!("mods/{}", item.file_name);
                    if crate::core::safe_enclosed_filename(
                        plan.prepared.stage.path(),
                        &item.file_name,
                    )
                    .is_none()
                    {
                        return Err(ContentError::InvalidFilename(item.file_name));
                    }
                    if let Some(report) = on_progress {
                        report(
                            "Installing Modpack",
                            idx as u64,
                            total,
                            &format!("Downloading {} ({}/{})", item.file_name, idx + 1, total),
                        );
                    }
                    let download = item
                        .download_url
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| construct_curseforge_cdn_url(item.id, &item.file_name));
                    let mut integrity = Integrity {
                        size: item.file_length,
                        hashes: HashMap::new(),
                    };
                    for hash in item.hashes {
                        let algorithm = match hash.algo {
                            1 => "sha1",
                            2 => "md5",
                            _ => {
                                return Err(ContentError::Integrity(
                                    "unsupported CurseForge hash algorithm",
                                ))
                            }
                        };
                        integrity.hashes.insert(algorithm.into(), hash.value);
                    }
                    write_download(
                        client,
                        &download,
                        &plan.prepared.stage,
                        &destination,
                        integrity,
                    )
                    .await?;
                }
                if let Some(report) = on_progress {
                    report(
                        "Installing Modpack",
                        total,
                        total.max(1),
                        "Finalizing modpack installation...",
                    );
                }
            }
            Downloads::Ftb(files) => {
                super::ftb_pack::download_ftb_files(
                    client,
                    curseforge_base,
                    &plan.prepared.stage,
                    files,
                    on_progress,
                )
                .await?;
            }
            Downloads::TechnicSolder(mods) => {
                let (gv, ldr, lv) = super::technic_pack::download_solder_mods(
                    client,
                    &plan.prepared.stage,
                    mods,
                    &plan.prepared.game_version,
                    on_progress,
                )
                .await?;
                plan.prepared.game_version = gv;
                plan.prepared.loader = ldr;
                plan.prepared.loader_version = lv;
            }
            Downloads::AtLauncher(mods) => {
                let configs_zip_url = manifest_url.and_then(|u| {
                    if u.ends_with("/Configs.json") {
                        Some(format!(
                            "{}/Configs.zip",
                            u.trim_end_matches("/Configs.json")
                        ))
                    } else {
                        None
                    }
                });
                let atl_base = manifest_url
                    .and_then(|u| u.split("/packs/").next())
                    .unwrap_or(crate::content::atlauncher::DEFAULT_ATLAUNCHER_BASE);
                super::atlauncher_pack::download_atlauncher_mods(
                    client,
                    curseforge_base,
                    atl_base,
                    &plan.prepared.stage,
                    mods,
                    configs_zip_url.as_deref(),
                    on_progress,
                )
                .await?;
            }
            Downloads::None => {
                if let Some(report) = on_progress {
                    report(
                        "Installing Modpack",
                        1,
                        1,
                        "Finalizing modpack installation...",
                    );
                }
            }
        }
        Ok(())
    }
    .await;
    if let Err(primary) = result {
        let stage = match Arc::try_unwrap(plan.prepared.stage) {
            Ok(stage) => stage,
            Err(_) => {
                return Err(ContentError::Cleanup {
                    primary: Box::new(primary),
                    cleanup: std::io::Error::other("staging is still owned by a writer"),
                })
            }
        };
        if let Err(cleanup) = stage.close() {
            return Err(ContentError::Cleanup {
                primary: Box::new(primary),
                cleanup,
            });
        }
        return Err(primary);
    }
    Ok(plan.prepared)
}
