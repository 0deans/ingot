//! Ely.by skin catalog search and query formatting.
use super::{
    error::ElyError,
    models::{url_encode, ElyRawCatalogResponse, ElySkinItem, ElySkinsCatalogResponse},
};
use reqwest::Client;

pub(crate) fn parse_slim_filter(model: Option<&str>) -> Option<bool> {
    model.and_then(|m| match m.trim().to_lowercase().as_str() {
        "slim" | "alex" => Some(true),
        "steve" | "classic" | "new" => Some(false),
        _ => None,
    })
}

pub(crate) fn build_catalog_query(
    page: u32,
    query: Option<&str>,
    sort: Option<&str>,
    uploader: Option<&str>,
) -> String {
    let mut query_params: Vec<String> = Vec::new();

    if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            if words.is_empty() {
                query_params.push(format!("tags[0]={}", url_encode(trimmed)));
            } else {
                for (idx, word) in words.iter().enumerate() {
                    query_params.push(format!("tags[{idx}]={}", url_encode(word)));
                }
            }
        }
    }

    if let Some(s) = sort {
        let trimmed = s.trim().to_lowercase();
        if trimmed == "latest" || trimmed == "time" {
            query_params.push("sort=time".to_string());
        } else if !trimmed.is_empty() && trimmed != "default" {
            query_params.push("sort=best".to_string());
        }
    }

    if let Some(u) = uploader {
        let trimmed = u.trim();
        if !trimmed.is_empty() {
            query_params.push(format!("uploader={}", url_encode(trimmed)));
        }
    }

    query_params.push(format!("page={page}"));
    query_params.join("&")
}

pub(crate) async fn fetch_catalog(
    client: &Client,
    base_url: &str,
    page: u32,
    query: Option<String>,
    sort: Option<String>,
    model: Option<String>,
    uploader: Option<String>,
) -> Result<ElySkinsCatalogResponse, ElyError> {
    let query_string =
        build_catalog_query(page, query.as_deref(), sort.as_deref(), uploader.as_deref());
    let slim_filter = parse_slim_filter(model.as_deref());

    let target_url = format!("{base_url}/api/legacy/skins?{query_string}");
    let res = client
        .get(&target_url)
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(ElyError::Transport)?;

    if !res.status().is_success() {
        return Err(ElyError::HttpStatus(res.status()));
    }

    let raw = res
        .json::<ElyRawCatalogResponse>()
        .await
        .map_err(ElyError::ParseCatalog)?;

    let items = raw
        .items
        .into_iter()
        .filter(|i| slim_filter.is_none_or(|slim| i.is_slim == slim))
        .map(|i| ElySkinItem {
            id: i.id,
            skin_url: i.skin_url,
            is_slim: i.is_slim,
            count_wearers: i.count_wearers,
            count_cubes: i.count_cubes,
            count_views: i.count_views_total,
            tags: i.tags,
        })
        .collect();

    Ok(ElySkinsCatalogResponse {
        items,
        total_items: raw.total_items.unwrap_or(0),
        current_page: raw.current.unwrap_or(page),
        last_page: raw.last.unwrap_or(page),
    })
}
