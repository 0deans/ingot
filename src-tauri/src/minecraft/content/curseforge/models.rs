use crate::minecraft::content::models::{
    ContentScreenshot, UnifiedContentDetails, UnifiedContentItem, UnifiedContentVersion,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeSearchResponse {
    pub(crate) data: Vec<CurseForgeModItem>,
    pub(crate) pagination: CurseForgePagination,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgePagination {
    #[serde(rename = "totalCount")]
    pub(crate) total_count: u64,
    #[allow(dead_code)]
    pub(crate) index: u32,
    #[allow(dead_code)]
    #[serde(rename = "pageSize")]
    pub(crate) page_size: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeModItem {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) summary: String,
    #[serde(rename = "downloadCount")]
    pub(crate) download_count: f64,
    pub(crate) authors: Option<Vec<CurseForgeAuthor>>,
    pub(crate) logo: Option<CurseForgeLogo>,
    pub(crate) categories: Option<Vec<CurseForgeCategory>>,
    #[serde(rename = "latestFiles")]
    pub(crate) latest_files: Option<Vec<CurseForgeFile>>,
    pub(crate) links: Option<CurseForgeLinks>,
    pub(crate) date_modified: Option<String>,
    #[serde(rename = "classId")]
    pub(crate) class_id: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeAuthor {
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeLogo {
    #[serde(rename = "thumbnailUrl")]
    pub(crate) thumbnail_url: Option<String>,
    pub(crate) url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeCategory {
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeFile {
    #[serde(rename = "displayName")]
    pub(crate) display_name: Option<String>,
    #[serde(rename = "gameVersions")]
    pub(crate) game_versions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeLinks {
    #[serde(rename = "websiteUrl")]
    pub(crate) website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeModDetailResponse {
    pub(crate) data: CurseForgeModDetail,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeModDetail {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) summary: String,
    pub(crate) links: Option<CurseForgeDetailLinks>,
    pub(crate) authors: Option<Vec<CurseForgeAuthor>>,
    pub(crate) logo: Option<CurseForgeLogo>,
    pub(crate) screenshots: Option<Vec<CurseForgeScreenshotItem>>,
    #[serde(rename = "classId")]
    pub(crate) class_id: Option<u32>,
    #[serde(rename = "downloadCount")]
    pub(crate) download_count: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeDetailLinks {
    pub(crate) website_url: Option<String>,
    pub(crate) wiki_url: Option<String>,
    pub(crate) issues_url: Option<String>,
    pub(crate) source_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeScreenshotItem {
    pub(crate) url: String,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeDescriptionResponse {
    pub(crate) data: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurseForgeFilesResponse {
    pub(crate) data: Vec<CurseForgeFileDetail>,
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
pub(crate) struct CurseForgeSingleFileResponse {
    pub(crate) data: CurseForgeFileDetail,
}

pub(crate) fn resolve_class_id(project_type: &str, is_bedrock: bool) -> Option<u32> {
    if is_bedrock {
        match project_type.to_lowercase().as_str() {
            "addon" | "addons" | "behaviorpack" | "behaviorpacks" => Some(4984),
            "resourcepack" | "resourcepacks" | "texturepack" | "texturepacks" => Some(6929),
            "map" | "maps" => Some(6913),
            _ => None,
        }
    } else {
        match project_type.to_lowercase().as_str() {
            "mod" | "mods" => Some(6),
            "modpack" | "modpacks" => Some(4471),
            "resourcepack" | "resourcepacks" => Some(12),
            "shader" | "shaders" | "shaderpack" | "shaderpacks" => Some(6552),
            _ => None,
        }
    }
}

pub(crate) fn resolve_mod_loader_type(loader: Option<&str>, is_bedrock: bool) -> Option<u32> {
    if is_bedrock {
        None
    } else {
        match loader.unwrap_or("").to_lowercase().as_str() {
            "forge" => Some(1),
            "fabric" => Some(4),
            "quilt" => Some(5),
            "neoforge" => Some(6),
            _ => None,
        }
    }
}

pub(crate) fn resolve_sort_field(sort: Option<&str>) -> u32 {
    match sort.unwrap_or("downloads") {
        "downloads" | "popularity" => 5,
        "updated" | "recent" => 3,
        "newest" => 1,
        "relevance" => 2,
        _ => 5,
    }
}

pub(crate) fn map_mod_item_to_unified(
    item: CurseForgeModItem,
    is_bedrock: bool,
) -> UnifiedContentItem {
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
                    if vl == "fabric" || vl == "forge" || vl == "neoforge" || vl == "quilt" {
                        if !loaders.contains(&v) {
                            loaders.push(v);
                        }
                    } else if !v.eq_ignore_ascii_case("client")
                        && !v.eq_ignore_ascii_case("server")
                        && !game_versions.contains(&v)
                    {
                        game_versions.push(v);
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
}

pub(crate) fn map_file_detail_to_unified(f: CurseForgeFileDetail) -> UnifiedContentVersion {
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
            } else if gv.chars().next().is_some_and(|c| c.is_ascii_digit())
                && !game_versions.contains(gv)
            {
                game_versions.push(gv.clone());
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

    let download_url = f.download_url.filter(|u| !u.is_empty()).unwrap_or_else(|| {
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

    UnifiedContentVersion {
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
    }
}

pub(crate) fn map_mod_detail_to_unified(
    m: CurseForgeModDetail,
    body: String,
    versions: Vec<UnifiedContentVersion>,
) -> UnifiedContentDetails {
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

    let website_url = website_url.or_else(|| {
        if m.class_id == Some(4984) || m.class_id == Some(6929) || m.class_id == Some(6913) {
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
    });

    UnifiedContentDetails {
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
        website_url,
        issues_url,
        source_url,
        wiki_url,
        screenshots,
        versions,
    }
}
