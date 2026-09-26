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

/// On Android, try to find `libpumpkin.so` in the native library directory.
/// This is the fast path — if the APK was built with the binary bundled,
/// we can execute it directly without memfd overhead.
pub fn locate_pumpkin_bundled() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    if let Ok(lib_dir) = std::env::var("ANDROID_APP_LIB_DIR") {
        let candidate = PathBuf::from(lib_dir).join("libpumpkin.so");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Fetches available Pumpkin versions from GitHub Releases
pub async fn fetch_pumpkin_versions(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let url = "https://api.github.com/repos/Pumpkin-MC/Pumpkin/releases?per_page=10";
    let res = client
        .get(url)
        .header("User-Agent", crate::USER_AGENT)
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

/// Ensures the Pumpkin native executable is available and returns its path.
///
/// On Android:
///   1. If the APK was built with `libpumpkin.so` bundled → returns that path (executable directly).
///   2. Otherwise → downloads to app-data cache dir and returns that path.
///      The caller (`build_pumpkin_command`) will use `memfd_create` to execute it.
///
/// On Desktop: downloads the appropriate binary to the server directory if
/// not already present.
pub async fn ensure_pumpkin_binary(
    client: &reqwest::Client,
    server_dir: &Path,
) -> Result<PathBuf, String> {
    // Fast path: bundled in APK (preferred)
    if let Some(bundled) = locate_pumpkin_bundled() {
        return Ok(bundled);
    }

    // Determine where to cache the binary
    #[cfg(target_os = "android")]
    let bin_path = {
        // On Android, download to a cache dir under app data.
        // The binary lives here on disk but is executed via memfd (not directly).
        let cache_dir = server_dir.join(".bin_cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        cache_dir.join("pumpkin")
    };

    #[cfg(not(target_os = "android"))]
    let bin_path = get_pumpkin_binary_path(server_dir);

    if bin_path.exists() {
        if std::fs::metadata(&bin_path)
            .map(|m| m.len() > 1_000_000)
            .unwrap_or(false)
        {
            return Ok(bin_path);
        }
        // Remove corrupt/tiny previous download (e.g. a checksum text file)
        let _ = std::fs::remove_file(&bin_path);
    }

    let url = "https://api.github.com/repos/Pumpkin-MC/Pumpkin/releases/latest";
    let res = client
        .get(url)
        .header("User-Agent", crate::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch latest Pumpkin release: {e}"))?;

    let release: GithubRelease = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Pumpkin release info: {e}"))?;

    // Determine target asset name pattern for this platform
    let target_pattern = if cfg!(target_os = "android") {
        "aarch64-android"
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "ARM64-Windows"
        } else {
            "X64-Windows"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "ARM64-macOS"
        } else {
            "macOS"
        }
    } else if cfg!(target_arch = "aarch64") {
        "ARM64-Linux"
    } else {
        "X64-Linux"
    };

    let asset = release
        .assets
        .iter()
        .filter(|a| {
            !a.name.ends_with(".sha256")
                && !a.name.ends_with(".md5")
                && !a.name.contains("checksum")
        })
        .find(|a| a.name.contains(target_pattern))
        .ok_or_else(|| {
            format!(
                "No compatible Pumpkin binary found matching '{target_pattern}' in release {}",
                release.tag_name
            )
        })?;

    let download_res = client
        .get(&asset.browser_download_url)
        .header("User-Agent", crate::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to download Pumpkin binary: {e}"))?;

    if !download_res.status().is_success() {
        return Err(format!(
            "Failed to download Pumpkin binary from {}: HTTP {}",
            asset.browser_download_url,
            download_res.status()
        ));
    }

    let bytes = download_res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read Pumpkin binary stream: {e}"))?;

    std::fs::write(&bin_path, &bytes)
        .map_err(|e| format!("Failed to write Pumpkin binary to {}: {e}", bin_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            &bin_path,
            std::fs::Permissions::from_mode(0o755),
        );
    }

    Ok(bin_path)
}

/// Builds the Command to run Pumpkin natively.
///
/// `binary_path` is the resolved executable path (from `ensure_pumpkin_binary`).
///
/// On Android, if the binary is NOT in `nativeLibraryDir` (i.e. it was downloaded
/// to app-data which is noexec), we transparently use `memfd_create` to execute
/// it from RAM — bypassing the W^X filesystem restriction.
pub fn build_pumpkin_command(binary_path: &Path, server_dir: &Path, port: u16) -> Result<Command, String> {
    let config_path = server_dir.join("configuration.toml");
    if !config_path.exists() {
        let content = format!(
            "[server]\naddress = \"0.0.0.0:{port}\"\n\n[bedrock]\nenabled = true\naddress = \"0.0.0.0:19132\"\n"
        );
        let _ = std::fs::write(&config_path, content);
    }

    // On Android: if the binary is in nativeLibraryDir, exec directly.
    // Otherwise (downloaded to noexec app-data) — use memfd to exec from RAM.
    #[cfg(target_os = "android")]
    {
        let is_bundled = std::env::var("ANDROID_APP_LIB_DIR")
            .ok()
            .map(|lib_dir| binary_path.starts_with(&lib_dir))
            .unwrap_or(false);

        if !is_bundled {
            eprintln!(
                "[Pumpkin] Binary is on noexec fs ({}), using memfd_create to execute.",
                binary_path.display()
            );
            return crate::server::memfd::memfd::command_from_noexec_path(binary_path, server_dir);
        }
    }

    let mut cmd = Command::new(binary_path);
    cmd.current_dir(server_dir);
    Ok(cmd)
}
