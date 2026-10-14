//! CurseForge API client for search and mod/addon details retrieval.
use super::{
    error::ContentError,
    models::{
        ContentScreenshot, ContentSearchQuery, ContentSearchResult, UnifiedContentDetails,
        UnifiedContentItem, UnifiedContentVersion,
    },
};
use reqwest::Client;
use serde::Deserialize;

pub const CURSEFORGE_API_KEY: &str = "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm";
pub const DEFAULT_CURSEFORGE_BASE: &str = "https://api.curseforge.com/v1";

#[derive(Debug, Deserialize)]
struct CurseForgeSearchResponse {
    data: Vec<CurseForgeModItem>,
    pagination: CurseForgePagination,
}

#[derive(Debug, Deserialize)]
struct CurseForgePagination {
    #[serde(rename = "totalCount")]
    total_count: u64,
    #[allow(dead_code)]
    index: u32,
    #[allow(dead_code)]
    #[serde(rename = "pageSize")]
    page_size: u32,
}

#[derive(Debug, Deserialize)]
struct CurseForgeModItem {
    id: u64,
    name: String,
    slug: String,
    summary: String,
    #[serde(rename = "downloadCount")]
    download_count: f64,
    authors: Option<Vec<CurseForgeAuthor>>,
    logo: Option<CurseForgeLogo>,
    categories: Option<Vec<CurseForgeCategory>>,
    #[serde(rename = "latestFiles")]
    latest_files: Option<Vec<CurseForgeFile>>,
    links: Option<CurseForgeLinks>,
    #[serde(rename = "dateModified")]
    date_modified: Option<String>,
    #[serde(rename = "classId")]
    class_id: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CurseForgeLogo {
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeCategory {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CurseForgeFile {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "gameVersions")]
    game_versions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeLinks {
    #[serde(rename = "websiteUrl")]
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeModDetailResponse {
    data: CurseForgeModDetail,
}

#[derive(Debug, Deserialize)]
struct CurseForgeModDetail {
    id: u32,
    name: String,
    slug: String,
    summary: String,
    links: Option<CurseForgeDetailLinks>,
    authors: Option<Vec<CurseForgeAuthor>>,
    logo: Option<CurseForgeLogo>,
    screenshots: Option<Vec<CurseForgeScreenshotItem>>,
    #[serde(rename = "classId")]
    class_id: Option<u32>,
    #[serde(rename = "downloadCount")]
    download_count: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeDetailLinks {
    website_url: Option<String>,
    wiki_url: Option<String>,
    issues_url: Option<String>,
    source_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeScreenshotItem {
    url: String,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeDescriptionResponse {
    data: String,
}

#[derive(Debug, Deserialize)]
struct CurseForgeFilesResponse {
    data: Vec<CurseForgeFileDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFileDetail {
    pub id: u32,
    pub display_name: String,
    pub file_name: String,
    pub release_type: Option<u32>,
    pub file_date: Option<String>,
    pub file_length: Option<u64>,
    pub download_count: Option<u64>,
    pub download_url: Option<String>,
    #[allow(dead_code)]
    pub game_versions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeSingleFileResponse {
    data: CurseForgeFileDetail,
}

pub struct CurseForgeClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl Default for CurseForgeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CurseForgeClient {
    pub fn new() -> Self {
        Self::with_endpoint(
            Client::builder()
                .user_agent(crate::USER_AGENT)
                .build()
                .unwrap_or_default(),
            DEFAULT_CURSEFORGE_BASE,
            CURSEFORGE_API_KEY,
        )
    }

    pub fn with_endpoint(
        client: Client,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            api_key: api_key.into(),
        }
    }

    pub async fn search(
        &self,
        query: &ContentSearchQuery<'_>,
    ) -> Result<ContentSearchResult, ContentError> {
        let is_bedrock = query
            .loader
            .is_some_and(|l| l.eq_ignore_ascii_case("bedrock"))
            || query.project_type.eq_ignore_ascii_case("bedrock")
            || query.project_type.eq_ignore_ascii_case("addon")
            || query.project_type.eq_ignore_ascii_case("addons")
            || query.project_type.eq_ignore_ascii_case("behaviorpack")
            || query.project_type.eq_ignore_ascii_case("behaviorpacks");

        let game_id = if is_bedrock { "78022" } else { "432" };

        let class_id = if is_bedrock {
            match query.project_type.to_lowercase().as_str() {
                "addon" | "addons" | "behaviorpack" | "behaviorpacks" => Some(4984),
                "resourcepack" | "resourcepacks" | "texturepack" | "texturepacks" => Some(6929),
                "map" | "maps" => Some(6913),
                _ => None,
            }
        } else {
            match query.project_type.to_lowercase().as_str() {
                "mod" | "mods" => Some(6),
                "modpack" | "modpacks" => Some(4471),
                "resourcepack" | "resourcepacks" => Some(12),
                "shader" | "shaders" | "shaderpack" | "shaderpacks" => Some(6552),
                _ => None,
            }
        };

        let mod_loader_type = if is_bedrock {
            None
        } else {
            match query.loader.unwrap_or("").to_lowercase().as_str() {
                "forge" => Some(1),
                "fabric" => Some(4),
                "quilt" => Some(5),
                "neoforge" => Some(6),
                _ => None,
            }
        };

        let sort_field = match query.sort.unwrap_or("downloads") {
            "downloads" | "popularity" => 5, // TotalDownloads
            "updated" | "recent" => 3,       // LastUpdated
            "newest" => 1,                   // Featured / New
            "relevance" => 2,                // Popularity/Score
            _ => 5,
        };

        let search_url = format!("{}/mods/search", self.base_url);
        let mut req = self
            .client
            .get(&search_url)
            .header("x-api-key", &self.api_key)
            .header("Accept", "application/json")
            .query(&[
                ("gameId", game_id.to_string()),
                ("index", query.offset.to_string()),
                ("pageSize", query.limit.to_string()),
                ("sortField", sort_field.to_string()),
                ("sortOrder", "desc".to_string()),
            ]);

        if let Some(cid) = class_id {
            req = req.query(&[("classId", cid.to_string())]);
        }

        if let Some(q) = query.query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                req = req.query(&[("searchFilter", trimmed.to_string())]);
            }
        }

        if let Some(gv) = query.game_version {
            let trimmed = gv.trim();
            if !trimmed.is_empty() && trimmed != "all" {
                req = req.query(&[("gameVersion", trimmed.to_string())]);
            }
        }

        if let Some(mlt) = mod_loader_type {
            req = req.query(&[("modLoaderType", mlt.to_string())]);
        }

        let res = req.send().await.map_err(ContentError::Transport)?;
        if !res.status().is_success() {
            return Err(ContentError::HttpStatus(res.status()));
        }

        let parsed = res
            .json::<CurseForgeSearchResponse>()
            .await
            .map_err(ContentError::Response)?;

        let items = parsed
            .data
            .into_iter()
            .map(|item| {
                let author = item
                    .authors
                    .and_then(|a| a.into_iter().next().map(|x| x.name))
                    .unwrap_or_else(|| "Unknown".to_string());

                let icon_url = item.logo.and_then(|l| l.thumbnail_url.or(l.url));

                let categories = item
                    .categories
                    .map(|cats| cats.into_iter().map(|c| c.name).collect())
                    .unwrap_or_default();

                let mut game_versions = Vec::new();
                let mut loaders = Vec::new();
                let mut latest_version = None;

                if let Some(files) = item.latest_files {
                    if let Some(first_file) = files.first() {
                        latest_version = first_file.display_name.clone();
                    }
                    for f in files {
                        if let Some(gvs) = f.game_versions {
                            for v in gvs {
                                let vl = v.to_lowercase();
                                if vl == "fabric"
                                    || vl == "forge"
                                    || vl == "neoforge"
                                    || vl == "quilt"
                                {
                                    if !loaders.contains(&v) {
                                        loaders.push(v);
                                    }
                                } else if !v.eq_ignore_ascii_case("client")
                                    && !v.eq_ignore_ascii_case("server")
                                {
                                    if !game_versions.contains(&v) {
                                        game_versions.push(v);
                                    }
                                }
                            }
                        }
                    }
                }

                let project_type_clean = if is_bedrock {
                    match item.class_id {
                        Some(4984) => "addon",
                        Some(6929) => "resourcepack",
                        Some(6913) => "map",
                        _ => "addon",
                    }
                } else {
                    match item.class_id {
                        Some(4471) => "modpack",
                        Some(12) => "resourcepack",
                        Some(6552) => "shader",
                        _ => "mod",
                    }
                }
                .to_string();

                let website_url = item.links.and_then(|l| l.website_url).unwrap_or_else(|| {
                    if is_bedrock {
                        format!(
                            "https://www.curseforge.com/minecraft-bedrock/addons/{}",
                            item.slug
                        )
                    } else {
                        format!("https://www.curseforge.com/minecraft/mc-mods/{}", item.slug)
                    }
                });

                UnifiedContentItem {
                    id: format!("cf:{}", item.id),
                    source: "curseforge".to_string(),
                    project_type: project_type_clean,
                    slug: item.slug,
                    title: item.name,
                    description: item.summary,
                    author,
                    icon_url,
                    downloads: item.download_count as u64,
                    follows: None,
                    categories,
                    latest_version,
                    game_versions,
                    loaders,
                    website_url: Some(website_url),
                    date_modified: item.date_modified,
                }
            })
            .collect();

        Ok(ContentSearchResult {
            items,
            total_hits: parsed.pagination.total_count,
            offset: query.offset,
            limit: query.limit,
        })
    }

    pub async fn get_details(
        &self,
        project_id: &str,
    ) -> Result<UnifiedContentDetails, ContentError> {
        let mod_id: u32 = match project_id.parse::<u32>() {
            Ok(id) => id,
            Err(_) => {
                let search_url = format!(
                    "{}/mods/search?gameId=432&slug={}",
                    self.base_url, project_id
                );
                let resp = self
                    .client
                    .get(&search_url)
                    .header("x-api-key", &self.api_key)
                    .send()
                    .await
                    .map_err(ContentError::Transport)?;

                let found = if resp.status().is_success() {
                    let parsed: Result<CurseForgeSearchResponse, _> = resp.json().await;
                    parsed
                        .ok()
                        .and_then(|p| p.data.first().map(|m| m.id as u32))
                } else {
                    None
                };

                if let Some(id) = found {
                    id
                } else {
                    let b_search_url = format!(
                        "{}/mods/search?gameId=78022&slug={}",
                        self.base_url, project_id
                    );
                    let b_resp = self
                        .client
                        .get(&b_search_url)
                        .header("x-api-key", &self.api_key)
                        .send()
                        .await
                        .map_err(ContentError::Transport)?;

                    if b_resp.status().is_success() {
                        let b_parsed: CurseForgeSearchResponse =
                            b_resp.json().await.map_err(ContentError::Response)?;
                        b_parsed.data.first().map(|m| m.id as u32).ok_or_else(|| {
                            ContentError::UnknownSource(format!(
                                "CurseForge mod '{project_id}' not found"
                            ))
                        })?
                    } else {
                        return Err(ContentError::UnknownSource(format!(
                            "Invalid CurseForge mod ID or slug: {project_id}"
                        )));
                    }
                }
            }
        };

        let mod_url = format!("{}/mods/{}", self.base_url, mod_id);
        let desc_url = format!("{}/mods/{}/description", self.base_url, mod_id);
        let files_url = format!("{}/mods/{}/files?pageSize=50", self.base_url, mod_id);

        let (mod_res, desc_res, files_res) = tokio::join!(
            self.client
                .get(&mod_url)
                .header("x-api-key", &self.api_key)
                .send(),
            self.client
                .get(&desc_url)
                .header("x-api-key", &self.api_key)
                .send(),
            self.client
                .get(&files_url)
                .header("x-api-key", &self.api_key)
                .send(),
        );

        let mod_resp = mod_res.map_err(ContentError::Transport)?;
        if !mod_resp.status().is_success() {
            return Err(ContentError::HttpStatus(mod_resp.status()));
        }
        let mod_data: CurseForgeModDetailResponse =
            mod_resp.json().await.map_err(ContentError::Response)?;
        let m = mod_data.data;

        let body = match desc_res {
            Ok(r) if r.status().is_success() => r
                .json::<CurseForgeDescriptionResponse>()
                .await
                .map(|d| d.data)
                .unwrap_or_default(),
            _ => m.summary.clone(),
        };

        let mut versions = Vec::new();
        if let Ok(f_resp) = files_res {
            if f_resp.status().is_success() {
                if let Ok(f_data) = f_resp.json::<CurseForgeFilesResponse>().await {
                    for f in f_data.data {
                        let mut game_versions = Vec::new();
                        let mut loaders = Vec::new();
                        if let Some(ref gvs) = f.game_versions {
                            for gv in gvs {
                                let gv_lower = gv.to_lowercase();
                                if gv_lower == "fabric"
                                    || gv_lower == "forge"
                                    || gv_lower == "neoforge"
                                    || gv_lower == "quilt"
                                {
                                    if !loaders.contains(&gv_lower) {
                                        loaders.push(gv_lower);
                                    }
                                } else if gv.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                                    if !game_versions.contains(gv) {
                                        game_versions.push(gv.clone());
                                    }
                                }
                            }
                        }

                        let version_type = match f.release_type {
                            Some(1) => "release",
                            Some(2) => "beta",
                            Some(3) => "alpha",
                            _ => "release",
                        }
                        .to_string();

                        let download_url =
                            f.download_url.filter(|u| !u.is_empty()).unwrap_or_else(|| {
                                let id_str = f.id.to_string();
                                let (part1, part2) = if id_str.len() > 4 {
                                    let split = id_str.len() - 3;
                                    (&id_str[..split], &id_str[split..])
                                } else {
                                    ("0", id_str.as_str())
                                };
                                format!(
                                    "https://edge.forgecdn.net/files/{}/{}/{}",
                                    part1, part2, f.file_name
                                )
                            });

                        versions.push(UnifiedContentVersion {
                            id: f.id.to_string(),
                            name: f.display_name,
                            version_number: f.id.to_string(),
                            game_versions,
                            loaders,
                            version_type,
                            date_published: f.file_date.unwrap_or_default(),
                            downloads: f.download_count.unwrap_or(0),
                            filename: f.file_name,
                            download_url,
                            size_bytes: f.file_length.unwrap_or(0),
                        });
                    }
                }
            }
        }

        let project_type = match m.class_id {
            Some(4471) => "modpack",
            Some(12) | Some(6929) => "resourcepack",
            Some(6552) => "shader",
            Some(4984) => "addon",
            Some(6913) => "map",
            _ => "mod",
        }
        .to_string();

        let author = m
            .authors
            .as_ref()
            .and_then(|a| a.first())
            .map(|a| a.name.clone())
            .unwrap_or_default();

        let icon_url = m
            .logo
            .as_ref()
            .and_then(|l| l.thumbnail_url.clone().or_else(|| l.url.clone()));

        let screenshots = m
            .screenshots
            .unwrap_or_default()
            .into_iter()
            .map(|s| ContentScreenshot {
                url: s.url,
                title: s.title,
            })
            .collect();

        let (website_url, issues_url, wiki_url, source_url) = if let Some(links) = m.links {
            (
                links.website_url,
                links.issues_url,
                links.wiki_url,
                links.source_url,
            )
        } else {
            (None, None, None, None)
        };

        Ok(UnifiedContentDetails {
            id: m.id.to_string(),
            source: "curseforge".to_string(),
            project_type: project_type.clone(),
            slug: m.slug.clone(),
            title: m.name,
            description: m.summary,
            body,
            author,
            icon_url,
            downloads: m.download_count.map(|d| d as u64).unwrap_or(0),
            follows: None,
            categories: Vec::new(),
            game_versions: Vec::new(),
            loaders: Vec::new(),
            website_url: website_url.or_else(|| {
                if m.class_id == Some(4984) || m.class_id == Some(6929) || m.class_id == Some(6913)
                {
                    Some(format!(
                        "https://www.curseforge.com/minecraft-bedrock/addons/{}",
                        m.slug
                    ))
                } else {
                    Some(format!(
                        "https://www.curseforge.com/minecraft/mc-mods/{}",
                        m.id
                    ))
                }
            }),
            issues_url,
            source_url,
            wiki_url,
            screenshots,
            versions,
        })
    }

    pub async fn get_file(
        &self,
        project_id: u32,
        file_id: u32,
    ) -> Result<CurseForgeFileDetail, ContentError> {
        let file_url = format!("{}/mods/{}/files/{}", self.base_url, project_id, file_id);
        let resp = self
            .client
            .get(&file_url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .map_err(ContentError::Transport)?;

        if !resp.status().is_success() {
            return Err(ContentError::HttpStatus(resp.status()));
        }

        let body: CurseForgeSingleFileResponse =
            resp.json().await.map_err(ContentError::Response)?;
        Ok(body.data)
    }
}
