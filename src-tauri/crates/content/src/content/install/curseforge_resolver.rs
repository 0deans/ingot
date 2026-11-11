//! Shared CurseForge file metadata resolution, CDN URL fallback, and integrity translation.
use super::support::Integrity;
use crate::content::error::ContentError;
use ingot_artifacts::sources::CURSEFORGE_API_KEY;
use serde::Deserialize;
use std::collections::HashMap;

pub(super) const DEFAULT_CURSEFORGE_CDN_BASE: &str = "https://edge.forgecdn.net/files";

#[derive(Clone, Copy)]
pub(super) struct CurseForgeEndpoints<'a> {
    pub(super) api_base: &'a str,
    pub(super) cdn_base: &'a str,
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

pub(super) struct ResolvedCurseForgeFile {
    pub(super) file_name: String,
    pub(super) download_url: String,
    pub(super) integrity: Integrity,
}

pub(super) fn construct_curseforge_cdn_url_with_base(
    cdn_base: &str,
    file_id: u32,
    file_name: &str,
) -> String {
    let id_str = file_id.to_string();
    let (part1, part2) = if id_str.len() > 4 {
        let split = id_str.len() - 3;
        (&id_str[..split], &id_str[split..])
    } else {
        ("0", id_str.as_str())
    };
    format!(
        "{}/{part1}/{part2}/{file_name}",
        cdn_base.trim_end_matches('/')
    )
}

fn translate_curseforge_download(
    item: CurseForgeDownload,
    cdn_base: &str,
) -> Result<ResolvedCurseForgeFile, ContentError> {
    let download_url = item
        .download_url
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            construct_curseforge_cdn_url_with_base(cdn_base, item.id, &item.file_name)
        });
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
    Ok(ResolvedCurseForgeFile {
        file_name: item.file_name,
        download_url,
        integrity,
    })
}

pub(super) async fn resolve_curseforge_file_with_cdn(
    client: &reqwest::Client,
    endpoints: CurseForgeEndpoints<'_>,
    project_id: u32,
    file_id: u32,
) -> Result<ResolvedCurseForgeFile, ContentError> {
    let url = format!(
        "{}/mods/{}/files/{}",
        endpoints.api_base.trim_end_matches('/'),
        project_id,
        file_id
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
    translate_curseforge_download(item, endpoints.cdn_base)
}

#[cfg(test)]
pub(super) async fn resolve_curseforge_file(
    client: &reqwest::Client,
    curseforge_base: &str,
    project_id: u32,
    file_id: u32,
) -> Result<ResolvedCurseForgeFile, ContentError> {
    resolve_curseforge_file_with_cdn(
        client,
        CurseForgeEndpoints {
            api_base: curseforge_base,
            cdn_base: DEFAULT_CURSEFORGE_CDN_BASE,
        },
        project_id,
        file_id,
    )
    .await
}
