use super::models::{PluginDependency, PluginProject, PluginSource, PluginVersion};
use crate::USER_AGENT;
use serde_json::Value;

pub const MODRINTH: &str = "https://api.modrinth.com/v2";
pub const HANGAR: &str = "https://hangar.papermc.io/api/v1";

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub async fn get_json(
    client: &reqwest::Client,
    url: &str,
    query: &[(&str, String)],
) -> Result<Value, String> {
    let res = client
        .get(url)
        .query(query)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("{url} returned HTTP {}", res.status()));
    }
    res.json()
        .await
        .map_err(|e| format!("Invalid response from {url}: {e}"))
}

pub fn s(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn n(v: &Value, key: &str) -> u32 {
    v.get(key)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u32::MAX as u64) as u32
}

pub fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn hangar_project(p: &Value) -> PluginProject {
    let ns = p.get("namespace").cloned().unwrap_or(Value::Null);
    let (owner, slug) = (s(&ns, "owner"), s(&ns, "slug"));
    PluginProject {
        id: slug.clone(),
        source: PluginSource::Hangar,
        slug: slug.clone(),
        title: s(p, "name"),
        description: s(p, "description"),
        author: owner.clone(),
        icon_url: p
            .get("avatarUrl")
            .and_then(Value::as_str)
            .map(str::to_string),
        downloads: p.get("stats").map(|st| n(st, "downloads")).unwrap_or(0),
        categories: vec![s(p, "category").replace('_', " ")],
        page_url: format!("https://hangar.papermc.io/{owner}/{slug}"),
        players_need_it: false,
    }
}

pub fn modrinth_version(v: &Value) -> Option<PluginVersion> {
    let files = v.get("files")?.as_array()?;
    let file = files
        .iter()
        .find(|f| f.get("primary").and_then(Value::as_bool) == Some(true))
        .or_else(|| files.first())?;
    let dependencies = v
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|deps| {
            deps.iter()
                .filter(|d| s(d, "dependency_type") == "required")
                .filter_map(|d| {
                    let id = d.get("project_id").and_then(Value::as_str)?;
                    Some(PluginDependency {
                        project_id: id.to_string(),
                        name: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(PluginVersion {
        id: s(v, "id"),
        version_number: s(v, "version_number"),
        channel: v
            .get("version_type")
            .and_then(Value::as_str)
            .unwrap_or("release")
            .to_string(),
        game_versions: strings(v, "game_versions"),
        date: s(v, "date_published"),
        downloads: n(v, "downloads"),
        file_name: s(file, "filename"),
        download_url: file.get("url").and_then(Value::as_str).map(str::to_string),
        external_url: None,
        size: n(file, "size"),
        dependencies,
    })
}

pub fn hangar_version(v: &Value) -> Option<PluginVersion> {
    let download = v.get("downloads")?.get("PAPER")?;
    let file = download.get("fileInfo");
    let channel = v
        .get("channel")
        .map(|c| s(c, "name").to_lowercase())
        .map(|c| {
            if c == "release" {
                c
            } else {
                "beta".to_string()
            }
        })
        .unwrap_or_else(|| "release".to_string());
    let dependencies = v
        .get("pluginDependencies")
        .and_then(|d| d.get("PAPER"))
        .and_then(Value::as_array)
        .map(|deps| {
            deps.iter()
                .filter(|d| d.get("required").and_then(Value::as_bool) == Some(true))
                .filter(|d| d.get("externalUrl").is_none_or(Value::is_null))
                .map(|d| PluginDependency {
                    project_id: s(d, "name"),
                    name: Some(s(d, "name")),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(PluginVersion {
        id: s(v, "name"),
        version_number: s(v, "name"),
        channel,
        game_versions: v
            .get("platformDependencies")
            .map(|p| strings(p, "PAPER"))
            .unwrap_or_default(),
        date: s(v, "createdAt"),
        downloads: v
            .get("stats")
            .map(|st| n(st, "totalDownloads"))
            .unwrap_or(0),
        file_name: file.map(|f| s(f, "name")).unwrap_or_default(),
        download_url: download
            .get("downloadUrl")
            .and_then(Value::as_str)
            .map(str::to_string),
        external_url: download
            .get("externalUrl")
            .and_then(Value::as_str)
            .map(str::to_string),
        size: file.map(|f| n(f, "sizeBytes")).unwrap_or(0),
        dependencies,
    })
}

/// Newest stable version, falling back to pre-releases
pub fn pick_version(versions: &[PluginVersion]) -> Option<&PluginVersion> {
    let installable = || versions.iter().filter(|v| v.download_url.is_some());
    installable()
        .find(|v| v.channel == "release")
        .or_else(|| installable().next())
}

/// Full description (Markdown) of a project
pub async fn page(source: PluginSource, project_id: &str) -> Result<String, String> {
    let client = client()?;
    match source {
        PluginSource::Modrinth => {
            let project =
                get_json(&client, &format!("{MODRINTH}/project/{project_id}"), &[]).await?;
            Ok(s(&project, "body"))
        }
        PluginSource::Hangar => {
            let res = client
                .get(format!("{HANGAR}/pages/main/{project_id}"))
                .send()
                .await
                .map_err(|e| format!("Request failed: {e}"))?;
            res.text().await.map_err(|e| e.to_string())
        }
    }
}

pub async fn project_title(
    client: &reqwest::Client,
    source: PluginSource,
    id: &str,
) -> (String, Option<String>) {
    let url = match source {
        PluginSource::Modrinth => format!("{MODRINTH}/project/{id}"),
        PluginSource::Hangar => format!("{HANGAR}/projects/{id}"),
    };
    match get_json(client, &url, &[]).await {
        Ok(p) => match source {
            PluginSource::Modrinth => (
                s(&p, "title"),
                p.get("icon_url")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            ),
            PluginSource::Hangar => (
                s(&p, "name"),
                p.get("avatarUrl")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            ),
        },
        Err(_) => (id.to_string(), None),
    }
}
