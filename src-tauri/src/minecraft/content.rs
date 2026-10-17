//! Unified Minecraft content discovery, details retrieval, and installation coordinator.

pub(crate) mod curseforge;
pub(crate) mod error;
pub(crate) mod install;
pub(crate) mod models;
pub(crate) mod modrinth;
#[cfg(test)]
mod tests;

pub use curseforge::{CurseForgeClient, CURSEFORGE_API_KEY};
pub use error::ContentError;
pub use models::{
    ContentScreenshot, ContentSearchQuery, ContentSearchResult, UnifiedContentDetails,
    UnifiedContentItem, UnifiedContentVersion,
};
pub use modrinth::ModrinthClient;

use tauri::Runtime;

/// Extracts the content source prefix and clean project identifier.
pub(crate) fn parse_content_source_and_id<'a>(
    source: &'a str,
    project_id: &'a str,
) -> (&'a str, &'a str) {
    let trimmed_id = project_id.trim();
    if let Some(stripped) = trimmed_id.strip_prefix("mr:") {
        ("modrinth", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("cf:") {
        ("curseforge", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("modrinth:") {
        ("modrinth", stripped)
    } else if let Some(stripped) = trimmed_id.strip_prefix("curseforge:") {
        ("curseforge", stripped)
    } else if !source.trim().is_empty() {
        (source.trim(), trimmed_id)
    } else {
        ("", trimmed_id)
    }
}

/// Searches content across Modrinth and CurseForge with unified filters and pagination.
#[allow(clippy::too_many_arguments)]
pub async fn search_content(
    source: String,
    project_type: String,
    query: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    sort: Option<String>,
    page: u32,
    page_size: u32,
) -> Result<ContentSearchResult, ContentError> {
    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(ContentError::ClientInit)?;

    let offset = page * page_size;
    let src_clean = source.trim().to_lowercase();

    let is_bedrock = loader
        .as_deref()
        .map(|l| l.eq_ignore_ascii_case("bedrock"))
        .unwrap_or(false)
        || project_type.eq_ignore_ascii_case("bedrock")
        || project_type.eq_ignore_ascii_case("addon")
        || project_type.eq_ignore_ascii_case("addons")
        || project_type.eq_ignore_ascii_case("behaviorpack");

    let mr_client = ModrinthClient::with_endpoint(client.clone(), modrinth::DEFAULT_MODRINTH_BASE);
    let cf_client = CurseForgeClient::with_endpoint(
        client.clone(),
        curseforge::DEFAULT_CURSEFORGE_BASE,
        CURSEFORGE_API_KEY,
    );

    let search_query = ContentSearchQuery {
        project_type: &project_type,
        query: query.as_deref(),
        game_version: game_version.as_deref(),
        loader: loader.as_deref(),
        sort: sort.as_deref(),
        offset,
        limit: page_size,
    };

    if is_bedrock || src_clean == "curseforge" {
        return cf_client.search(&search_query).await;
    }

    if src_clean == "modrinth" {
        return mr_client.search(&search_query).await;
    }

    // "all": query both in parallel
    let mut mr_query = search_query.clone();
    mr_query.offset = offset / 2;
    let mut cf_query = search_query;
    cf_query.offset = offset / 2;

    let (res_mr, res_cf) = tokio::join!(mr_client.search(&mr_query), cf_client.search(&cf_query));

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

    combined_items.sort_by_key(|a| std::cmp::Reverse(a.downloads));
    combined_items.truncate(page_size as usize);

    Ok(ContentSearchResult {
        items: combined_items,
        total_hits,
        offset,
        limit: page_size,
    })
}

/// Retrieves comprehensive details and version history for a project.
pub async fn get_content_details(
    source: &str,
    project_id: &str,
) -> Result<UnifiedContentDetails, ContentError> {
    let (detected_source, clean_id) = parse_content_source_and_id(source, project_id);
    let src = detected_source.to_lowercase();

    if src == "modrinth" {
        let client = ModrinthClient::new();
        client.get_details(clean_id).await
    } else if src == "curseforge" {
        let client = CurseForgeClient::new();
        client.get_details(clean_id).await
    } else {
        Err(ContentError::UnknownSource(source.to_string()))
    }
}

/// Installs a single content file (mod, resourcepack, shaderpack) into an instance directory.
pub async fn install_content_file<R: Runtime>(
    app: &tauri::AppHandle<R>,
    instance_id: &str,
    project_type: &str,
    download_url: &str,
    filename: &str,
) -> Result<String, ContentError> {
    install::install_content_file(app, instance_id, project_type, download_url, filename).await
}

/// Installs a full modpack archive (.mrpack or CurseForge zip) into a new instance.
pub async fn install_modpack_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    name: &str,
    source: &str,
    download_url: &str,
    filename: &str,
) -> Result<crate::minecraft::instance::InstanceConfig, ContentError> {
    install::install_modpack_instance(app, name, source, download_url, filename).await
}
