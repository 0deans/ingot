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

/// Ensures the Pumpkin executable is in the server folder and returns its path,
/// downloading the build for this platform on first use. On Android that's the Linux
/// musl build, which runs inside the PRoot sandbox's Alpine rootfs like Java does.
pub async fn ensure_pumpkin_binary(
    client: &reqwest::Client,
    server_dir: &Path,
) -> Result<PathBuf, String> {
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

    // Release asset for this platform (e.g. "pumpkin-ARM64-Linux-musl"). Matched by
    // suffix: "ARM64-Linux" must not pick the musl build, and vice versa.
    let target_pattern = if cfg!(target_os = "android") {
        // Alpine uses musl, so the glibc build wouldn't start in the sandbox
        "ARM64-Linux-musl"
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
        .find(|a| a.name.ends_with(target_pattern) || a.name.ends_with(&format!("{target_pattern}.exe")))
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

    // Write beside the target, then rename: an interrupted download never looks complete
    let part = bin_path.with_extension("part");
    std::fs::write(&part, &bytes)
        .and_then(|()| std::fs::rename(&part, &bin_path))
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

/// Bedrock port for a server whose players join on `public_port`: 19132 for the first
/// server (25565), then moving along with it so several Pumpkin servers don't collide.
/// Bedrock connects to Pumpkin directly (the wake-up proxy is Java-only).
fn bedrock_port(public_port: u16) -> u16 {
    19132u16.saturating_add(public_port.saturating_sub(25565))
}

/// Points Pumpkin at the ports Ingot chose, on every start (the port can change in
/// Settings). Pumpkin reads `pumpkin.toml`: `[networking.java] address` and
/// `[networking.bedrock.nethernet] address`. Edits only those keys, so everything else
/// in the file (and Pumpkin's own defaults for missing keys) stays as it is.
pub fn set_ports(server_dir: &Path, java_port: u16, public_port: u16) -> Result<(), String> {
    let path = server_dir.join("pumpkin.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .map_err(|e| format!("pumpkin.toml has an error, so Ingot can't set the port: {e}"))?;
    let set = |doc: &mut toml_edit::DocumentMut, table_path: &[&str], address: String| {
        let mut table = doc.as_table_mut();
        for key in table_path {
            let entry = table.entry(key).or_insert(toml_edit::table());
            let Some(next) = entry.as_table_mut() else { return };
            next.set_implicit(true);
            table = next;
        }
        table["address"] = toml_edit::value(address);
    };
    let java_host = if java_port != public_port {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    set(
        &mut doc,
        &["networking", "java"],
        format!("{java_host}:{java_port}"),
    );
    set(
        &mut doc,
        &["networking", "bedrock", "nethernet"],
        format!("0.0.0.0:{}", bedrock_port(public_port)),
    );
    let out = doc.to_string();
    if out != text {
        std::fs::write(&path, out).map_err(|e| format!("Failed to write pumpkin.toml: {e}"))?;
    }
    Ok(())
}

/// Builds the command that runs Pumpkin, after pointing it at its ports.
///
/// `sandbox` is the PRoot sandbox and rootfs on Android: Android won't run programs
/// an app downloaded, so Pumpkin runs inside the Alpine rootfs through PRoot (the only
/// bundled program), with the server folder mounted at /server.
pub fn build_pumpkin_command(
    binary_path: &Path,
    server_dir: &Path,
    port: u16,
    public_port: u16,
    sandbox: Option<(&Path, &Path)>,
) -> Result<Command, String> {
    set_ports(server_dir, port, public_port)?;
    let mut cmd = match sandbox {
        Some((sandbox_dir, rootfs)) => {
            let name = binary_path
                .file_name()
                .ok_or("Invalid Pumpkin binary path")?
                .to_string_lossy()
                .into_owned();
            let mut cmd = crate::server::sandbox::proot_command(
                sandbox_dir,
                Some(rootfs),
                false,
                &[(server_dir.to_path_buf(), "/server")],
                "/server",
            )?;
            cmd.arg(format!("/server/{name}"));
            cmd
        }
        None => Command::new(binary_path),
    };
    cmd.current_dir(server_dir);
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    #[test]
    fn sets_ports_in_pumpkin_toml() {
        let dir = std::env::temp_dir().join(format!("ingot-pumpkin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("pumpkin.toml");

        // No file yet: Pumpkin fills in its defaults around the ports
        super::set_ports(&dir, 25568, 25567).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        let doc: toml_edit::DocumentMut = text.parse().unwrap();
        assert_eq!(doc["networking"]["java"]["address"].as_str(), Some("127.0.0.1:25568"));
        assert_eq!(doc["networking"]["bedrock"]["nethernet"]["address"].as_str(), Some("0.0.0.0:19134"));

        // Pumpkin's own file: only the addresses change, comments and settings stay
        std::fs::write(
            &file,
            "# My server\nhardcore = true\n\n[networking.java]\naddress = \"0.0.0.0:25565\"\nonline_mode = false\n",
        )
        .unwrap();
        super::set_ports(&dir, 25571, 25570).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("# My server\nhardcore = true\n"), "{text}");
        assert!(text.contains("address = \"127.0.0.1:25571\"\nonline_mode = false"), "{text}");
        assert!(text.contains("0.0.0.0:19137"), "{text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
