use crate::minecraft::instance::{self, InstanceConfig, ModLoaderType};
use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::Path;
use tauri::Runtime;
use zip::ZipArchive;

const CURSEFORGE_API_KEY: &str = "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm";

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedContentItem {
    pub id: String,
    pub source: String, // "modrinth" | "curseforge"
    pub project_type: String, // "modpack" | "mod" | "resourcepack" | "shader"
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: Option<u64>,
    pub categories: Vec<String>,
    pub latest_version: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub website_url: Option<String>,
    pub date_modified: Option<String>,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchResult {
    pub items: Vec<UnifiedContentItem>,
    pub total_hits: u64,
    pub offset: u32,
    pub limit: u32,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct ContentScreenshot {
    pub url: String,
    pub title: Option<String>,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedContentVersion {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub version_type: String, // "release" | "beta" | "alpha"
    pub date_published: String,
    pub downloads: u64,
    pub filename: String,
    pub download_url: String,
    pub size_bytes: u64,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedContentDetails {
    pub id: String,
    pub source: String,
    pub project_type: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: Option<u64>,
    pub categories: Vec<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub website_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub screenshots: Vec<ContentScreenshot>,
    pub versions: Vec<UnifiedContentVersion>,
}

// -------------------------------------------------------------------------------------------------
// Modrinth Models
// -------------------------------------------------------------------------------------------------

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

// -------------------------------------------------------------------------------------------------
// CurseForge Models
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CurseForgeSearchResponse {
    data: Vec<CurseForgeModItem>,
    pagination: CurseForgePagination,
}

#[derive(Debug, Deserialize)]
struct CurseForgePagination {
    #[serde(rename = "totalCount")]
    total_count: u64,
    index: u32,
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

// -------------------------------------------------------------------------------------------------
// Modrinth Search
// -------------------------------------------------------------------------------------------------

async fn search_modrinth(
    client: &reqwest::Client,
    project_type: &str,
    query: Option<&str>,
    game_version: Option<&str>,
    loader: Option<&str>,
    sort: Option<&str>,
    offset: u32,
    limit: u32,
) -> Result<ContentSearchResult, String> {
    let mut facets: Vec<Vec<String>> = Vec::new();

    let pt = match project_type.to_lowercase().as_str() {
        "mod" | "mods" => Some("project_type:mod"),
        "modpack" | "modpacks" => Some("project_type:modpack"),
        "resourcepack" | "resourcepacks" => Some("project_type:resourcepack"),
        "shader" | "shaders" | "shaderpack" | "shaderpacks" => Some("project_type:shader"),
        _ => None,
    };
    if let Some(p) = pt {
        facets.push(vec![p.to_string()]);
    }

    if let Some(gv) = game_version {
        let trimmed = gv.trim();
        if !trimmed.is_empty() && trimmed != "all" {
            facets.push(vec![format!("versions:{trimmed}")]);
        }
    }

    if let Some(ldr) = loader {
        let trimmed = ldr.trim().to_lowercase();
        if !trimmed.is_empty() && trimmed != "all" {
            facets.push(vec![format!("categories:{trimmed}")]);
        }
    }

    let sort_index = match sort.unwrap_or("downloads") {
        "downloads" | "popularity" => "downloads",
        "relevance" => "relevance",
        "updated" | "recent" => "updated",
        "newest" => "newest",
        "follows" => "follows",
        _ => "downloads",
    };

    let mut req = client
        .get("https://api.modrinth.com/v2/search")
        .query(&[
            ("offset", offset.to_string()),
            ("limit", limit.to_string()),
            ("index", sort_index.to_string()),
        ]);

    if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            req = req.query(&[("query", trimmed)]);
        }
    }

    if !facets.is_empty() {
        let facets_json = serde_json::to_string(&facets).unwrap_or_default();
        req = req.query(&[("facets", facets_json)]);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Modrinth request failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Modrinth returned HTTP {}", res.status()));
    }

    let parsed = res
        .json::<ModrinthSearchResponse>()
        .await
        .map_err(|e| format!("Failed to parse Modrinth response: {e}"))?;

    let items = parsed
        .hits
        .into_iter()
        .map(|hit| {
            let cat_list = hit
                .display_categories
                .or(hit.categories)
                .unwrap_or_default();

            // Extract loaders vs categories
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

// -------------------------------------------------------------------------------------------------
// CurseForge Search
// -------------------------------------------------------------------------------------------------

async fn search_curseforge(
    client: &reqwest::Client,
    project_type: &str,
    query: Option<&str>,
    game_version: Option<&str>,
    loader: Option<&str>,
    sort: Option<&str>,
    offset: u32,
    limit: u32,
) -> Result<ContentSearchResult, String> {
    let class_id = match project_type.to_lowercase().as_str() {
        "mod" | "mods" => Some(6),
        "modpack" | "modpacks" => Some(4471),
        "resourcepack" | "resourcepacks" => Some(12),
        "shader" | "shaders" | "shaderpack" | "shaderpacks" => Some(6552),
        _ => None,
    };

    let mod_loader_type = match loader.unwrap_or("").to_lowercase().as_str() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    };

    let sort_field = match sort.unwrap_or("downloads") {
        "downloads" | "popularity" => 5, // TotalDownloads
        "updated" | "recent" => 3,       // LastUpdated
        "newest" => 1,                   // Featured / New
        "relevance" => 2,                // Popularity/Score
        _ => 5,
    };

    let mut req = client
        .get("https://api.curseforge.com/v1/mods/search")
        .header("x-api-key", CURSEFORGE_API_KEY)
        .header("Accept", "application/json")
        .query(&[
            ("gameId", "432".to_string()),
            ("index", offset.to_string()),
            ("pageSize", limit.to_string()),
            ("sortField", sort_field.to_string()),
            ("sortOrder", "desc".to_string()),
        ]);

    if let Some(cid) = class_id {
        req = req.query(&[("classId", cid.to_string())]);
    }

    if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            req = req.query(&[("searchFilter", trimmed.to_string())]);
        }
    }

    if let Some(gv) = game_version {
        let trimmed = gv.trim();
        if !trimmed.is_empty() && trimmed != "all" {
            req = req.query(&[("gameVersion", trimmed.to_string())]);
        }
    }

    if let Some(mlt) = mod_loader_type {
        req = req.query(&[("modLoaderType", mlt.to_string())]);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("CurseForge request failed: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("CurseForge returned HTTP {}", res.status()));
    }

    let parsed = res
        .json::<CurseForgeSearchResponse>()
        .await
        .map_err(|e| format!("Failed to parse CurseForge response: {e}"))?;

    let items = parsed
        .data
        .into_iter()
        .map(|item| {
            let author = item
                .authors
                .and_then(|a| a.into_iter().next().map(|x| x.name))
                .unwrap_or_else(|| "Unknown".to_string());

            let icon_url = item
                .logo
                .and_then(|l| l.thumbnail_url.or(l.url));

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
                            if vl == "fabric" || vl == "forge" || vl == "neoforge" || vl == "quilt" {
                                if !loaders.contains(&v) {
                                    loaders.push(v);
                                }
                            } else if !v.eq_ignore_ascii_case("client") && !v.eq_ignore_ascii_case("server") {
                                if !game_versions.contains(&v) {
                                    game_versions.push(v);
                                }
                            }
                        }
                    }
                }
            }

            let project_type_clean = match item.class_id {
                Some(4471) => "modpack",
                Some(6) => "mod",
                Some(12) => "resourcepack",
                Some(6552) => "shader",
                _ => match project_type.to_lowercase().as_str() {
                    "modpack" | "modpacks" => "modpack",
                    "resourcepack" | "resourcepacks" => "resourcepack",
                    "shader" | "shaders" | "shaderpack" => "shader",
                    _ => "mod",
                },
            }
            .to_string();

            let website_url = item.links.and_then(|l| l.website_url);

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
                website_url,
                date_modified: item.date_modified,
            }
        })
        .collect();

    Ok(ContentSearchResult {
        items,
        total_hits: parsed.pagination.total_count,
        offset: parsed.pagination.index,
        limit: parsed.pagination.page_size,
    })
}

