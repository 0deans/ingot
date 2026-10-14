//! Modrinth API client for search and project details retrieval.
use super::{
    error::ContentError,
    models::{
        ContentScreenshot, ContentSearchQuery, ContentSearchResult, UnifiedContentDetails,
        UnifiedContentItem, UnifiedContentVersion,
    },
};
use reqwest::Client;
use serde::Deserialize;

pub const DEFAULT_MODRINTH_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthHit>,
    total_hits: u64,
    offset: u32,
    limit: u32,
}

#[derive(Debug, Deserialize)]
struct ModrinthHit {
    project_id: String,
    project_type: String,
    slug: String,
    author: String,
    title: String,
    description: String,
    categories: Option<Vec<String>>,
    display_categories: Option<Vec<String>>,
    versions: Option<Vec<String>>,
    downloads: u64,
    follows: Option<u64>,
    icon_url: Option<String>,
    date_modified: Option<String>,
    latest_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthProjectDetail {
    id: String,
    slug: String,
    project_type: String,
    title: String,
    description: String,
    body: Option<String>,
    issues_url: Option<String>,
    source_url: Option<String>,
    wiki_url: Option<String>,
    icon_url: Option<String>,
    downloads: u64,
    followers: Option<u64>,
    categories: Option<Vec<String>>,
    additional_categories: Option<Vec<String>>,
    game_versions: Option<Vec<String>>,
    loaders: Option<Vec<String>>,
    gallery: Option<Vec<ModrinthGalleryItem>>,
}

#[derive(Debug, Deserialize)]
struct ModrinthGalleryItem {
    url: String,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersionItem {
    id: String,
    name: String,
    version_number: String,
    game_versions: Option<Vec<String>>,
    loaders: Option<Vec<String>>,
    version_type: Option<String>,
    date_published: Option<String>,
    downloads: Option<u64>,
    files: Option<Vec<ModrinthFileItem>>,
}

#[derive(Debug, Deserialize)]
struct ModrinthFileItem {
    url: String,
    filename: String,
    primary: Option<bool>,
    size: Option<u64>,
}

pub struct ModrinthClient {
    client: Client,
    base_url: String,
}

impl Default for ModrinthClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ModrinthClient {
    pub fn new() -> Self {
        Self::with_endpoint(
            Client::builder()
                .user_agent(crate::USER_AGENT)
                .build()
                .unwrap_or_default(),
            DEFAULT_MODRINTH_BASE,
        )
    }

    pub fn with_endpoint(client: Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    pub async fn search(
        &self,
        query: &ContentSearchQuery<'_>,
    ) -> Result<ContentSearchResult, ContentError> {
        let mut facets: Vec<Vec<String>> = Vec::new();

        let pt = match query.project_type.to_lowercase().as_str() {
            "mod" | "mods" => Some("project_type:mod"),
            "modpack" | "modpacks" => Some("project_type:modpack"),
            "resourcepack" | "resourcepacks" => Some("project_type:resourcepack"),
            "shader" | "shaders" | "shaderpack" | "shaderpacks" => Some("project_type:shader"),
            _ => None,
        };
        if let Some(p) = pt {
            facets.push(vec![p.to_string()]);
        }

        if let Some(gv) = query.game_version {
            let trimmed = gv.trim();
            if !trimmed.is_empty() && trimmed != "all" {
                facets.push(vec![format!("versions:{trimmed}")]);
            }
        }

        if let Some(ldr) = query.loader {
            let trimmed = ldr.trim().to_lowercase();
            if !trimmed.is_empty() && trimmed != "all" {
                facets.push(vec![format!("categories:{trimmed}")]);
            }
        }

        let sort_index = match query.sort.unwrap_or("downloads") {
            "downloads" | "popularity" => "downloads",
            "relevance" => "relevance",
            "updated" | "recent" => "updated",
            "newest" => "newest",
            "follows" => "follows",
            _ => "downloads",
        };

        let search_url = format!("{}/search", self.base_url);
        let mut req = self.client.get(&search_url).query(&[
            ("offset", query.offset.to_string()),
            ("limit", query.limit.to_string()),
            ("index", sort_index.to_string()),
        ]);

        if let Some(q) = query.query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                req = req.query(&[("query", trimmed)]);
            }
        }

        if !facets.is_empty() {
            let facets_json = serde_json::to_string(&facets).unwrap_or_default();
            req = req.query(&[("facets", facets_json)]);
        }

        let res = req.send().await.map_err(ContentError::Transport)?;

        if !res.status().is_success() {
            return Err(ContentError::HttpStatus(res.status()));
        }

        let parsed = res
            .json::<ModrinthSearchResponse>()
            .await
            .map_err(ContentError::Response)?;

