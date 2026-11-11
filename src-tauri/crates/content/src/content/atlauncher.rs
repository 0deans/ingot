//! ATLauncher public API client.
mod mapping;
mod models;

use super::{
    error::ContentError,
    models::{ContentSearchQuery, ContentSearchResult, UnifiedContentDetails},
};
#[cfg(test)]
pub(super) use mapping::compute_atlauncher_safe_name;
use models::{AtLauncherPackDetailResponse, AtLauncherPackListResponse, AtLauncherPackSummary};
use reqwest::{Client, StatusCode};

pub const DEFAULT_ATLAUNCHER_BASE: &str = "https://download.nodecdn.net/containers/atl";

fn is_missing_endpoint_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::FORBIDDEN | StatusCode::GONE
    )
}

#[derive(Clone)]
pub struct AtLauncherClient {
    client: Client,
    base_url: String,
}

impl AtLauncherClient {
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

        let packs = self.fetch_pack_list().await?;
        Ok(mapping::search_packs(packs, query, &self.base_url))
    }

    pub async fn get_details(
        &self,
        project_id: &str,
    ) -> Result<UnifiedContentDetails, ContentError> {
        let clean_id = mapping::clean_project_id(project_id);
        let detail_url = format!("{}/pack/{}", self.base_url, clean_id);
        let response = self
            .client
            .get(&detail_url)
            .send()
            .await
            .map_err(ContentError::Transport)?;

        if response.status().is_success() {
            let detail = response
                .json::<AtLauncherPackDetailResponse>()
                .await
                .map_err(ContentError::Response)?
                .into_detail();
            return Ok(mapping::map_pack_detail_response(
                detail,
                clean_id,
                &self.base_url,
            ));
        }

        if !is_missing_endpoint_status(response.status()) {
            return Err(ContentError::HttpStatus(response.status()));
        }

        let packsnew_url = format!("{}/launcher/json/packsnew.json", self.base_url);
        let pn_res = self
            .client
            .get(&packsnew_url)
            .send()
            .await
            .map_err(ContentError::Transport)?;

        if !pn_res.status().is_success() {
            return Err(ContentError::HttpStatus(pn_res.status()));
        }

        let packs = pn_res
            .json::<AtLauncherPackListResponse>()
            .await
            .map_err(ContentError::Response)?
            .into_vec();

        let pack = mapping::find_pack_by_id(packs, clean_id)
            .ok_or(ContentError::HttpStatus(StatusCode::NOT_FOUND))?;

        Ok(mapping::map_pack_summary_to_details(pack, &self.base_url))
    }

    async fn fetch_pack_list(&self) -> Result<Vec<AtLauncherPackSummary>, ContentError> {
        let candidate_paths = ["/launcher/json/packsnew.json", "/packs/simple", "/packs"];
        for (idx, path) in candidate_paths.iter().enumerate() {
            let url = format!("{}{}", self.base_url, path);
            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(ContentError::Transport)?;
            if response.status().is_success() {
                return response
                    .json::<AtLauncherPackListResponse>()
                    .await
                    .map(AtLauncherPackListResponse::into_vec)
                    .map_err(ContentError::Response);
            }
            let is_last = idx + 1 == candidate_paths.len();
            if is_last || !is_missing_endpoint_status(response.status()) {
                return Err(ContentError::HttpStatus(response.status()));
            }
        }
        Err(ContentError::HttpStatus(StatusCode::NOT_FOUND))
    }
}