// -------------------------------------------------------------------------------------------------
// Public Unified Search API
// -------------------------------------------------------------------------------------------------

pub async fn search_content(
    source: String,
    project_type: String,
    query: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    sort: Option<String>,
    page: u32,
    page_size: u32,
) -> Result<ContentSearchResult, String> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let offset = page * page_size;
    let src_clean = source.trim().to_lowercase();

    if src_clean == "modrinth" {
        return search_modrinth(
            &client,
            &project_type,
            query.as_deref(),
            game_version.as_deref(),
            loader.as_deref(),
            sort.as_deref(),
            offset,
            page_size,
        )
        .await;
    }

    if src_clean == "curseforge" {
        return search_curseforge(
            &client,
            &project_type,
            query.as_deref(),
            game_version.as_deref(),
            loader.as_deref(),
            sort.as_deref(),
            offset,
            page_size,
        )
        .await;
    }

    // "all": query both in parallel
    let (res_mr, res_cf) = tokio::join!(
        search_modrinth(
            &client,
            &project_type,
            query.as_deref(),
            game_version.as_deref(),
            loader.as_deref(),
            sort.as_deref(),
            offset / 2,
            page_size,
        ),
        search_curseforge(
            &client,
            &project_type,
            query.as_deref(),
            game_version.as_deref(),
            loader.as_deref(),
            sort.as_deref(),
            offset / 2,
            page_size,
        )
    );

    let mut combined_items = Vec::new();
    let mut total_hits = 0;

    if let Ok(mr) = res_mr {
        total_hits += mr.total_hits;
        combined_items.extend(mr.items);
    }
    if let Ok(cf) = res_cf {
        total_hits += cf.total_hits;
        combined_items.extend(cf.items);
    }

    // Sort combined items by downloads descending
    combined_items.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    combined_items.truncate(page_size as usize);

    Ok(ContentSearchResult {
        items: combined_items,
        total_hits,
        offset,
        limit: page_size,
    })
}

