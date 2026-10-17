use super::progress::ProgressCallback;
use crate::minecraft::downloader::download_file_chunked;
use std::path::{Path, PathBuf};

pub async fn setup_ely_authlib_agent(
    client: &reqwest::Client,
    cache_dir: &Path,
    report_prog: &ProgressCallback,
) -> PathBuf {
    let authlib_path = cache_dir.join("authlib-injector.jar");
    if !authlib_path.exists() {
        report_prog(
            "Auth Setup",
            7,
            10,
            "Setting up Ely.by skin & authentication agent...",
        );
        let mut dl_url =
            "https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar".to_string();
        if let Ok(resp) = client
            .get("https://authlib-injector.yushi.moe/artifact/latest.json")
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(u) = json.get("download_url").and_then(|v| v.as_str()) {
                    dl_url = u.to_string();
                }
            }
        }

        if let Err(e) = download_file_chunked(client, &dl_url, &authlib_path, None, None).await {
            eprintln!(
                "[Launcher] Primary authlib-injector download failed ({e}), trying GitHub releases mirror..."
            );
            let github_url = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.8/authlib-injector-1.2.8.jar";
            let _ = download_file_chunked(client, github_url, &authlib_path, None, None).await;
        }
    }
    authlib_path
}
