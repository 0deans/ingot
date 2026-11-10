use super::models::{HorizonChannel, HorizonError, HorizonRelease};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/horizon-rust/horizon/releases?per_page=100";
const CACHE_TTL: Duration = Duration::from_secs(15 * 60);
const DEFAULT_HORIZON_TARGET_MC_VERSION: &str = "26.3";

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheMeta {
    etag: Option<String>,
    cached_at: u64,
}

pub fn parse_horizon_channel(tag: &str) -> Option<HorizonChannel> {
    if tag.starts_with("launcher@")
        || tag.starts_with("launcher-")
        || tag.starts_with("nightly-source/")
    {
        return None;
    }
    if tag.starts_with("nightly") {
        Some(HorizonChannel::Nightly)
    } else if tag.starts_with('v') {
        Some(HorizonChannel::Stable)
    } else {
        Some(HorizonChannel::Nightly)
    }
}

pub fn target_platform_asset_name() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "horizon-windows-x64.zip"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "horizon-linux-x86_64.tar.gz"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "horizon-macos-arm64.zip"
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        "horizon-windows-x64.zip"
    }
}

pub async fn fetch_horizon_releases(
    client: &reqwest::Client,
    cache_dir: &Path,
) -> Result<Vec<HorizonRelease>, HorizonError> {
    let cache_file = cache_dir.join("horizon_releases.json");
    let meta_file = cache_dir.join("horizon_releases_meta.json");

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cached_meta: Option<CacheMeta> = fs::read_to_string(&meta_file)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok());

    if let Some(ref meta) = cached_meta {
        if now_secs.saturating_sub(meta.cached_at) < CACHE_TTL.as_secs() {
            if let Ok(content) = fs::read_to_string(&cache_file) {
                if let Ok(releases) = serde_json::from_str::<Vec<HorizonRelease>>(&content) {
                    return Ok(releases);
                }
            }
        }
    }

    let mut request = client
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", "Ingot-Launcher");

    if let Some(etag) = cached_meta.as_ref().and_then(|m| m.etag.as_deref()) {
        request = request.header("If-None-Match", etag);
    }

    let response = match request.send().await {
        Ok(res) => res,
        Err(err) => {
            if let Ok(content) = fs::read_to_string(&cache_file) {
                if let Ok(releases) = serde_json::from_str::<Vec<HorizonRelease>>(&content) {
                    return Ok(releases);
                }
            }
            return Err(HorizonError::Transport(err));
        }
    };

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        if let Some(mut meta) = cached_meta {
            meta.cached_at = now_secs;
            let _ = fs::write(&meta_file, serde_json::to_string(&meta).unwrap_or_default());
        }
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(releases) = serde_json::from_str::<Vec<HorizonRelease>>(&content) {
                return Ok(releases);
            }
        }
    }

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(releases) = serde_json::from_str::<Vec<HorizonRelease>>(&content) {
                eprintln!("[Horizon] Warning: GitHub API rate limited; serving cached releases.");
                return Ok(releases);
            }
        }
    }

    let etag = response
        .headers()
        .get("ETag")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string());

    let raw_releases: Vec<GithubRelease> = response.json().await?;
    let expected_asset = target_platform_asset_name();

    let mut parsed_releases = Vec::new();
    for release in raw_releases {
        let Some(channel) = parse_horizon_channel(&release.tag_name) else {
            continue;
        };

        let matched_asset = release
            .assets
            .into_iter()
            .find(|asset| asset.name.eq_ignore_ascii_case(expected_asset));

        let display_name = release
            .name
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| release.tag_name.clone());

        parsed_releases.push(HorizonRelease {
            tag: release.tag_name,
            name: display_name,
            published_at: release.published_at,
            channel,
            target_game_version: DEFAULT_HORIZON_TARGET_MC_VERSION.to_string(),
            asset_url: matched_asset
                .as_ref()
                .map(|a| a.browser_download_url.clone()),
            asset_name: matched_asset.as_ref().map(|a| a.name.clone()),
            asset_size: matched_asset.as_ref().map(|a| a.size),
        });
    }

    let _ = fs::create_dir_all(cache_dir);
    if let Ok(serialized) = serde_json::to_string(&parsed_releases) {
        let _ = fs::write(&cache_file, serialized);
        let meta = CacheMeta {
            etag,
            cached_at: now_secs,
        };
        let _ = fs::write(&meta_file, serde_json::to_string(&meta).unwrap_or_default());
    }

    Ok(parsed_releases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_horizon_channel() {
        assert_eq!(
            parse_horizon_channel("nightly-2026-10-31"),
            Some(HorizonChannel::Nightly)
        );
        assert_eq!(
            parse_horizon_channel("nightly"),
            Some(HorizonChannel::Nightly)
        );
        assert_eq!(
            parse_horizon_channel("v0.2.30"),
            Some(HorizonChannel::Stable)
        );
        assert_eq!(
            parse_horizon_channel("v2026.10.19"),
            Some(HorizonChannel::Stable)
        );
        assert_eq!(parse_horizon_channel("launcher@0.2.30"), None);
        assert_eq!(parse_horizon_channel("launcher-v0.1.1"), None);
        assert_eq!(parse_horizon_channel("nightly-source/2026-10-31"), None);
    }

    #[test]
    fn test_parse_release_json_filtering() {
        let json_data = r#"[
            {
                "tag_name": "launcher@0.2.30",
                "name": "Horizon launcher",
                "assets": []
            },
            {
                "tag_name": "nightly-2026-10-31",
                "name": "Nightly 2026-10-31",
                "published_at": "2026-10-31T08:00:00Z",
                "assets": [
                    {
                        "name": "horizon-windows-x64.zip",
                        "browser_download_url": "https://example.com/windows.zip",
                        "size": 12345
                    },
                    {
                        "name": "horizon-linux-x86_64.tar.gz",
                        "browser_download_url": "https://example.com/linux.tar.gz",
                        "size": 12345
                    }
                ]
            }
        ]"#;

        let raw: Vec<GithubRelease> = serde_json::from_str(json_data).unwrap();
        assert_eq!(raw.len(), 2);
        let channel_0 = parse_horizon_channel(&raw[0].tag_name);
        assert_eq!(channel_0, None);
        let channel_1 = parse_horizon_channel(&raw[1].tag_name);
        assert_eq!(channel_1, Some(HorizonChannel::Nightly));
    }
}
