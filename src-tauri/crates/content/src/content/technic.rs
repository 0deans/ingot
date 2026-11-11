//! Technic Platform and Solder API client.
use super::{
    error::ContentError,
    models::{
        ContentScreenshot, ContentSearchQuery, ContentSearchResult, UnifiedContentDetails,
        UnifiedContentItem, UnifiedContentVersion,
    },
};
use reqwest::Client;
use serde::Deserialize;

pub const DEFAULT_TECHNIC_BASE: &str = "https://api.technicpack.net";

/// Extracts the modpack slug segment from a Technic Platform URL, if present.
pub(crate) fn extract_technic_slug_from_query(q: &str) -> Option<&str> {
    let lower = q.to_lowercase();
    let marker = "technicpack.net/modpack/";
    let idx = lower.find(marker)?;
    let after = &q[idx + marker.len()..];
    let slug = after.split(['/', '?', '#', '&']).next()?.trim();
    if slug.is_empty() {
        return None;
    }
    if let Some((base, suffix)) = slug.rsplit_once('.') {
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return Some(base);
        }
    }
    Some(slug)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum TechnicImage {
    Obj { url: Option<String> },
    Str(String),
}

impl TechnicImage {
    fn to_url(&self) -> Option<String> {
        match self {
            Self::Obj { url } => url.clone(),
            Self::Str(s) => Some(s.clone()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TechnicSearchResponse {
    #[serde(default)]
    modpacks: Vec<TechnicSearchEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct TechnicSearchEntry {
    name: String,
    slug: Option<String>,
    #[serde(alias = "displayName")]
    display_name: Option<String>,
    user: Option<String>,
    description: Option<String>,
    #[serde(alias = "iconUrl")]
    icon_url: Option<String>,
    icon: Option<TechnicImage>,
    logo: Option<TechnicImage>,
    #[serde(alias = "installs")]
    downloads: Option<u64>,
    runs: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
struct TechnicPackDetail {
    name: String,
    #[serde(alias = "displayName")]
    display_name: Option<String>,
    #[serde(alias = "platformUrl")]
    platform_url: Option<String>,
    user: Option<String>,
    description: Option<String>,
    overview: Option<String>,
    url: Option<String>,
    solder: Option<String>,
    version: Option<String>,
    minecraft: Option<String>,
    icon: Option<TechnicImage>,
    logo: Option<TechnicImage>,
    background: Option<TechnicImage>,
    #[serde(alias = "installs")]
    downloads: Option<u64>,
    runs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SolderPackResponse {
    recommended: Option<String>,
    #[serde(default)]
    builds: Vec<String>,
}

#[derive(Clone)]
pub struct TechnicClient {
    client: Client,
    base_url: String,
}

impl TechnicClient {
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

        if let Some(slug) = query.query.and_then(extract_technic_slug_from_query) {
            match self.get_pack_item(slug).await {
                Ok(item) => {
                    return Ok(ContentSearchResult {
                        items: vec![item],
                        total_hits: 1,
                        offset: 0,
                        limit: query.limit,
                    });
                }
                Err(err) => return Err(err),
            }
        }

        let request = if let Some(q) = query.query.map(str::trim).filter(|s| !s.is_empty()) {
            let url = format!("{}/search", self.base_url);
            self.client.get(url).query(&[("build", "ingot"), ("q", q)])
        } else {
            let url = format!("{}/trending", self.base_url);
            self.client.get(url).query(&[("build", "ingot")])
        };

        let response = request.send().await.map_err(ContentError::Transport)?;
        if !response.status().is_success() {
            return Err(ContentError::HttpStatus(response.status()));
        }

        let parsed = response
            .json::<TechnicSearchResponse>()
            .await
            .map_err(ContentError::Response)?;

        let entries = parsed.modpacks;
        let total_hits = entries.len() as u64;
        let start = (query.offset as usize).min(entries.len());
        let end = (start + query.limit as usize).min(entries.len());
        let slice = &entries[start..end];

        let mut set = tokio::task::JoinSet::new();
        for (idx, entry) in slice.iter().enumerate() {
            let real_slug = entry
                .slug
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&entry.name)
                .to_string();
            let entry_clone = entry.clone();
            let base_url = self.base_url.clone();
            let client = self.client.clone();
            set.spawn(async move {
                if entry_clone.description.is_none() {
                    let url = format!("{}/modpack/{}?build=ingot", base_url, real_slug);
                    if let Ok(res) = client.get(url).send().await {
                        if res.status().is_success() {
                            if let Ok(detail) = res.json::<TechnicPackDetail>().await {
                                return (idx, entry_clone, real_slug, Some(detail));
                            }
                        }
                    }
                }
                (idx, entry_clone, real_slug, None)
            });
        }

        let mut hydrated: Vec<Option<(TechnicSearchEntry, String, Option<TechnicPackDetail>)>> =
            (0..slice.len()).map(|_| None).collect();
        while let Some(res) = set.join_next().await {
            if let Ok((idx, entry, real_slug, detail_opt)) = res {
                if idx < hydrated.len() {
                    hydrated[idx] = Some((entry, real_slug, detail_opt));
                }
            }
        }

        let mut items = Vec::new();
        for (entry, real_slug, detail_opt) in hydrated.into_iter().flatten() {
            if let Some(detail) = detail_opt {
                let icon_url = detail
                    .icon
                    .as_ref()
                    .and_then(|i| i.to_url())
                    .or_else(|| detail.logo.as_ref().and_then(|l| l.to_url()))
                    .or_else(|| entry.icon_url.clone())
                    .or_else(|| entry.icon.as_ref().and_then(|i| i.to_url()))
                    .or_else(|| entry.logo.as_ref().and_then(|l| l.to_url()));

                let title = detail.display_name.unwrap_or_else(|| detail.name.clone());
                let website_url = detail.platform_url.unwrap_or_else(|| {
                    format!("https://www.technicpack.net/modpack/{}", real_slug)
                });

                items.push(UnifiedContentItem {
                    id: format!("technic:{}", real_slug),
                    source: "technic".to_string(),
                    project_type: "modpack".to_string(),
                    slug: real_slug,
                    title,
                    description: detail.description.unwrap_or_default(),
                    author: detail.user.unwrap_or_else(|| "Technic".to_string()),
                    icon_url,
                    downloads: detail.downloads.unwrap_or(0),
                    follows: detail.runs,
                    categories: Vec::new(),
                    latest_version: detail.version,
                    game_versions: detail.minecraft.into_iter().collect(),
                    loaders: vec!["Forge".to_string()],
                    website_url: Some(website_url),
                    date_modified: None,
                });
            } else {
                let icon_url = entry
                    .icon_url
                    .clone()
                    .or_else(|| entry.icon.as_ref().and_then(|i| i.to_url()))
                    .or_else(|| entry.logo.as_ref().and_then(|l| l.to_url()));

                let title = entry
                    .display_name
                    .clone()
                    .unwrap_or_else(|| entry.name.clone());
                let website_url = format!("https://www.technicpack.net/modpack/{}", real_slug);

                items.push(UnifiedContentItem {
                    id: format!("technic:{}", real_slug),
                    source: "technic".to_string(),
                    project_type: "modpack".to_string(),
                    slug: real_slug,
                    title,
                    description: entry.description.clone().unwrap_or_default(),
                    author: entry.user.clone().unwrap_or_else(|| "Technic".to_string()),
                    icon_url,
                    downloads: entry.downloads.unwrap_or(0),
                    follows: entry.runs,
                    categories: Vec::new(),
                    latest_version: None,
                    game_versions: Vec::new(),
                    loaders: vec!["Forge".to_string()],
                    website_url: Some(website_url),
                    date_modified: None,
                });
            }
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
        let clean_slug = extract_technic_slug_from_query(project_id).unwrap_or_else(|| {
            project_id
                .trim()
                .strip_prefix("technic:")
                .unwrap_or(project_id)
                .trim()
        });

        let url = format!("{}/modpack/{}?build=ingot", self.base_url, clean_slug);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(ContentError::Transport)?;

        if !response.status().is_success() {
            return Err(ContentError::HttpStatus(response.status()));
        }

        let pack: TechnicPackDetail = response.json().await.map_err(ContentError::Response)?;
        let mut versions = Vec::new();

        if let Some(solder_url) = pack.solder.as_deref().filter(|s| !s.trim().is_empty()) {
            let solder_endpoint =
                format!("{}/modpack/{}", solder_url.trim_end_matches('/'), pack.name);
            if let Ok(solder_res) = self.client.get(solder_endpoint).send().await {
                if solder_res.status().is_success() {
                    if let Ok(solder_data) = solder_res.json::<SolderPackResponse>().await {
                        for build in solder_data.builds {
                            let is_recommended = solder_data.recommended.as_deref() == Some(&build);
                            let vtype = if is_recommended { "release" } else { "beta" };
                            let durl = format!(
                                "{}/modpack/{}/{}?include=mods",
                                solder_url.trim_end_matches('/'),
                                pack.name,
                                build
                            );
                            let fname = format!("{}-{}-solder.json", pack.name, build);

                            versions.push(UnifiedContentVersion {
                                id: build.clone(),
                                name: format!(
                                    "{} {}",
                                    pack.display_name.as_deref().unwrap_or(&pack.name),
                                    build
                                ),
                                version_number: build,
                                game_versions: pack.minecraft.clone().into_iter().collect(),
                                loaders: vec!["Forge".to_string()],
                                version_type: vtype.to_string(),
                                date_published: String::new(),
                                downloads: 0,
                                filename: fname,
                                download_url: durl,
                                size_bytes: 0,
                            });
                        }
                    }
                }
            }
        }

        if let Some(direct_url) = pack.url.as_deref().filter(|s| !s.trim().is_empty()) {
            let ver = pack.version.clone().unwrap_or_else(|| "1.0.0".to_string());
            let name = format!(
                "{} {}",
                pack.display_name.as_deref().unwrap_or(&pack.name),
                ver
            );
            let mc_part = pack
                .minecraft
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|m| format!("-mc{}", m))
                .unwrap_or_default();
            let fname = format!("{}-{}{}.zip", pack.name, ver, mc_part);
            versions.push(UnifiedContentVersion {
                id: ver.clone(),
                name,
                version_number: ver,
                game_versions: pack.minecraft.clone().into_iter().collect(),
                loaders: vec!["Forge".to_string()],
                version_type: "release".to_string(),
                date_published: String::new(),
                downloads: 0,
                filename: fname,
                download_url: direct_url.to_string(),
                size_bytes: 0,
            });
        }

        if versions.is_empty() {
            return Err(ContentError::MissingField(
                "modpack download URL or Solder build",
            ));
        }

        let icon_url = pack
            .icon
            .as_ref()
            .and_then(|i| i.to_url())
            .or_else(|| pack.logo.as_ref().and_then(|l| l.to_url()));

        let screenshots = pack
            .background
            .as_ref()
            .and_then(|b| b.to_url())
            .map(|url| vec![ContentScreenshot { url, title: None }])
            .unwrap_or_default();

        let desc = pack.description.unwrap_or_default();
        let body = pack.overview.unwrap_or_else(|| desc.clone());
        let title = pack.display_name.unwrap_or_else(|| pack.name.clone());

        Ok(UnifiedContentDetails {
            id: format!("technic:{}", pack.name),
            source: "technic".to_string(),
            project_type: "modpack".to_string(),
            slug: pack.name.clone(),
            title,
            description: desc,
            body,
            author: pack.user.unwrap_or_else(|| "Technic".to_string()),
            icon_url,
            downloads: pack.downloads.unwrap_or(0),
            follows: pack.runs,
            categories: Vec::new(),
            game_versions: pack.minecraft.into_iter().collect(),
            loaders: vec!["Forge".to_string()],
            website_url: Some(format!("https://www.technicpack.net/modpack/{}", pack.name)),
            issues_url: None,
            source_url: None,
            wiki_url: None,
            screenshots,
            versions,
        })
    }

    async fn get_pack_item(&self, slug: &str) -> Result<UnifiedContentItem, ContentError> {
        let details = self.get_details(slug).await?;
        Ok(UnifiedContentItem {
            id: details.id,
            source: details.source,
            project_type: details.project_type,
            slug: details.slug,
            title: details.title,
            description: details.description,
            author: details.author,
            icon_url: details.icon_url,
            downloads: details.downloads,
            follows: details.follows,
            categories: details.categories,
            latest_version: details.versions.first().map(|v| v.version_number.clone()),
            game_versions: details.game_versions,
            loaders: details.loaders,
            website_url: details.website_url,
            date_modified: None,
        })
    }
}
