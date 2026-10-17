use super::super::{
    models::{
        platform, PluginError, PluginProject, PluginSearchResult, PluginSource, PluginVersion,
    },
    providers::{
        client, get_json, hangar_project, hangar_version, modrinth_version, n, required_array,
        required_string, s, strings, PluginProvider, HANGAR, MODRINTH,
    },
};
use crate::server::config::ServerCoreType;
use serde_json::Value;
#[allow(clippy::too_many_arguments)]
pub async fn search(
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    query: &str,
    sort: &str,
    compatible_only: bool,
    page: u32,
    page_size: u32,
) -> Result<PluginSearchResult, PluginError> {
    let platform = platform(core).ok_or(PluginError::UnsupportedCore)?;
    let client = client()?;
    let offset = (page * page_size).to_string();
    let limit = page_size.to_string();

    match source {
        PluginSource::Modrinth => {
            let mut facets = vec![
                vec![format!("project_type:{}", platform.modrinth_type)],
                platform
                    .modrinth_loaders
                    .iter()
                    .map(|l| format!("categories:{l}"))
                    .collect(),
            ];
            if platform.modrinth_type == "mod" {
                facets.push(vec![
                    "server_side:required".into(),
                    "server_side:optional".into(),
                ]);
            }
            if compatible_only {
                facets.push(vec![format!("versions:{game_version}")]);
            }
            let index = match sort {
                "relevance" => "relevance",
                "updated" => "updated",
                "newest" => "newest",
                _ if !query.trim().is_empty() => "relevance",
                _ => "downloads",
            };
            let mut params = vec![
                ("facets", serde_json::to_string(&facets)?),
                ("index", index.to_string()),
                ("offset", offset),
                ("limit", limit),
            ];
            if !query.trim().is_empty() {
                params.push(("query", query.trim().to_string()));
            }
            let res = get_json(&client, &format!("{MODRINTH}/search"), &params).await?;
            let hits = required_array(&res, "hits")?;
            for hit in hits {
                required_string(hit, "project_id")?;
                required_string(hit, "title")?;
            }
            let items = hits
                .iter()
                .map(|h| PluginProject {
                    id: s(h, "project_id"),
                    source: PluginSource::Modrinth,
                    slug: s(h, "slug"),
                    title: s(h, "title"),
                    description: s(h, "description"),
                    author: s(h, "author"),
                    icon_url: h
                        .get("icon_url")
                        .and_then(Value::as_str)
                        .filter(|u| !u.is_empty())
                        .map(str::to_string),
                    downloads: n(h, "downloads"),
                    categories: strings(h, "display_categories"),
                    page_url: format!(
                        "https://modrinth.com/{}/{}",
                        platform.modrinth_type,
                        s(h, "slug")
                    ),
                    players_need_it: platform.modrinth_type == "mod"
                        && s(h, "client_side") == "required",
                })
                .collect();
            Ok(PluginSearchResult {
                items,
                total: n(&res, "total_hits"),
            })
        }
        PluginSource::Hangar => {
            if !platform.hangar {
                return Ok(PluginSearchResult {
                    items: Vec::new(),
                    total: 0,
                });
            }
            let sort = match sort {
                "updated" => "-updated",
                "newest" => "-newest",
                "stars" => "-stars",
                _ => "-downloads",
            };
            let mut params = vec![
                ("platform", "PAPER".to_string()),
                ("sort", sort.to_string()),
                ("offset", offset),
                ("limit", limit),
            ];
            if !query.trim().is_empty() {
                params.push(("q", query.trim().to_string()));
            }
            if compatible_only {
                params.push(("version", game_version.to_string()));
            }
            let res = get_json(&client, &format!("{HANGAR}/projects"), &params).await?;
            let projects = required_array(&res, "result")?;
            for project in projects {
                required_string(project, "name")?;
            }
            let items = projects.iter().map(hangar_project).collect();
            let total = res.get("pagination").map(|p| n(p, "count")).unwrap_or(0);
            Ok(PluginSearchResult { items, total })
        }
    }
}

pub async fn versions(
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    compatible_only: bool,
) -> Result<Vec<PluginVersion>, PluginError> {
    let provider = PluginProvider::new()?;
    versions_with_provider(
        core,
        game_version,
        source,
        project_id,
        compatible_only,
        &provider,
    )
    .await
}

pub(crate) async fn versions_with_provider(
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    compatible_only: bool,
    provider: &PluginProvider,
) -> Result<Vec<PluginVersion>, PluginError> {
    let platform = platform(core).ok_or(PluginError::UnsupportedCore)?;
    let client = &provider.client;
    match source {
        PluginSource::Modrinth => {
            let mut params = vec![("loaders", serde_json::to_string(platform.modrinth_loaders)?)];
            if compatible_only {
                params.push(("game_versions", serde_json::to_string(&[game_version])?));
            }
            let res = get_json(
                client,
                &format!("{}/project/{project_id}/version", provider.modrinth),
                &params,
            )
            .await?;
            let values = res
                .as_array()
                .ok_or(PluginError::InvalidPayload("versions"))?;
            values
                .iter()
                .map(|value| {
                    required_string(value, "id")?;
                    let version = modrinth_version(value)
                        .ok_or(PluginError::InvalidPayload("version files"))?;
                    if version
                        .download_url
                        .as_ref()
                        .is_some_and(|url| !url.is_empty())
                        && version.file_name.is_empty()
                    {
                        return Err(PluginError::InvalidPayload("version filename"));
                    }
                    Ok(version)
                })
                .collect()
        }
        PluginSource::Hangar => {
            let mut params = vec![
                ("platform", "PAPER".to_string()),
                ("limit", "25".to_string()),
            ];
            if compatible_only {
                params.push(("platformVersion", game_version.to_string()));
            }
            let res = get_json(
                client,
                &format!("{}/projects/{project_id}/versions", provider.hangar),
                &params,
            )
            .await?;
            let values = required_array(&res, "result")?;
            values
                .iter()
                .map(|value| {
                    required_string(value, "name")?;
                    hangar_version(value)
                        .ok_or(PluginError::InvalidPayload("Hangar platform download"))
                })
                .collect()
        }
    }
}
