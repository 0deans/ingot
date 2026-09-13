use serde::Deserialize;

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
        .user_agent("Ingot-Launcher/0.1.0 (https://github.com/0deans/ingot)")
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
}