        let items = parsed
            .hits
            .into_iter()
            .map(|hit| {
                let cat_list = hit
                    .display_categories
                    .or(hit.categories)
                    .unwrap_or_default();

                let mut loaders = Vec::new();
                let mut categories = Vec::new();
                for c in cat_list {
                    let cl = c.to_lowercase();
                    if cl == "fabric" || cl == "forge" || cl == "neoforge" || cl == "quilt" {
                        loaders.push(c);
                    } else {
                        categories.push(c);
                    }
                }

                let project_type_clean = match hit.project_type.as_str() {
                    "mod" => "mod",
                    "modpack" => "modpack",
                    "resourcepack" => "resourcepack",
                    "shader" => "shader",
                    other => other,
                }
                .to_string();

                let website_url = format!("https://modrinth.com/{}/{}", hit.project_type, hit.slug);

                UnifiedContentItem {
                    id: format!("mr:{}", hit.project_id),
                    source: "modrinth".to_string(),
                    project_type: project_type_clean,
                    slug: hit.slug,
                    title: hit.title,
                    description: hit.description,
                    author: hit.author,
                    icon_url: hit.icon_url,
                    downloads: hit.downloads,
                    follows: hit.follows,
                    categories,
                    latest_version: hit.latest_version,
                    game_versions: hit.versions.unwrap_or_default(),
                    loaders,
                    website_url: Some(website_url),
                    date_modified: hit.date_modified,
                }
            })
            .collect();

        Ok(ContentSearchResult {
            items,
            total_hits: parsed.total_hits,
            offset: parsed.offset,
            limit: parsed.limit,
        })
    }

    pub async fn get_details(
        &self,
        project_id: &str,
    ) -> Result<UnifiedContentDetails, ContentError> {
        let proj_url = format!("{}/project/{}", self.base_url, project_id);
        let vers_url = format!("{}/project/{}/version", self.base_url, project_id);

        let (proj_res, vers_res) = tokio::join!(
            self.client.get(&proj_url).send(),
            self.client.get(&vers_url).send()
        );

        let proj_resp = proj_res.map_err(ContentError::Transport)?;
        if !proj_resp.status().is_success() {
            return Err(ContentError::HttpStatus(proj_resp.status()));
        }
        let proj: ModrinthProjectDetail = proj_resp.json().await.map_err(ContentError::Response)?;

        let mut versions = Vec::new();
        if let Ok(v_resp) = vers_res {
            if v_resp.status().is_success() {
                if let Ok(v_list) = v_resp.json::<Vec<ModrinthVersionItem>>().await {
                    for v in v_list {
                        if let Some(files) = v.files {
                            if let Some(f) = files
                                .iter()
                                .find(|f| f.primary.unwrap_or(false))
                                .or_else(|| files.first())
                            {
                                versions.push(UnifiedContentVersion {
                                    id: v.id,
                                    name: v.name,
                                    version_number: v.version_number,
                                    game_versions: v.game_versions.unwrap_or_default(),
                                    loaders: v.loaders.unwrap_or_default(),
                                    version_type: v
                                        .version_type
                                        .unwrap_or_else(|| "release".into()),
                                    date_published: v.date_published.unwrap_or_default(),
                                    downloads: v.downloads.unwrap_or(0),
                                    filename: f.filename.clone(),
                                    download_url: f.url.clone(),
                                    size_bytes: f.size.unwrap_or(0),
                                });
                            }
                        }
                    }
                }
            }
        }

        let mut categories = proj.categories.unwrap_or_default();
        if let Some(add_cats) = proj.additional_categories {
            for c in add_cats {
                if !categories.contains(&c) {
                    categories.push(c);
                }
            }
        }

        let screenshots = proj
            .gallery
            .unwrap_or_default()
            .into_iter()
            .map(|g| ContentScreenshot {
                url: g.url,
                title: g.title,
            })
            .collect();

        Ok(UnifiedContentDetails {
            id: format!("mr:{}", proj.id),
            source: "modrinth".to_string(),
            project_type: proj.project_type,
            slug: proj.slug.clone(),
            title: proj.title,
            description: proj.description,
            body: proj.body.unwrap_or_default(),
            author: "".to_string(),
            icon_url: proj.icon_url,
            downloads: proj.downloads,
            follows: proj.followers,
            categories,
            game_versions: proj.game_versions.unwrap_or_default(),
            loaders: proj.loaders.unwrap_or_default(),
            website_url: Some(format!("https://modrinth.com/{}/{}", proj.id, proj.slug)),
            issues_url: proj.issues_url,
            source_url: proj.source_url,
            wiki_url: proj.wiki_url,
            screenshots,
            versions,
        })
    }
}
