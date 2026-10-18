//! CurseForge API client for search and mod/addon details retrieval.

pub(crate) mod models;

use super::{
    error::ContentError,
    models::{ContentSearchQuery, ContentSearchResult, UnifiedContentDetails},
};
pub use models::CurseForgeFileDetail;
use models::*;
use reqwest::Client;

pub const CURSEFORGE_API_KEY: &str = "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm";
pub const DEFAULT_CURSEFORGE_BASE: &str = "https://api.curseforge.com/v1";

pub struct CurseForgeClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl CurseForgeClient {
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
        let class_id = resolve_class_id(query.project_type, is_bedrock);
        let mod_loader_type = resolve_mod_loader_type(query.loader, is_bedrock);
        let sort_field = resolve_sort_field(query.sort);

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
            .map(|item| map_mod_item_to_unified(item, is_bedrock))
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
        let mod_id = self.resolve_project_id(project_id).await?;
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
                .unwrap_or_else(|_| m.summary.clone()),
            _ => m.summary.clone(),
        };

        let response = files_res.map_err(ContentError::Transport)?;
        if !response.status().is_success() {
            return Err(ContentError::HttpStatus(response.status()));
        }
        let files = response
            .json::<CurseForgeFilesResponse>()
            .await
            .map_err(ContentError::Response)?;
        let versions = files
            .data
            .into_iter()
            .map(map_file_detail_to_unified)
            .collect();

        Ok(map_mod_detail_to_unified(m, body, versions))
    }

    async fn resolve_project_id(&self, project_id: &str) -> Result<u32, ContentError> {
        if let Ok(id) = project_id.parse::<u32>() {
            return Ok(id);
        }

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
            return Ok(id);
        }

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
                ContentError::UnknownSource(format!("CurseForge mod '{project_id}' not found"))
            })
        } else {
            Err(ContentError::UnknownSource(format!(
                "Invalid CurseForge mod ID or slug: {project_id}"
            )))
        }
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
