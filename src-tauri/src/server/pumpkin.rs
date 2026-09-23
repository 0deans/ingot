use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    #[allow(dead_code)]
    size: u64,
}

/// Resolves the Pumpkin executable name for the current platform
pub fn get_pumpkin_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "pumpkin.exe";
    #[cfg(not(target_os = "windows"))]
    return "pumpkin";
}

/// Locates the pumpkin executable inside the server directory
pub fn get_pumpkin_binary_path(server_dir: &Path) -> PathBuf {
    server_dir.join(get_pumpkin_binary_name())
}

/// Fetches available Pumpkin versions from GitHub Releases
pub async fn fetch_pumpkin_versions(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let url = "https://api.github.com/repos/Pumpkin-MC/Pumpkin/releases?per_page=10";
    let res = client
        .get(url)
        .header("User-Agent", "Ingot-Minecraft-Launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Pumpkin releases: {e}"))?;

    let releases: Vec<GithubRelease> = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Pumpkin releases response: {e}"))?;

    let versions: Vec<String> = releases.into_iter().map(|r| r.tag_name).collect();
    if versions.is_empty() {
        Ok(vec!["v0.1.0".to_string()])
    } else {
        Ok(versions)
    }
}

/// Ensures the Pumpkin native executable is downloaded in the server directory
pub async fn ensure_pumpkin_binary(client: &reqwest::Client, server_dir: &Path) -> Result<PathBuf, String> {
    let bin_path = get_pumpkin_binary_path(server_dir);
    if bin_path.exists() {
        return Ok(bin_path);
    }

    let url = "https://api.github.com/repos/Pumpkin-MC/Pumpkin/releases/latest";
    let res = client
        .get(url)
        .header("User-Agent", "Ingot-Minecraft-Launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch latest Pumpkin release: {e}"))?;

    let release: GithubRelease = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Pumpkin release info: {e}"))?;

    // Determine target asset pattern based on OS and ARCH
    let target_pattern = if cfg!(target_os = "windows") {
        "x86_64-pc-windows"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux"
    } else {
        "x86_64-unknown-linux"
    };

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.contains(target_pattern))
        .or_else(|| release.assets.first())
        .ok_or_else(|| "No compatible Pumpkin binary asset found for this architecture".to_string())?;

    // Download asset
    let download_res = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "Ingot-Minecraft-Launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to download Pumpkin binary: {e}"))?;

    let bytes = download_res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read Pumpkin binary stream: {e}"))?;

    std::fs::write(&bin_path, bytes)
        .map_err(|e| format!("Failed to write Pumpkin binary to {}: {e}", bin_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o755));
    }

    Ok(bin_path)
}

/// Builds the Command to run Pumpkin natively
pub fn build_pumpkin_command(server_dir: &Path, port: u16) -> Command {
    let bin = get_pumpkin_binary_path(server_dir);
    let mut cmd = Command::new(bin);
    cmd.current_dir(server_dir);
    cmd.arg("--port");
    cmd.arg(port.to_string());
    cmd
}
