//! Technic Platform and Solder API client.
mod mapping;
mod models;

use super::{
    error::ContentError,
    models::{ContentSearchQuery, ContentSearchResult, UnifiedContentDetails, UnifiedContentItem},
};
pub(crate) use mapping::extract_technic_slug_from_query;
use models::{SolderPackResponse, TechnicPackDetail, TechnicSearchEntry, TechnicSearchResponse};
use reqwest::Client;

pub const DEFAULT_TECHNIC_BASE: &str = "https://api.technicpack.net";

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
        if !mapping::supports_project_type(query.project_type) {
            return Ok(ContentSearchResult {
                items: Vec::new(),
                total_hits: 0,
                offset: query.offset,
                limit: query.limit,
            });
        }

        if let Some(slug) = query.query.and_then(extract_technic_slug_from_query) {
            let item = self.get_pack_item(slug).await?;
            return Ok(ContentSearchResult {
                items: vec![item],
                total_hits: 1,
                offset: 0,
                limit: query.limit,
            });
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
        let slice = mapping::slice_page_entries(&entries, query.offset, query.limit);
        let hydrated = hydrate_search_entries(&self.client, &self.base_url, slice).await?;

        let items = hydrated
            .into_iter()
            .map(|(entry, real_slug, detail_opt)| {
                mapping::map_hydrated_entry_to_item(entry, real_slug, detail_opt)
            })
            .collect();

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
        let clean_slug = mapping::clean_pack_slug(project_id);
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
        let solder_data = if let Some(solder_url) = pack
            .solder
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let solder_endpoint =
                format!("{}/modpack/{}", solder_url.trim_end_matches('/'), pack.name);
            let solder_res = self
                .client
                .get(solder_endpoint)
                .send()
                .await
                .map_err(ContentError::Transport)?;
            if !solder_res.status().is_success() {
                return Err(ContentError::HttpStatus(solder_res.status()));
            }
            let parsed = solder_res
                .json::<SolderPackResponse>()
                .await
                .map_err(ContentError::Response)?;
            Some(parsed)
        } else {
            None
        };

        mapping::map_pack_details(pack, solder_data)
    }

    async fn get_pack_item(&self, slug: &str) -> Result<UnifiedContentItem, ContentError> {
        let details = self.get_details(slug).await?;
        Ok(mapping::details_to_item(details))
    }
}

async fn hydrate_search_entries(
    client: &Client,
    base_url: &str,
    entries: &[TechnicSearchEntry],
) -> Result<Vec<(TechnicSearchEntry, String, Option<TechnicPackDetail>)>, ContentError> {
    let mut set = tokio::task::JoinSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let real_slug = mapping::resolve_entry_slug(entry);
        let entry_clone = entry.clone();
        let base_url = base_url.to_string();
        let client = client.clone();
        set.spawn(async move {
            if entry_clone.description.is_none() {
                let url = format!("{base_url}/modpack/{real_slug}?build=ingot");
                let res = client
                    .get(url)
                    .send()
                    .await
                    .map_err(ContentError::Transport)?;
                if !res.status().is_success() {
                    return Err(ContentError::HttpStatus(res.status()));
                }
                let detail = res
                    .json::<TechnicPackDetail>()
                    .await
                    .map_err(ContentError::Response)?;
                return Ok::<_, ContentError>((idx, entry_clone, real_slug, Some(detail)));
            }
            Ok((idx, entry_clone, real_slug, None))
        });
    }

    let mut hydrated: Vec<Option<(TechnicSearchEntry, String, Option<TechnicPackDetail>)>> =
        (0..entries.len()).map(|_| None).collect();
    while let Some(res) = set.join_next().await {
        let (idx, entry, real_slug, detail_opt) = res.map_err(ContentError::Task)??;
        if let Some(slot) = hydrated.get_mut(idx) {
            *slot = Some((entry, real_slug, detail_opt));
        }
    }

    Ok(hydrated.into_iter().flatten().collect())
}
