//! ATLauncher public API client.
use super::{
    error::ContentError,
    models::{
        ContentSearchQuery, ContentSearchResult, UnifiedContentDetails, UnifiedContentItem,
        UnifiedContentVersion,
    },
};
use reqwest::Client;
use serde::Deserialize;

pub const DEFAULT_ATLAUNCHER_BASE: &str = "https://download.nodecdn.net/containers/atl";

pub(crate) fn compute_atlauncher_safe_name(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum AtLauncherPackListResponse {
    Data { data: Vec<AtLauncherPackSummary> },
    List(Vec<AtLauncherPackSummary>),
}

impl AtLauncherPackListResponse {
    fn into_vec(self) -> Vec<AtLauncherPackSummary> {
        match self {
            Self::Data { data } => data,
            Self::List(list) => list,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AtLauncherPackSummary {
    #[serde(
        default,
        deserialize_with = "crate::content::ftb::deserialize_opt_u64_or_string"
    )]
    pub(crate) id: Option<u64>,
    pub(crate) position: Option<u64>,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) pack_type: Option<String>,
    #[serde(rename = "safeName")]
    pub(crate) safe_name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) system: Option<String>,
    pub(crate) versions: Option<Vec<AtLauncherVersionRef>>,
    pub(crate) downloads: Option<u64>,
    #[serde(rename = "supportURL")]
    pub(crate) support_url: Option<String>,
    #[serde(rename = "websiteURL")]
    pub(crate) website_url: Option<String>,
    #[serde(default)]
    pub(crate) categories: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AtLauncherVersionRef {
    pub(crate) version: String,
    pub(crate) minecraft: Option<String>,
    pub(crate) published: Option<u64>,
    #[serde(rename = "type")]
    pub(crate) version_type: Option<String>,
    #[serde(rename = "hasLoader")]
    pub(crate) has_loader: Option<bool>,
    #[serde(rename = "downloadUrl")]
    pub(crate) download_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum AtLauncherPackDetailResponse {
    Data { data: AtLauncherPackDetail },
    Direct(AtLauncherPackDetail),
}

impl AtLauncherPackDetailResponse {
    fn into_detail(self) -> AtLauncherPackDetail {
        match self {
            Self::Data { data } => data,
            Self::Direct(detail) => detail,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AtLauncherPackDetail {
    pub(crate) name: String,
    #[serde(rename = "safeName")]
    pub(crate) safe_name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) body: Option<String>,
    pub(crate) versions: Option<Vec<AtLauncherDetailVersion>>,
    pub(crate) downloads: Option<u64>,
    #[serde(default)]
    pub(crate) categories: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct AtLauncherDetailVersion {
    pub(crate) version: String,
    pub(crate) minecraft: Option<String>,
    pub(crate) loader: Option<String>,
    pub(crate) published: Option<u64>,
    #[serde(rename = "type")]
    pub(crate) version_type: Option<String>,
    #[serde(rename = "downloadUrl")]
    pub(crate) download_url: Option<String>,
}

#[derive(Clone)]
pub struct AtLauncherClient {
    client: Client,
    base_url: String,
}

impl AtLauncherClient {
    pub fn with_endpoint(client: Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub async fn search(
        &self,
        query: &ContentSearchQuery<'_>,
    ) -> Result<ContentSearchResult, ContentError> {
        let pt = query.project_type.trim().to_lowercase();
        if !matches!(pt.as_str(), "all" | "modpack" | "modpacks" | "") {
            return Ok(ContentSearchResult {
                items: Vec::new(),
                total_hits: 0,
                offset: query.offset,
                limit: query.limit,
            });
        }

        let packsnew_url = format!("{}/launcher/json/packsnew.json", self.base_url);
        let response = match self.client.get(&packsnew_url).send().await {
            Ok(res) if res.status().is_success() => res,
            _ => {
                let simple_url = format!("{}/packs/simple", self.base_url);
                match self.client.get(&simple_url).send().await {
                    Ok(res) if res.status().is_success() => res,
                    _ => {
                        let fallback_url = format!("{}/packs", self.base_url);
                        self.client
                            .get(&fallback_url)
                            .send()
                            .await
                            .map_err(ContentError::Transport)?
                    }
                }
            }
        };

        if !response.status().is_success() {
            return Err(ContentError::HttpStatus(response.status()));
        }

        let packs = response
            .json::<AtLauncherPackListResponse>()
            .await
            .map_err(ContentError::Response)?
            .into_vec();

        let mut filtered: Vec<AtLauncherPackSummary> = packs
            .into_iter()
            .filter(|pack| {
                if pack
                    .pack_type
                    .as_deref()
                    .is_some_and(|t| t.eq_ignore_ascii_case("private"))
                {
                    return false;
                }
                if pack.pack_type.is_some()
                    && pack.versions.as_ref().is_some_and(|vl| vl.is_empty())
                {
                    return false;
                }

                if let Some(q) = query.query.map(str::trim).filter(|s| !s.is_empty()) {
                    let q_lower = q.to_lowercase();
                    let name_matches = pack.name.to_lowercase().contains(&q_lower);
                    let desc_matches = pack
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q_lower);
                    if !name_matches && !desc_matches {
                        return false;
                    }
                }

                if let Some(gv) = query
                    .game_version
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "all")
                {
                    let has_mc = pack
                        .versions
                        .as_ref()
                        .map(|vl| {
                            vl.iter().any(|v| {
                                v.minecraft
                                    .as_deref()
                                    .unwrap_or("")
                                    .eq_ignore_ascii_case(gv)
                            })
                        })
                        .unwrap_or(false);
                    if !has_mc {
                        return false;
                    }
                }

                if let Some(ldr) = query
                    .loader
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "all")
                {
                    let has_ldr = pack.categories.iter().any(|c| c.eq_ignore_ascii_case(ldr))
                        || pack
                            .system
                            .as_deref()
                            .map(|s| s.to_lowercase().contains(&ldr.to_lowercase()))
                            .unwrap_or(false);
                    if !has_ldr && !ldr.eq_ignore_ascii_case("forge") {
                        return false;
                    }
                }

                true
            })
            .collect();

        match query.sort.unwrap_or("downloads") {
            "downloads" | "popularity" => {
                filtered.sort_by(|a, b| match (a.downloads, b.downloads) {
                    (Some(ad), Some(bd)) => bd.cmp(&ad),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => match (a.position, b.position) {
                        (Some(ap), Some(bp)) => ap.cmp(&bp),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name.cmp(&b.name),
                    },
                });
            }
            "updated" => {
                filtered.sort_by_key(|p| {
                    let latest_pub = p
                        .versions
                        .as_ref()
                        .and_then(|v| v.first())
                        .and_then(|v| v.published)
                        .unwrap_or(0);
                    std::cmp::Reverse(latest_pub)
                });
            }
            "newest" => {
                filtered.sort_by_key(|p| std::cmp::Reverse(p.id.unwrap_or(0)));
            }
            _ => {
                filtered.sort_by(|a, b| match (a.downloads, b.downloads) {
                    (Some(ad), Some(bd)) => bd.cmp(&ad),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => match (a.position, b.position) {
                        (Some(ap), Some(bp)) => ap.cmp(&bp),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name.cmp(&b.name),
                    },
                });
            }
        }

        let total_hits = filtered.len() as u64;
        let start = (query.offset as usize).min(filtered.len());
        let end = (start + query.limit as usize).min(filtered.len());
        let page_items = &filtered[start..end];

        let mut items = Vec::new();
        for pack in page_items {
            let safe_name = pack
                .safe_name
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| compute_atlauncher_safe_name(&pack.name));

            let mut game_versions = Vec::new();
            if let Some(vers) = &pack.versions {
                for v in vers {
                    if let Some(mc) = &v.minecraft {
                        if !game_versions.contains(mc) {
                            game_versions.push(mc.clone());
                        }
                    }
                }
            }

            let latest_version = pack
                .versions
                .as_ref()
                .and_then(|v| v.first())
                .map(|v| v.version.clone());

            let icon_url = Some(format!(
                "{}/launcher/images/{}.png",
                self.base_url.trim_end_matches('/'),
                safe_name.to_lowercase()
            ));

            let website_url = pack
                .website_url
                .clone()
                .unwrap_or_else(|| format!("https://atlauncher.com/pack/{}", safe_name));

            items.push(UnifiedContentItem {
                id: format!("atlauncher:{}", safe_name),
                source: "atlauncher".to_string(),
                project_type: "modpack".to_string(),
                slug: safe_name,
                title: pack.name.clone(),
                description: pack.description.clone().unwrap_or_default(),
                author: "ATLauncher".to_string(),
                icon_url,
                downloads: pack.downloads.unwrap_or(0),
                follows: None,
                categories: pack.categories.clone(),
                latest_version,
                game_versions,
                loaders: vec!["Forge".to_string()],
                website_url: Some(website_url),
                date_modified: None,
            });
        }

        Ok(ContentSearchResult {
            items,
            total_hits,
            offset: query.offset,
            limit: query.limit,
        })
    }

    pub async fn get_details(
        &self,
        project_id: &str,
    ) -> Result<UnifiedContentDetails, ContentError> {
        let clean_id = project_id
            .trim()
            .strip_prefix("atlauncher:")
            .or_else(|| project_id.trim().strip_prefix("atl:"))
            .unwrap_or(project_id)
            .trim();

        let detail_url = format!("{}/pack/{}", self.base_url, clean_id);
        if let Ok(response) = self.client.get(&detail_url).send().await {
            if response.status().is_success() {
                if let Ok(resp) = response.json::<AtLauncherPackDetailResponse>().await {
                    return Ok(self.map_pack_detail_response(resp.into_detail(), clean_id));
                }
            }
        }

        let packsnew_url = format!("{}/launcher/json/packsnew.json", self.base_url);
        let pn_res = self
            .client
            .get(&packsnew_url)
            .send()
            .await
            .map_err(ContentError::Transport)?;

        if !pn_res.status().is_success() {
            return Err(ContentError::HttpStatus(pn_res.status()));
        }

        let packs = pn_res
            .json::<AtLauncherPackListResponse>()
            .await
            .map_err(ContentError::Response)?
            .into_vec();

        let found = packs.into_iter().find(|p| {
            let computed = compute_atlauncher_safe_name(&p.name);
            let sname = p.safe_name.as_deref().unwrap_or(&computed);
            sname.eq_ignore_ascii_case(clean_id)
                || p.name.eq_ignore_ascii_case(clean_id)
                || p.id.map(|id| id.to_string()).as_deref() == Some(clean_id)
        });

        let Some(pack) = found else {
            return Err(ContentError::HttpStatus(reqwest::StatusCode::NOT_FOUND));
        };

        Ok(self.map_pack_summary_to_details(pack, clean_id))
    }

    fn map_pack_detail_response(
        &self,
        pack: AtLauncherPackDetail,
        clean_id: &str,
    ) -> UnifiedContentDetails {
        let safe_name = pack
            .safe_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(clean_id);

        let mut game_versions = Vec::new();
        let mut loaders = Vec::new();

        let base = self.base_url.trim_end_matches('/');
        let versions: Vec<UnifiedContentVersion> = pack
            .versions
            .unwrap_or_default()
            .into_iter()
            .map(|ver| {
                if let Some(mc) = &ver.minecraft {
                    if !game_versions.contains(mc) {
                        game_versions.push(mc.clone());
                    }
                }
                let ldr = ver.loader.unwrap_or_else(|| "Forge".to_string());
                if !loaders.contains(&ldr) {
                    loaders.push(ldr.clone());
                }

                let durl = ver
                    .download_url
                    .unwrap_or_else(|| format!("{base}/pack/{clean_id}/version/{}", ver.version));
                let fname = format!("atlauncher-{clean_id}-{}.json", ver.version);

                UnifiedContentVersion {
                    id: ver.version.clone(),
                    name: format!("{} {}", pack.name, ver.version),
                    version_number: ver.version,
                    game_versions: ver.minecraft.into_iter().collect(),
                    loaders: vec![ldr],
                    version_type: ver.version_type.unwrap_or_else(|| "release".into()),
                    date_published: ver.published.map(|p| p.to_string()).unwrap_or_default(),
                    downloads: 0,
                    filename: fname,
                    download_url: durl,
                    size_bytes: 0,
                }
            })
            .collect();

        if loaders.is_empty() {
            loaders.push("Forge".to_string());
        }

        let desc = pack.description.unwrap_or_default();
        let body = pack.body.unwrap_or_else(|| desc.clone());
        let icon_url = Some(format!("{base}/images/{safe_name}.png"));
        let website_url = format!("https://atlauncher.com/pack/{safe_name}");

        UnifiedContentDetails {
            id: format!("atlauncher:{safe_name}"),
            source: "atlauncher".to_string(),
            project_type: "modpack".to_string(),
            slug: safe_name.to_string(),
            title: pack.name,
            description: desc,
            body,
            author: "ATLauncher".to_string(),
            icon_url,
            downloads: pack.downloads.unwrap_or(0),
            follows: None,
            categories: pack.categories,
            game_versions,
            loaders,
            website_url: Some(website_url),
            issues_url: None,
            source_url: None,
            wiki_url: None,
            screenshots: Vec::new(),
            versions,
        }
    }

    fn map_pack_summary_to_details(
        &self,
        pack: AtLauncherPackSummary,
        _clean_id: &str,
    ) -> UnifiedContentDetails {
        let safe_name = pack
            .safe_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| compute_atlauncher_safe_name(&pack.name));

        let mut game_versions = Vec::new();
        let mut loaders = Vec::new();

        let base = self.base_url.trim_end_matches('/');
        let versions: Vec<UnifiedContentVersion> = pack
            .versions
            .unwrap_or_default()
            .into_iter()
            .map(|ver| {
                if let Some(mc) = &ver.minecraft {
                    if !game_versions.contains(mc) {
                        game_versions.push(mc.clone());
                    }
                }
                let ldr = if ver.has_loader.unwrap_or(true) {
                    "Forge".to_string()
                } else {
                    "Vanilla".to_string()
                };
                if !loaders.contains(&ldr) {
                    loaders.push(ldr.clone());
                }

                let durl = ver.download_url.unwrap_or_else(|| {
                    format!(
                        "{base}/packs/{safe_name}/versions/{}/Configs.json",
                        ver.version
                    )
                });
                let fname = format!("atlauncher-{safe_name}-{}.json", ver.version);

                UnifiedContentVersion {
                    id: ver.version.clone(),
                    name: format!("{} {}", pack.name, ver.version),
                    version_number: ver.version,
                    game_versions: ver.minecraft.into_iter().collect(),
                    loaders: vec![ldr],
                    version_type: ver.version_type.unwrap_or_else(|| "release".into()),
                    date_published: ver.published.map(|p| p.to_string()).unwrap_or_default(),
                    downloads: 0,
                    filename: fname,
                    download_url: durl,
                    size_bytes: 0,
                }
            })
            .collect();

        if loaders.is_empty() {
            loaders.push("Forge".to_string());
        }

        let desc = pack.description.unwrap_or_default();
        let body = desc.clone();
        let icon_url = Some(format!(
            "{base}/launcher/images/{}.png",
            safe_name.to_lowercase()
        ));
        let website_url = pack
            .website_url
            .unwrap_or_else(|| format!("https://atlauncher.com/pack/{safe_name}"));

        UnifiedContentDetails {
            id: format!("atlauncher:{safe_name}"),
            source: "atlauncher".to_string(),
            project_type: "modpack".to_string(),
            slug: safe_name,
            title: pack.name,
            description: desc,
            body,
            author: "ATLauncher".to_string(),
            icon_url,
            downloads: pack.downloads.unwrap_or(0),
            follows: None,
            categories: pack.categories,
            game_versions,
            loaders,
            website_url: Some(website_url),
            issues_url: pack.support_url,
            source_url: None,
            wiki_url: None,
            screenshots: Vec::new(),
            versions,
        }
    }
}
