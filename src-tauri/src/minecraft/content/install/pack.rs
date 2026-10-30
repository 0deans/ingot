use super::{
    construct_curseforge_cdn_url, extract_zip_overrides, parse_curseforge_loader,
    parse_modrinth_loader, CurseForgeManifest, ModrinthIndex,
};
use crate::minecraft::{
    content::{curseforge::CURSEFORGE_API_KEY, error::ContentError},
    instance::ModLoaderType,
};
use serde::Deserialize;
use sha1::Digest;
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
};

pub(super) struct PreparedModpack {
    pub(super) stage: Arc<tempfile::TempDir>,
    pub(super) name: String,
    pub(super) game_version: String,
    pub(super) loader: ModLoaderType,
    pub(super) loader_version: Option<String>,
}

enum Downloads {
    Modrinth(Vec<super::ModrinthIndexFile>),
    CurseForge(Vec<super::CurseForgeManifestFile>),
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
struct CurseForgeDownload {
    id: u32,
    file_name: String,
    download_url: Option<String>,
    file_length: Option<u64>,
    #[serde(default)]
    hashes: Vec<CurseForgeHash>,
}

#[derive(Deserialize)]
struct CurseForgeHash {
    algo: u8,
    value: String,
}

#[derive(Deserialize)]
struct CurseForgeResponse {
    data: CurseForgeDownload,
}

fn read_manifest<T: serde::de::DeserializeOwned>(
    zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>,
    name: &'static str,
) -> Result<T, ContentError> {
    let mut file = zip
        .by_name(name)
        .map_err(|_| ContentError::MissingManifest(name))?;
    let mut content = Vec::new();
    file.read_to_end(&mut content).map_err(ContentError::Io)?;
    serde_json::from_slice(&content).map_err(ContentError::Json)
}

fn plan(
    root: &Path,
    bytes: Vec<u8>,
    source: &str,
    filename: &str,
    requested_name: &str,
) -> Result<Plan, ContentError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(ContentError::Zip)?;
    let is_modrinth = source.trim().eq_ignore_ascii_case("modrinth")
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
        if crate::app::safe_enclosed_path(stage.path(), &folder).is_none() {
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

async fn write_download(
    client: &reqwest::Client,
    url: &str,
    stage: &Arc<tempfile::TempDir>,
    relative: &str,
    integrity: Integrity,
) -> Result<(), ContentError> {
    let destination = crate::app::safe_enclosed_path(stage.path(), relative)
        .ok_or_else(|| ContentError::UnsafePath(relative.into()))?;
    let bytes = fetch(client, url).await?;
    let stage = Arc::clone(stage);
    tokio::task::spawn_blocking(move || {
        // A cancelled caller must not remove the stage while this writer owns it.
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

pub(super) async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, ContentError> {
    let mut request = client.get(url);
    let parsed =
        reqwest::Url::parse(url).map_err(|_| ContentError::MissingField("valid download URL"))?;
    if parsed
        .host_str()
        .is_some_and(|host| host == "curseforge.com" || host.ends_with(".curseforge.com"))
    {
        request = request.header("x-api-key", CURSEFORGE_API_KEY);
    }
    let response = request.send().await.map_err(ContentError::Transport)?;
    if !response.status().is_success() {
        return Err(ContentError::HttpStatus(response.status()));
    }
    Ok(response
        .bytes()
        .await
        .map_err(ContentError::Response)?
        .to_vec())
}

pub(super) async fn prepare_modpack(
    client: &reqwest::Client,
    curseforge_base: &str,
    root: &Path,
    bytes: Vec<u8>,
    source: &str,
    filename: &str,
    name: &str,
) -> Result<PreparedModpack, ContentError> {
    let root = root.to_owned();
    let source = source.to_owned();
    let filename = filename.to_owned();
    let name = name.to_owned();
    let plan = tokio::task::spawn_blocking(move || plan(&root, bytes, &source, &filename, &name))
        .await
        .map_err(ContentError::Task)??;
    let result = async {
        match plan.downloads {
            Downloads::Modrinth(files) => {
                for file in files {
                    if file
                        .env
                        .as_ref()
                        .and_then(|env| env.get("client"))
                        .is_some_and(|value| value == "unsupported")
                    {
                        continue;
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
            }
            Downloads::CurseForge(files) => {
                for file in files {
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
                    if crate::app::safe_enclosed_filename(
                        plan.prepared.stage.path(),
                        &item.file_name,
                    )
                    .is_none()
                    {
                        return Err(ContentError::InvalidFilename(item.file_name));
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