// -------------------------------------------------------------------------------------------------
// Content Details & Installation Models
// -------------------------------------------------------------------------------------------------

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
struct CurseForgeFileDetail {
    id: u32,
    display_name: String,
    file_name: String,
    release_type: Option<u32>,
    file_date: Option<String>,
    file_length: Option<u64>,
    download_count: Option<u64>,
    download_url: Option<String>,
    game_versions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeSingleFileResponse {
    data: CurseForgeFileDetail,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ModrinthIndex {
    game: Option<String>,
    name: Option<String>,
    dependencies: Option<std::collections::HashMap<String, String>>,
    files: Option<Vec<ModrinthIndexFile>>,
}

#[derive(Debug, Deserialize)]
struct ModrinthIndexFile {
    path: String,
    downloads: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeManifest {
    minecraft: CurseForgeManifestMinecraft,
    name: Option<String>,
    files: Option<Vec<CurseForgeManifestFile>>,
    overrides: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeManifestMinecraft {
    version: String,
    mod_loaders: Option<Vec<CurseForgeManifestModLoader>>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeManifestModLoader {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CurseForgeManifestFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
}

// -------------------------------------------------------------------------------------------------
// Content Details Implementation
// -------------------------------------------------------------------------------------------------

pub async fn get_content_details(
    source: &str,
    project_id: &str,
) -> Result<UnifiedContentDetails, String> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let trimmed_id = project_id.trim();
    let (detected_source, clean_id) = if let Some(stripped) = trimmed_id.strip_prefix("mr:") {
        ("modrinth", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("cf:") {
        ("curseforge", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("modrinth:") {
        ("modrinth", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("curseforge:") {
        ("curseforge", stripped)
    } else {
        (source.trim(), trimmed_id)
    };

    let src = if !detected_source.is_empty() {
        detected_source.to_lowercase()
    } else {
        source.trim().to_lowercase()
    };

    if src == "modrinth" {
        get_modrinth_details(&client, clean_id).await
    } else if src == "curseforge" {
        get_curseforge_details(&client, clean_id).await
    } else {
        Err(format!("Unknown source: {source}"))
    }
}

async fn get_modrinth_details(
    client: &reqwest::Client,
    project_id: &str,
) -> Result<UnifiedContentDetails, String> {
    let proj_url = format!("https://api.modrinth.com/v2/project/{}", project_id);
    let vers_url = format!("https://api.modrinth.com/v2/project/{}/version", project_id);

    let (proj_res, vers_res) = tokio::join!(
        client.get(&proj_url).send(),
        client.get(&vers_url).send()
    );

    let proj_resp = proj_res.map_err(|e| format!("Modrinth project request failed: {e}"))?;
    if !proj_resp.status().is_success() {
        return Err(format!("Modrinth project not found (status {})", proj_resp.status()));
    }
    let proj: ModrinthProjectDetail = proj_resp.json().await
        .map_err(|e| format!("Failed to parse Modrinth project data: {e}"))?;

    let mut versions = Vec::new();
    if let Ok(v_resp) = vers_res {
        if v_resp.status().is_success() {
            if let Ok(v_list) = v_resp.json::<Vec<ModrinthVersionItem>>().await {
                for v in v_list {
                    if let Some(files) = v.files {
                        if let Some(f) = files.iter().find(|f| f.primary.unwrap_or(false)).or_else(|| files.first()) {
                            versions.push(UnifiedContentVersion {
                                id: v.id,
                                name: v.name,
                                version_number: v.version_number,
                                game_versions: v.game_versions.unwrap_or_default(),
                                loaders: v.loaders.unwrap_or_default(),
                                version_type: v.version_type.unwrap_or_else(|| "release".into()),
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

    let screenshots = proj.gallery.unwrap_or_default().into_iter().map(|g| ContentScreenshot {
        url: g.url,
        title: g.title,
    }).collect();

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

async fn get_curseforge_details(
    client: &reqwest::Client,
    project_id: &str,
) -> Result<UnifiedContentDetails, String> {
    let mod_id: u32 = match project_id.parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            let search_url = format!("https://api.curseforge.com/v1/mods/search?gameId=432&slug={}", project_id);
            let resp = client.get(&search_url)
                .header("x-api-key", CURSEFORGE_API_KEY)
                .send()
                .await
                .map_err(|e| format!("CurseForge slug search failed: {e}"))?;
            if resp.status().is_success() {
                let parsed: CurseForgeSearchResponse = resp.json().await
                    .map_err(|e| format!("Failed to parse CurseForge slug response: {e}"))?;
                parsed.data.first().map(|m| m.id as u32).ok_or_else(|| format!("CurseForge mod '{project_id}' not found"))?
            } else {
                return Err(format!("Invalid CurseForge mod ID or slug: {project_id}"));
            }
        }
    };

    let mod_url = format!("https://api.curseforge.com/v1/mods/{}", mod_id);
    let desc_url = format!("https://api.curseforge.com/v1/mods/{}/description", mod_id);
    let files_url = format!("https://api.curseforge.com/v1/mods/{}/files?pageSize=50", mod_id);

    let (mod_res, desc_res, files_res) = tokio::join!(
        client.get(&mod_url).header("x-api-key", CURSEFORGE_API_KEY).send(),
        client.get(&desc_url).header("x-api-key", CURSEFORGE_API_KEY).send(),
        client.get(&files_url).header("x-api-key", CURSEFORGE_API_KEY).send(),
    );

    let mod_resp = mod_res.map_err(|e| format!("CurseForge mod request failed: {e}"))?;
    if !mod_resp.status().is_success() {
        return Err(format!("CurseForge mod not found (status {})", mod_resp.status()));
    }
    let mod_data: CurseForgeModDetailResponse = mod_resp.json().await
        .map_err(|e| format!("Failed to parse CurseForge mod data: {e}"))?;
    let m = mod_data.data;

    let body = match desc_res {
        Ok(r) if r.status().is_success() => {
            r.json::<CurseForgeDescriptionResponse>().await.map(|d| d.data).unwrap_or_default()
        }
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
                            if gv_lower == "fabric" || gv_lower == "forge" || gv_lower == "neoforge" || gv_lower == "quilt" {
                                if !loaders.contains(&gv_lower) {
                                    loaders.push(gv_lower);
                                }
                            } else if gv.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
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
                    }.to_string();

                    let download_url = f.download_url.filter(|u| !u.is_empty()).unwrap_or_else(|| {
                        let id_str = f.id.to_string();
                        let (part1, part2) = if id_str.len() > 4 {
                            let split = id_str.len() - 3;
                            (&id_str[..split], &id_str[split..])
                        } else {
                            ("0", id_str.as_str())
                        };
                        format!("https://edge.forgecdn.net/files/{}/{}/{}", part1, part2, f.file_name)
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
        Some(12) => "resourcepack",
        Some(6552) => "shader",
        _ => "mod",
    }.to_string();

    let author = m.authors.as_ref().and_then(|a| a.first()).map(|a| a.name.clone()).unwrap_or_default();
    let icon_url = m.logo.as_ref().and_then(|l| l.thumbnail_url.clone().or_else(|| l.url.clone()));
    let screenshots = m.screenshots.unwrap_or_default().into_iter().map(|s| ContentScreenshot {
        url: s.url,
        title: s.title,
    }).collect();

    let (website_url, issues_url, wiki_url, source_url) = if let Some(links) = m.links {
        (links.website_url, links.issues_url, links.wiki_url, links.source_url)
    } else {
        (None, None, None, None)
    };

    Ok(UnifiedContentDetails {
        id: m.id.to_string(),
        source: "curseforge".to_string(),
        project_type,
        slug: m.slug,
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
        website_url: website_url.or_else(|| Some(format!("https://www.curseforge.com/minecraft/mc-mods/{}", m.id))),
        issues_url,
        source_url,
        wiki_url,
        screenshots,
        versions,
    })
}

// -------------------------------------------------------------------------------------------------
// Installation Implementation
// -------------------------------------------------------------------------------------------------

pub async fn install_content_file<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    project_type: &str,
    download_url: &str,
    filename: &str,
) -> Result<String, String> {
    let instance_dir = instance::get_instance_dir(app, instance_id)?;

    let clean_name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid filename".to_string())?;

    let subfolder = match project_type.to_lowercase().as_str() {
        "mod" => "mods",
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        _ => "mods",
    };

    let target_dir = instance_dir.join(subfolder);
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create folder {}: {e}", target_dir.display()))?;
    }

    let target_file = target_dir.join(clean_name);
    let tmp_file = target_dir.join(format!("{}.tmp", clean_name));

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let mut req = client.get(download_url);
    if download_url.contains("curseforge.com") {
        req = req.header("x-api-key", CURSEFORGE_API_KEY);
    }

    let resp = req.send().await.map_err(|e| format!("Download request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Download failed with HTTP status: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Failed to receive file data: {e}"))?;
    fs::write(&tmp_file, &bytes)
        .map_err(|e| format!("Failed to write temporary file: {e}"))?;

    fs::rename(&tmp_file, &target_file)
        .map_err(|e| format!("Failed to finalize installed file: {e}"))?;

    Ok(format!("Installed {} into {}", clean_name, subfolder))
}

pub async fn install_modpack_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: &str,
    source: &str,
    download_url: &str,
    filename: &str,
) -> Result<InstanceConfig, String> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let mut req = client.get(download_url);
    if download_url.contains("curseforge.com") {
        req = req.header("x-api-key", CURSEFORGE_API_KEY);
    }

    let resp = req.send().await.map_err(|e| format!("Failed to download modpack archive: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Modpack download failed with HTTP status: {}", resp.status()));
    }

    let pack_bytes = resp.bytes().await.map_err(|e| format!("Failed to read modpack bytes: {e}"))?;

    let cursor = std::io::Cursor::new(pack_bytes);
    let mut zip = ZipArchive::new(cursor).map_err(|e| format!("Failed to read modpack zip archive: {e}"))?;

    let src = source.trim().to_lowercase();
    let is_modrinth = src == "modrinth" || filename.ends_with(".mrpack") || zip.by_name("modrinth.index.json").is_ok();

    if is_modrinth {
        let index_data = {
            let mut file = zip.by_name("modrinth.index.json")
                .map_err(|e| format!("Missing modrinth.index.json in pack: {e}"))?;
            let mut content = String::new();
            file.read_to_string(&mut content)
                .map_err(|e| format!("Failed to read modrinth.index.json: {e}"))?;
            content
        };

        let index: ModrinthIndex = serde_json::from_str(&index_data)
            .map_err(|e| format!("Failed to parse modrinth.index.json: {e}"))?;

        let deps = index.dependencies.unwrap_or_default();
        let game_version = deps.get("minecraft").cloned().unwrap_or_else(|| "1.20.1".into());

        let (loader, loader_version) = if let Some(lv) = deps.get("fabric-loader") {
            (ModLoaderType::Fabric, Some(lv.clone()))
        } else if let Some(lv) = deps.get("forge") {
            (ModLoaderType::Forge, Some(lv.clone()))
        } else if let Some(lv) = deps.get("neoforge") {
            (ModLoaderType::NeoForge, Some(lv.clone()))
        } else if let Some(lv) = deps.get("quilt-loader") {
            (ModLoaderType::Quilt, Some(lv.clone()))
        } else {
            (ModLoaderType::Vanilla, None)
        };

        let inst_name = if !name.trim().is_empty() {
            name.trim().to_string()
        } else {
            index.name.unwrap_or_else(|| "Modpack Instance".into())
        };

        let instance = instance::create_instance(app, inst_name, game_version, loader, loader_version)?;
        let inst_dir = instance::get_instance_dir(app, &instance.id)?;

        // Extract overrides
        for i in 0..zip.len() {
            let mut file = match zip.by_index(i) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let file_name = file.name().to_string();
            let subpath = if let Some(stripped) = file_name.strip_prefix("overrides/") {
                stripped
            } else if let Some(stripped) = file_name.strip_prefix("client-overrides/") {
                stripped
            } else {
                continue;
            };

            if subpath.is_empty() {
                continue;
            }

            let out_path = inst_dir.join(subpath);
            if file.is_dir() {
                let _ = fs::create_dir_all(&out_path);
            } else {
                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut outfile = match fs::File::create(&out_path) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let _ = std::io::copy(&mut file, &mut outfile);
            }
        }

        // Download mod files listed in index
        if let Some(files) = index.files {
            let mods_dir = inst_dir.join("mods");
            let _ = fs::create_dir_all(&mods_dir);

            for f in files {
                if let Some(dl_url) = f.downloads.first() {
                    let dest = inst_dir.join(&f.path);
                    if let Some(p) = dest.parent() {
                        let _ = fs::create_dir_all(p);
                    }
                    if let Ok(dl_resp) = client.get(dl_url).send().await {
                        if dl_resp.status().is_success() {
                            if let Ok(b) = dl_resp.bytes().await {
                                let _ = fs::write(&dest, &b);
                            }
                        }
                    }
                }
            }
        }

        Ok(instance)
    } else {
        let manifest_data = {
            let mut file = zip.by_name("manifest.json")
                .map_err(|e| format!("Missing manifest.json in CurseForge modpack: {e}"))?;
            let mut content = String::new();
            file.read_to_string(&mut content)
                .map_err(|e| format!("Failed to read manifest.json: {e}"))?;
            content
        };

        let manifest: CurseForgeManifest = serde_json::from_str(&manifest_data)
            .map_err(|e| format!("Failed to parse CurseForge manifest.json: {e}"))?;

        let game_version = manifest.minecraft.version;
        let mut loader = ModLoaderType::Vanilla;
        let mut loader_version = None;

        if let Some(loaders) = manifest.minecraft.mod_loaders {
            if let Some(primary) = loaders.into_iter().next() {
                let id = primary.id.to_lowercase();
                if id.starts_with("fabric-") {
                    loader = ModLoaderType::Fabric;
                    loader_version = Some(id.trim_start_matches("fabric-").to_string());
                } else if id.starts_with("forge-") {
                    loader = ModLoaderType::Forge;
                    loader_version = Some(id.trim_start_matches("forge-").to_string());
                } else if id.starts_with("neoforge-") {
                    loader = ModLoaderType::NeoForge;
                    loader_version = Some(id.trim_start_matches("neoforge-").to_string());
                } else if id.starts_with("quilt-") {
                    loader = ModLoaderType::Quilt;
                    loader_version = Some(id.trim_start_matches("quilt-").to_string());
                }
            }
        }

        let inst_name = if !name.trim().is_empty() {
            name.trim().to_string()
        } else {
            manifest.name.unwrap_or_else(|| "CurseForge Modpack".into())
        };

        let instance = instance::create_instance(app, inst_name, game_version, loader, loader_version)?;
        let inst_dir = instance::get_instance_dir(app, &instance.id)?;

        let overrides_folder = manifest.overrides.unwrap_or_else(|| "overrides".into());
        let prefix = format!("{}/", overrides_folder);

        for i in 0..zip.len() {
            let mut file = match zip.by_index(i) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let file_name = file.name().to_string();
            let subpath = if let Some(stripped) = file_name.strip_prefix(&prefix) {
                stripped
            } else {
                continue;
            };

            if subpath.is_empty() {
                continue;
            }

            let out_path = inst_dir.join(subpath);
            if file.is_dir() {
                let _ = fs::create_dir_all(&out_path);
            } else {
                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let mut outfile = match fs::File::create(&out_path) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let _ = std::io::copy(&mut file, &mut outfile);
            }
        }

        if let Some(files) = manifest.files {
            let mods_dir = inst_dir.join("mods");
            let _ = fs::create_dir_all(&mods_dir);

            for f in files {
                let file_info_url = format!("https://api.curseforge.com/v1/mods/{}/files/{}", f.project_id, f.file_id);
                if let Ok(info_resp) = client.get(&file_info_url).header("x-api-key", CURSEFORGE_API_KEY).send().await {
                    if info_resp.status().is_success() {
                        if let Ok(file_info) = info_resp.json::<CurseForgeSingleFileResponse>().await {
                            let item = file_info.data;
                            let dl_url = item.download_url.filter(|u| !u.is_empty()).unwrap_or_else(|| {
                                let id_str = item.id.to_string();
                                let (p1, p2) = if id_str.len() > 4 {
                                    let s = id_str.len() - 3;
                                    (&id_str[..s], &id_str[s..])
                                } else {
                                    ("0", id_str.as_str())
                                };
                                format!("https://edge.forgecdn.net/files/{}/{}/{}", p1, p2, item.file_name)
                            });

                            let dest = mods_dir.join(&item.file_name);
                            if let Ok(dl_resp) = client.get(&dl_url).send().await {
                                if dl_resp.status().is_success() {
                                    if let Ok(b) = dl_resp.bytes().await {
                                        let _ = fs::write(&dest, &b);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(instance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_modrinth_mods() {
        let res = search_content(
            "modrinth".into(),
            "mod".into(),
            Some("sodium".into()),
            Some("1.20.1".into()),
            Some("fabric".into()),
            Some("downloads".into()),
            0,
            5,
        )
        .await;

        assert!(res.is_ok(), "search_content failed: {:?}", res.err());
        let result = res.unwrap();
        assert!(!result.items.is_empty(), "expected items from Modrinth");
        assert_eq!(result.items[0].source, "modrinth");
    }

    #[tokio::test]
    async fn test_search_curseforge_mods() {
        let res = search_content(
            "curseforge".into(),
            "mod".into(),
            Some("jei".into()),
            Some("1.20.1".into()),
            Some("forge".into()),
            Some("downloads".into()),
            0,
            5,
        )
        .await;

        assert!(res.is_ok(), "CurseForge search failed: {:?}", res.err());
        let result = res.unwrap();
        assert!(!result.items.is_empty(), "expected items from CurseForge");
        assert_eq!(result.items[0].source, "curseforge");
    }

    #[tokio::test]
    async fn test_get_modrinth_details() {
        let res = get_content_details("modrinth", "mr:sodium").await;
        assert!(res.is_ok(), "get_modrinth_details failed: {:?}", res.err());
        let details = res.unwrap();
        assert_eq!(details.slug, "sodium");
        assert!(!details.versions.is_empty(), "expected versions for Sodium");
    }

    #[tokio::test]
    async fn test_get_curseforge_details() {
        let res = get_content_details("curseforge", "cf:238222").await;
        assert!(res.is_ok(), "get_curseforge_details failed: {:?}", res.err());
        let details = res.unwrap();
        assert_eq!(details.slug, "jei");
        assert!(!details.versions.is_empty(), "expected versions for JEI");
    }
}


