//! FTB (Feed The Beast) public API client.
use super::{
    error::ContentError,
    models::{
        ContentScreenshot, ContentSearchQuery, ContentSearchResult, UnifiedContentDetails,
        UnifiedContentItem, UnifiedContentVersion,
    },
};
use reqwest::Client;
use serde::{Deserialize, Deserializer};

pub const DEFAULT_FTB_BASE: &str = "https://api.modpacks.ch/public";

pub(crate) fn deserialize_u32_or_string<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrString {
        Int(u64),
        String(String),
    }

    match IntOrString::deserialize(deserializer)? {
        IntOrString::Int(n) => u32::try_from(n).map_err(serde::de::Error::custom),
        IntOrString::String(s) => s.trim().parse::<u32>().map_err(serde::de::Error::custom),
    }
}

pub(crate) fn deserialize_opt_u32_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrString {
        Int(u64),
        String(String),
    }

    let opt = Option::<IntOrString>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(IntOrString::Int(n)) => u32::try_from(n).map(Some).map_err(serde::de::Error::custom),
        Some(IntOrString::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<u32>()
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

pub(crate) fn deserialize_u64_or_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrString {
        Int(u64),
        String(String),
    }

    match IntOrString::deserialize(deserializer)? {
        IntOrString::Int(n) => Ok(n),
        IntOrString::String(s) => s.trim().parse::<u64>().map_err(serde::de::Error::custom),
    }
}

