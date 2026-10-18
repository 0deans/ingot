use super::{
    parse_content_source_and_id, ContentError, ContentSearchQuery, ContentSearchResult,
    CurseForgeClient, ModrinthClient, UnifiedContentDetails,
};

pub(crate) struct ContentSearchRequest {
    pub(crate) source: String,
    pub(crate) project_type: String,
    pub(crate) query: Option<String>,
    pub(crate) game_version: Option<String>,
    pub(crate) loader: Option<String>,
    pub(crate) sort: Option<String>,
    pub(crate) page: u32,
    pub(crate) page_size: u32,
}

pub(crate) struct ContentService {
    modrinth: ModrinthClient,
    curseforge: CurseForgeClient,
}

impl ContentService {
    pub(crate) fn new(client: reqwest::Client) -> Self {
        Self::with_providers(
            ModrinthClient::with_endpoint(client.clone(), super::modrinth::DEFAULT_MODRINTH_BASE),
            CurseForgeClient::with_endpoint(
                client,
                super::curseforge::DEFAULT_CURSEFORGE_BASE,
                super::CURSEFORGE_API_KEY,
            ),
        )
    }

    pub(crate) fn with_providers(modrinth: ModrinthClient, curseforge: CurseForgeClient) -> Self {
        Self {
            modrinth,
            curseforge,
        }
    }

    /// Combined search requires both providers: the current response has no partial-result status.
    pub(crate) async fn search_content(
        &self,
        request: ContentSearchRequest,
    ) -> Result<ContentSearchResult, ContentError> {
        let ContentSearchRequest {
            source,
            project_type,
            query,
            game_version,
            loader,
            sort,
            page,
            page_size,
        } = request;
        let offset = page
            .checked_mul(page_size)
            .ok_or(ContentError::InvalidPagination)?;
        let src_clean = source.trim().to_lowercase();

        let is_bedrock = loader
            .as_deref()
            .map(|l| l.eq_ignore_ascii_case("bedrock"))
            .unwrap_or(false)
            || project_type.eq_ignore_ascii_case("bedrock")
            || project_type.eq_ignore_ascii_case("addon")
            || project_type.eq_ignore_ascii_case("addons")
            || project_type.eq_ignore_ascii_case("behaviorpack");

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
            return self.curseforge.search(&search_query).await;
        }

        if src_clean == "modrinth" {
            return self.modrinth.search(&search_query).await;
        }

        // "all": query both in parallel
        let mut mr_query = search_query.clone();
        mr_query.offset = offset / 2;
        let mut cf_query = search_query;
        cf_query.offset = offset / 2;

        let (res_mr, res_cf) = tokio::join!(
            self.modrinth.search(&mr_query),
            self.curseforge.search(&cf_query)
        );

        let mut combined_items = Vec::new();
        let mut total_hits = 0;

        let (mr, cf) = match (res_mr, res_cf) {
            (Ok(mr), Ok(cf)) => (mr, cf),
            (Err(modrinth), Err(curseforge)) => {
                return Err(ContentError::ProvidersUnavailable {
                    modrinth: Box::new(modrinth),
                    curseforge: Box::new(curseforge),
                })
            }
            (Err(source), _) => {
                return Err(ContentError::Provider {
                    provider: "modrinth",
                    source: Box::new(source),
                })
            }
            (_, Err(source)) => {
                return Err(ContentError::Provider {
                    provider: "curseforge",
                    source: Box::new(source),
                })
            }
        };
        total_hits += mr.total_hits;
        total_hits += cf.total_hits;
        combined_items.extend(mr.items);
        combined_items.extend(cf.items);

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
    pub(crate) async fn get_content_details(
        &self,
        source: &str,
        project_id: &str,
    ) -> Result<UnifiedContentDetails, ContentError> {
        let (detected_source, clean_id) = parse_content_source_and_id(source, project_id);
        let src = detected_source.to_lowercase();

        if src == "modrinth" {
            self.modrinth.get_details(clean_id).await
        } else if src == "curseforge" {
            self.curseforge.get_details(clean_id).await
        } else {
            Err(ContentError::UnknownSource(source.to_string()))
        }
    }
}
