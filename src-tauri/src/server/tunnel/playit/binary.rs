use super::PlayitError;
use std::path::PathBuf;
use tauri::{Manager, Runtime};

/// Finds or downloads the playit agent, the first time someone goes public.
/// On Android it isn't bundled: it runs through PRoot (see `build_agent_command`),
/// which can start it from app storage where Android won't run programs itself.
pub async fn ensure_playit_binary<R: Runtime>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
) -> Result<PathBuf, PlayitError> {
    // 1. Desktop: an agent the user installed themselves
    #[cfg(not(target_os = "android"))]
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join(if cfg!(windows) {
                "playit.exe"
            } else {
                "playit"
            });
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // 2. Previously downloaded, else download it
    let data_dir = app.path().app_data_dir().map_err(PlayitError::AppData)?;
    let tools_dir = data_dir.join("tools");
    std::fs::create_dir_all(&tools_dir)?;

    let bin_name = if cfg!(target_os = "windows") {
        "playit.exe"
    } else {
        "playit"
    };
    let target_path = tools_dir.join(bin_name);

    if target_path.exists() {
        // Sanity-check: must be > 1 MB (reject corrupt/partial downloads)
        if std::fs::metadata(&target_path)
            .map(|m| m.len() > 1_000_000)
            .unwrap_or(false)
        {
            return Ok(target_path);
        }
        let _ = std::fs::remove_file(&target_path);
    }

    // Download official playit-agent binary from GitHub
    let download_url = if cfg!(target_os = "windows") {
        "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-windows-x86_64.exe"
    } else if cfg!(target_arch = "aarch64") {
        "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-linux-aarch64"
    } else {
        "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-linux-amd64"
    };

    eprintln!("[Playit] Downloading agent binary from {download_url}...");
    let res = client
        .get(download_url)
        .header("User-Agent", crate::USER_AGENT)
        .send()
        .await
        .map_err(|error| PlayitError::Network(error.without_url()))?;

    if !res.status().is_success() {
        return Err(PlayitError::HttpStatus(res.status()));
    }

    let bytes = res
        .bytes()
        .await
        .map_err(|error| PlayitError::Network(error.without_url()))?;

    // Write beside the target, then rename: an interrupted download never looks complete
    let part = target_path.with_extension("part");
    std::fs::write(&part, &bytes)?;
    std::fs::rename(&part, &target_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(target_path)
}