pub(crate) fn deserialize_opt_u64_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IntOrString {
        Int(u64),
        String(String),
    }

    let opt = Option::<IntOrString>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(IntOrString::Int(n)) => Ok(Some(n)),
        Some(IntOrString::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<u64>()
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
struct FtbPackId(#[serde(deserialize_with = "deserialize_u64_or_string")] u64);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum FtbPackListResponse {
    Object {
        #[serde(default)]
        packs: Option<Vec<FtbPackId>>,
        total: Option<u64>,
    },
    Array(Vec<FtbPackId>),
}

impl FtbPackListResponse {
    fn into_ids(self) -> Vec<u64> {
        match self {
            Self::Object { packs, .. } => {
                packs.unwrap_or_default().into_iter().map(|p| p.0).collect()
            }
            Self::Array(list) => list.into_iter().map(|p| p.0).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct FtbArtItem {
    #[serde(default)]
    r#type: String,
    url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum FtbAuthorItem {
    Obj { name: String },
    Str(String),
}

impl FtbAuthorItem {
    fn name(&self) -> &str {
        match self {
            Self::Obj { name } => name.as_str(),
            Self::Str(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum FtbTagItem {
    Obj { name: String },
    Str(String),
}

impl FtbTagItem {
    fn name(&self) -> &str {
        match self {
            Self::Obj { name } => name.as_str(),
            Self::Str(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct FtbTarget {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) r#type: String,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct FtbVersionSummary {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    id: u64,
    name: String,
    r#type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    updated: Option<u64>,
    #[serde(default)]
    targets: Vec<FtbTarget>,
}

#[derive(Debug, Deserialize, Clone)]
struct FtbPackDetail {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    id: u64,
    name: String,
    slug: Option<String>,
    synopsis: Option<String>,
    description: Option<String>,
    #[serde(default)]
    art: Vec<FtbArtItem>,
    #[serde(default)]
    authors: Vec<FtbAuthorItem>,
    #[serde(default)]
    tags: Vec<FtbTagItem>,
    #[serde(default)]
    targets: Vec<FtbTarget>,
    #[serde(default)]
    versions: Vec<FtbVersionSummary>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    installs: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
    plays: Option<u64>,
}

#[derive(Clone)]
pub struct FtbClient {
    client: Client,
    base_url: String,
}

impl FtbClient {
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

        let request = if let Some(q) = query.query.map(str::trim).filter(|s| !s.is_empty()) {
            let url = format!("{}/modpack/search/50", self.base_url);
            self.client.get(url).query(&[("term", q)])
        } else if query.sort == Some("updated") {
            let url = format!("{}/modpack/updated/50", self.base_url);
            self.client.get(url)
        } else {
            let url = format!("{}/modpack/popular/installs/50", self.base_url);
            self.client.get(url)
        };

        let response = request.send().await.map_err(ContentError::Transport)?;
        if !response.status().is_success() {
            return Err(ContentError::HttpStatus(response.status()));
        }

        let pack_ids = response
            .json::<FtbPackListResponse>()
            .await
            .map_err(ContentError::Response)?
            .into_ids();

        let total_hits = pack_ids.len() as u64;
        let start = (query.offset as usize).min(pack_ids.len());
        let end = (start + query.limit as usize).min(pack_ids.len());
        let slice = &pack_ids[start..end];

        let mut set = tokio::task::JoinSet::new();
        for (idx, &pack_id) in slice.iter().enumerate() {
            let base_url = self.base_url.clone();
            let client = self.client.clone();
            set.spawn(async move {
                let detail_url = format!("{}/modpack/{}", base_url, pack_id);
                let res = client.get(detail_url).send().await.ok()?;
                if !res.status().is_success() {
                    return None;
                }
                let pack = res.json::<FtbPackDetail>().await.ok()?;
                Some((idx, pack))
            });
        }

        let mut details: Vec<Option<FtbPackDetail>> = (0..slice.len()).map(|_| None).collect();
        while let Some(res) = set.join_next().await {
            if let Ok(Some((idx, pack))) = res {
                if idx < details.len() {
                    details[idx] = Some(pack);
                }
            }
        }

        let mut items = Vec::new();
        for pack in details.into_iter().flatten() {
            let item = self.map_pack_item(pack);

            if let Some(gv) = query
                .game_version
                .map(str::trim)
                .filter(|s| !s.is_empty() && *s != "all")
            {
                if !item
                    .game_versions
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(gv))
                {
                    continue;
                }
            }

            if let Some(ldr) = query
                .loader
                .map(str::trim)
                .filter(|s| !s.is_empty() && *s != "all")
            {
                if !item.loaders.iter().any(|l| l.eq_ignore_ascii_case(ldr)) {
                    continue;
                }
            }

            items.push(item);
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
            .strip_prefix("ftb:")
            .unwrap_or(project_id)
            .trim();
        let detail_url = format!("{}/modpack/{}", self.base_url, clean_id);
        let response = self
            .client
            .get(detail_url)
            .send()
            .await
            .map_err(ContentError::Transport)?;

        if !response.status().is_success() {
            return Err(ContentError::HttpStatus(response.status()));
        }

        let pack: FtbPackDetail = response.json().await.map_err(ContentError::Response)?;
        Ok(self.map_pack_details(pack))
    }

    fn map_pack_item(&self, pack: FtbPackDetail) -> UnifiedContentItem {
        let icon_url = pack
            .art
            .iter()
            .find(|a| a.r#type == "square")
            .or_else(|| pack.art.first())
            .map(|a| a.url.clone());

        let author = pack
            .authors
            .first()
            .map(|a| a.name().to_string())
            .unwrap_or_else(|| "FTB Team".to_string());

        let categories: Vec<String> = pack.tags.iter().map(|t| t.name().to_string()).collect();
        let (game_versions, loaders) = extract_targets(&pack.targets, &pack.versions);
        let latest_version = pack.versions.last().map(|v| v.name.clone());

        let slug = pack
            .slug
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| pack.id.to_string());

        let website_url = format!("https://www.feed-the-beast.com/modpacks/{}", pack.id);

        UnifiedContentItem {
            id: format!("ftb:{}", pack.id),
            source: "ftb".to_string(),
            project_type: "modpack".to_string(),
            slug,
            title: pack.name,
            description: pack.synopsis.unwrap_or_default(),
            author,
            icon_url,
            downloads: pack.installs.unwrap_or(0),
            follows: pack.plays,
            categories,
            latest_version,
            game_versions,
            loaders,
            website_url: Some(website_url),
            date_modified: None,
        }
    }

    fn map_pack_details(&self, pack: FtbPackDetail) -> UnifiedContentDetails {
        let (pack_game_versions, pack_loaders) = extract_targets(&pack.targets, &pack.versions);
        let icon_url = pack
            .art
            .iter()
            .find(|a| a.r#type == "square")
            .or_else(|| pack.art.first())
            .map(|a| a.url.clone());

        let author = pack
            .authors
            .first()
            .map(|a| a.name().to_string())
            .unwrap_or_else(|| "FTB Team".to_string());

        let screenshots: Vec<ContentScreenshot> = pack
            .art
            .iter()
            .filter(|a| a.r#type == "screenshot" || a.r#type == "gallery")
            .map(|a| ContentScreenshot {
                url: a.url.clone(),
                title: None,
            })
            .collect();

        let base = self.base_url.trim_end_matches('/');
        let pack_id = pack.id;
        let versions: Vec<UnifiedContentVersion> = pack
            .versions
            .into_iter()
            .rev()
            .map(|ver| {
                let (ver_games, ver_loaders) = extract_targets(&ver.targets, &[]);
                let game_versions = if ver_games.is_empty() {
                    pack_game_versions.clone()
                } else {
                    ver_games
                };
                let loaders = if ver_loaders.is_empty() {
                    pack_loaders.clone()
                } else {
                    ver_loaders
                };
                UnifiedContentVersion {
                    id: ver.id.to_string(),
                    name: ver.name.clone(),
                    version_number: ver.name,
                    game_versions,
                    loaders,
                    version_type: ver.r#type.unwrap_or_else(|| "release".into()),
                    date_published: ver.updated.map(|u| u.to_string()).unwrap_or_default(),
                    downloads: 0,
                    filename: format!("ftb-{}-{}.json", pack_id, ver.id),
                    download_url: format!("{base}/modpack/{pack_id}/{}", ver.id),
                    size_bytes: 0,
                }
            })
            .collect();

        let slug = pack
            .slug
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| pack.id.to_string());

        let synopsis = pack.synopsis.unwrap_or_default();
        let body = pack.description.unwrap_or_else(|| synopsis.clone());
        let categories = pack.tags.iter().map(|t| t.name().to_string()).collect();

        UnifiedContentDetails {
            id: format!("ftb:{}", pack.id),
            source: "ftb".to_string(),
            project_type: "modpack".to_string(),
            slug,
            title: pack.name,
            description: synopsis,
            body,
            author,
            icon_url,
            downloads: pack.installs.unwrap_or(0),
            follows: pack.plays,
            categories,
            game_versions: pack_game_versions,
            loaders: pack_loaders,
            website_url: Some(format!(
                "https://www.feed-the-beast.com/modpacks/{}",
                pack.id
            )),
            issues_url: None,
            source_url: None,
            wiki_url: None,
            screenshots,
            versions,
        }
    }
}

fn extract_targets(
    targets: &[FtbTarget],
    versions: &[FtbVersionSummary],
) -> (Vec<String>, Vec<String>) {
    let mut game_versions = Vec::new();
    let mut loaders = Vec::new();

    for t in targets {
        let t_type = t.r#type.to_lowercase();
        let t_name = t.name.to_lowercase();
        if t_type == "game" || t_name == "minecraft" {
            if let Some(v) = &t.version {
                if !game_versions.contains(v) {
                    game_versions.push(v.clone());
                }
            }
        } else if t_type == "modloader" {
            let ldr_name = match t_name.as_str() {
                "forge" => "Forge",
                "neoforge" => "NeoForge",
                "fabric" => "Fabric",
                "quilt" => "Quilt",
                _ => t.name.as_str(),
            }
            .to_string();
            if !loaders.contains(&ldr_name) {
                loaders.push(ldr_name);
            }
        }
    }

    if game_versions.is_empty() || loaders.is_empty() {
        for v in versions {
            let (vg, vl) = extract_targets(&v.targets, &[]);
            for g in vg {
                if !game_versions.contains(&g) {
                    game_versions.push(g);
                }
            }
            for l in vl {
                if !loaders.contains(&l) {
                    loaders.push(l);
                }
            }
        }
    }

    (game_versions, loaders)
}
