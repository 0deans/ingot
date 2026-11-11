//! FTB (Feed The Beast) public API client.
mod mapping;
mod models;

use super::{
    error::ContentError,
    models::{ContentSearchQuery, ContentSearchResult, UnifiedContentDetails},
};
use models::{FtbPackDetail, FtbPackListResponse};
use reqwest::Client;

pub const DEFAULT_FTB_BASE: &str = "https://api.modpacks.ch/public";

// Transitional re-export for `content/install/{ftb_pack,atlauncher_pack}.rs` outside this refactor slice.
pub(crate) use super::serde_helpers::{
    deserialize_opt_u32_or_string, deserialize_opt_u64_or_string, deserialize_u32_or_string,
};

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
        if !mapping::supports_project_type(query.project_type) {
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
        let slice = mapping::slice_page_ids(&pack_ids, query.offset, query.limit);
        let details = hydrate_pack_details(&self.client, &self.base_url, slice).await?;

        let items = details
            .into_iter()
            .map(mapping::map_pack_item)
            .filter(|item| mapping::matches_search_filters(item, query))
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
        let clean_id = mapping::clean_project_id(project_id);
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
        Ok(mapping::map_pack_details(pack, &self.base_url))
    }
}

async fn hydrate_pack_details(
    client: &Client,
    base_url: &str,
    pack_ids: &[u64],
) -> Result<Vec<FtbPackDetail>, ContentError> {
    let mut set = tokio::task::JoinSet::new();
    for (idx, &pack_id) in pack_ids.iter().enumerate() {
        let detail_url = format!("{base_url}/modpack/{pack_id}");
        let client = client.clone();
        set.spawn(async move {
            let res = client
                .get(detail_url)
                .send()
                .await
                .map_err(ContentError::Transport)?;
            if !res.status().is_success() {
                return Err(ContentError::HttpStatus(res.status()));
            }
            let pack = res
                .json::<FtbPackDetail>()
                .await
                .map_err(ContentError::Response)?;
            Ok::<_, ContentError>((idx, pack))
        });
    }

    let mut details: Vec<Option<FtbPackDetail>> = (0..pack_ids.len()).map(|_| None).collect();
    while let Some(res) = set.join_next().await {
        let (idx, pack) = res.map_err(ContentError::Task)??;
        if let Some(slot) = details.get_mut(idx) {
            *slot = Some(pack);
        }
    }

    Ok(details.into_iter().flatten().collect())
}
