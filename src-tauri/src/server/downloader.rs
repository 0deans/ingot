use crate::minecraft::downloader::download_file_chunked;
use crate::server::config::ServerCoreType;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct PaperProjectsResponseV3 {
    versions: indexmap::IndexMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PaperBuildResponseV3 {
    id: u32,
    #[serde(default)]
    downloads: std::collections::HashMap<String, PaperDownloadItemV3>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PaperDownloadItemV3 {
    #[serde(default)]
    name: Option<String>,
    url: String,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PurpurVersionsResponse {
    versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PurpurBuildsResponse {
    builds: PurpurBuildsInfo,
}

#[derive(Debug, Deserialize)]
struct PurpurBuildsInfo {
    latest: String,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

#[derive(Debug, Deserialize)]
struct MojangVersionManifest {
    versions: Vec<MojangVersionEntry>,
}

#[derive(Debug, Deserialize)]
struct MojangVersionEntry {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct MojangVersionPackage {
    downloads: Option<MojangDownloads>,
}

#[derive(Debug, Deserialize)]
struct MojangDownloads {
    server: Option<MojangDownloadFile>,
}

#[derive(Debug, Deserialize)]
struct MojangDownloadFile {
    url: String,
    sha1: Option<String>,
}

/// Fetches supported Minecraft versions for a given server core
pub async fn fetch_core_versions(
    client: &reqwest::Client,
    core: &ServerCoreType,
) -> Result<Vec<String>, String> {
    match core {
        ServerCoreType::Paper | ServerCoreType::Folia => {
            let project = if *core == ServerCoreType::Folia {
                "folia"
            } else {
                "paper"
            };
            let url = format!("https://fill.papermc.io/v3/projects/{}", project);
            let res = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("Failed to reach {project} API: {e}"))?;
            let data: PaperProjectsResponseV3 = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse {project} API response: {e}"))?;

            let mut list = Vec::new();
            for group in data.versions.values() {
                for ver in group {
                    list.push(ver.clone());
                }
            }
            Ok(list)
        }
        ServerCoreType::Purpur => {
            let res = client
                .get("https://api.purpurmc.org/v2/purpur")
                .send()
                .await
                .map_err(|e| format!("Failed to reach Purpur API: {e}"))?;
            let data: PurpurVersionsResponse = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse Purpur API: {e}"))?;
            let mut list = data.versions;
            list.reverse();
            Ok(list)
        }
        ServerCoreType::Fabric => {
            let res = client
                .get("https://meta.fabricmc.net/v2/versions/game")
                .send()
                .await
                .map_err(|e| format!("Failed to reach Fabric Meta API: {e}"))?;
            let data: Vec<serde_json::Value> = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse Fabric Meta: {e}"))?;

            let mut versions = Vec::new();
            for item in data {
                if item.get("stable").and_then(|v| v.as_bool()).unwrap_or(false) {
                    if let Some(ver) = item.get("version").and_then(|v| v.as_str()) {
                        versions.push(ver.to_string());
                    }
                }
            }
            Ok(versions)
        }
        ServerCoreType::Vanilla => {
            let res = client
                .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Mojang manifest: {e}"))?;
            let manifest: MojangVersionManifest = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse Mojang manifest: {e}"))?;

            let versions = manifest
                .versions
                .into_iter()
                .filter(|v| v.version_type == "release")
                .map(|v| v.id)
                .collect();
            Ok(versions)
        }
    }
}

/// Downloads and installs server.jar into server_dir if not already present
pub async fn ensure_server_jar(
    client: &reqwest::Client,
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    build_number: Option<&str>,
) -> Result<String, String> {
    let dest_jar = server_dir.join("server.jar");
    if dest_jar.exists() {
        return Ok("Server core already downloaded".to_string());
    }

    match core {
        ServerCoreType::Paper | ServerCoreType::Folia => {
            let project = if *core == ServerCoreType::Folia {
                "folia"
            } else {
                "paper"
            };

            let build_target = build_number.unwrap_or("latest");
            let build_url = format!(
                "https://fill.papermc.io/v3/projects/{}/versions/{}/builds/{}",
                project, game_version, build_target
            );

            let b_resp = client
                .get(&build_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch {} build info for {}: {e}", project, game_version))?;

            if !b_resp.status().is_success() {
                return Err(format!(
                    "{} API returned status {} for version {}",
                    project,
                    b_resp.status(),
                    game_version
                ));
            }

            let b_data: PaperBuildResponseV3 = b_resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse build info: {e}"))?;

            let dl_entry = b_data
                .downloads
                .get("server:default")
                .or_else(|| b_data.downloads.get("application"))
                .or_else(|| b_data.downloads.values().next())
                .ok_or_else(|| format!("No server download entry found for {} build #{}", project, b_data.id))?;

            download_file_chunked(client, &dl_entry.url, &dest_jar, None, None).await?;
            Ok(format!("{} build #{} installed", core, b_data.id))
        }

        ServerCoreType::Purpur => {
            let build = if let Some(b) = build_number {
                b.to_string()
            } else {
                let info_url = format!("https://api.purpurmc.org/v2/purpur/{}", game_version);
                let resp = client
                    .get(&info_url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to fetch Purpur builds for {}: {e}", game_version))?;
                let data: PurpurBuildsResponse = resp
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse Purpur builds: {e}"))?;
                data.builds.latest
            };

            let download_url = format!(
                "https://api.purpurmc.org/v2/purpur/{}/{}/download",
                game_version, build
            );

            download_file_chunked(client, &download_url, &dest_jar, None, None).await?;
            Ok(format!("Purpur build #{} installed", build))
        }

        ServerCoreType::Fabric => {
            // Find latest loader for this game version
            let loader_url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}",
                game_version
            );
            let resp = client
                .get(&loader_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Fabric loader for {}: {e}", game_version))?;
            let loaders: Vec<FabricLoaderEntry> = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse Fabric loaders: {e}"))?;

            let loader_ver = loaders
                .iter()
                .find(|l| l.loader.stable)
                .or_else(|| loaders.first())
                .map(|l| &l.loader.version)
                .ok_or_else(|| format!("No Fabric loader found for {}", game_version))?;

            let server_jar_url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}/{}/server/jar",
                game_version, loader_ver
            );

            download_file_chunked(client, &server_jar_url, &dest_jar, None, None).await?;
            Ok(format!("Fabric server (loader {}) installed", loader_ver))
        }

        ServerCoreType::Vanilla => {
            let manifest_res = client
                .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
                .send()
                .await
                .map_err(|e| format!("Failed to fetch Mojang manifest: {e}"))?;
            let manifest: MojangVersionManifest = manifest_res
                .json()
                .await
                .map_err(|e| format!("Failed to parse Mojang manifest: {e}"))?;

            let entry = manifest
                .versions
                .into_iter()
                .find(|v| v.id == game_version)
                .ok_or_else(|| format!("Vanilla version {} not found in manifest", game_version))?;

            let pkg_res = client
                .get(&entry.url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch version package: {e}"))?;
            let pkg: MojangVersionPackage = pkg_res
                .json()
                .await
                .map_err(|e| format!("Failed to parse version package: {e}"))?;

            let server_dl = pkg
                .downloads
                .and_then(|d| d.server)
                .ok_or_else(|| format!("No server download provided for vanilla {}", game_version))?;

            download_file_chunked(
                client,
                &server_dl.url,
                &dest_jar,
                server_dl.sha1.as_deref(),
                None,
            )
            .await?;
            Ok(format!("Vanilla server {} installed", game_version))
        }
    }
}
