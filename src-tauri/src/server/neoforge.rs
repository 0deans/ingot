//! NeoForge servers. Unlike the other cores there's no single server jar: NeoForge's
//! installer sets the server up once (downloading Minecraft's jar and the libraries, then
//! patching it), and the server is started from the argument file it writes.

use crate::minecraft::downloader::download_file_chunked;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

const MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const INSTALLER: &str = "neoforge-installer.jar";

/// NeoForge's version for a Minecraft version: "21.1.77" is 1.21.1, "21.0.x" is 1.21 and,
/// since the year-based versions, "26.1.2.111" is 26.1.2 and "26.3.0.5-beta" is 26.3.
/// None for snapshots, April Fools versions and alphas.
fn minecraft_version(neoforge: &str) -> Option<String> {
    if neoforge.contains("alpha") || neoforge.contains('+') {
        return None;
    }
    let numbers = neoforge.split('-').next()?;
    let parts: Vec<u32> = numbers.split('.').map(str::parse).collect::<Result<_, _>>().ok()?;
    match parts.as_slice() {
        [major, minor, patch, _] if *major >= 25 => {
            Some(if *patch == 0 { format!("{major}.{minor}") } else { format!("{major}.{minor}.{patch}") })
        }
        [major, minor, _] if (20..25).contains(major) => {
            Some(if *minor == 0 { format!("1.{major}") } else { format!("1.{major}.{minor}") })
        }
        _ => None,
    }
}

fn is_beta(version: &str) -> bool {
    version.contains('-')
}

/// Compares versions number by number ("21.1.100" > "21.1.99")
fn version_key(version: &str) -> Vec<u32> {
    version.split(['.', '-']).filter_map(|p| p.parse().ok()).collect()
}

async fn all_versions(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let xml = client
        .get(format!("{MAVEN}/maven-metadata.xml"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Failed to reach NeoForge: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read NeoForge versions: {e}"))?;
    Ok(xml
        .split("<version>")
        .skip(1)
        .filter_map(|s| s.split("</version>").next())
        .map(str::to_string)
        .collect())
}

/// The NeoForge version to use for this Minecraft version: newest stable, else newest beta
fn pick(versions: &[String], game_version: &str) -> Option<String> {
    versions
        .iter()
        .filter(|v| minecraft_version(v).as_deref() == Some(game_version))
        .max_by_key(|v| (!is_beta(v), version_key(v)))
        .cloned()
}

/// Minecraft versions NeoForge supports, newest first
pub async fn fetch_game_versions(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let mut games: Vec<String> = all_versions(client).await?.iter().filter_map(|v| minecraft_version(v)).collect();
    games.sort_by_key(|g| std::cmp::Reverse(version_key(g)));
    games.dedup();
    Ok(games)
}

/// The argument file written by the installer, relative to the server folder
/// ("libraries/net/neoforged/neoforge/<version>/unix_args.txt")
pub fn args_file(server_dir: &Path) -> Option<String> {
    let name = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
    let versions = server_dir.join("libraries/net/neoforged/neoforge");
    let mut found: Vec<PathBuf> = std::fs::read_dir(versions)
        .ok()?
        .flatten()
        .map(|e| e.path().join(name))
        .filter(|p| p.is_file())
        .collect();
    found.sort_by_key(|p| version_key(&p.parent().unwrap_or(p).file_name().unwrap_or_default().to_string_lossy()));
    let file = found.pop()?;
    let rel = file.strip_prefix(server_dir).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// Forgets the installed NeoForge so the next start installs it again (after a version change)
pub fn uninstall(server_dir: &Path) -> Result<(), String> {
    let dir = server_dir.join("libraries/net/neoforged/neoforge");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("Failed to remove the old NeoForge: {e}"))?;
    }
    Ok(())
}

/// Installs NeoForge into the server folder if it isn't yet, and returns the argument file
/// to start it with. `java` builds a command running Java with the given arguments in the
/// server folder (natively, or inside the sandbox on Android).
pub async fn ensure_installed(
    client: &reqwest::Client,
    server_dir: &Path,
    game_version: &str,
    pinned: Option<&str>,
    java: impl Fn(&[String]) -> Result<Command, String>,
) -> Result<String, String> {
    if let Some(file) = args_file(server_dir) {
        return Ok(file);
    }
    let version = match pinned {
        Some(v) => v.to_string(),
        None => pick(&all_versions(client).await?, game_version)
            .ok_or_else(|| format!("NeoForge isn't available for Minecraft {game_version}"))?,
    };

    let installer = server_dir.join(INSTALLER);
    download_file_chunked(
        client,
        &format!("{MAVEN}/{version}/neoforge-{version}-installer.jar"),
        &installer,
        None,
        None,
    )
    .await?;

    let mut cmd = java(&["-jar".into(), INSTALLER.into(), "--installServer".into()])?;
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let output = cmd.output().await.map_err(|e| format!("Failed to run the NeoForge installer: {e}"))?;

    let _ = std::fs::remove_file(&installer);
    let _ = std::fs::remove_file(server_dir.join(format!("{INSTALLER}.log")));
    if !output.status.success() {
        let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let tail: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let tail = tail[tail.len().saturating_sub(8)..].join("\n");
        return Err(format!("The NeoForge {version} installer failed:\n{tail}"));
    }
    args_file(server_dir).ok_or_else(|| format!("The NeoForge {version} installer didn't set the server up"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_minecraft_versions() {
        assert_eq!(minecraft_version("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(minecraft_version("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(minecraft_version("20.2.3-beta").as_deref(), Some("1.20.2"));
        assert_eq!(minecraft_version("26.1.2.111").as_deref(), Some("26.1.2"));
        assert_eq!(minecraft_version("26.3.0.23-beta").as_deref(), Some("26.3"));
        assert_eq!(minecraft_version("26.1.0.0-alpha.7+snapshot-3"), None);
        assert_eq!(minecraft_version("0.25w14craftmine.3-beta"), None);
    }

    #[test]
    fn picks_newest_stable_then_beta() {
        let versions: Vec<String> = ["21.1.9", "21.1.100", "21.1.99", "21.1.101-beta", "26.3.0.5-beta", "26.3.0.23-beta"]
            .map(str::to_string)
            .into();
        assert_eq!(pick(&versions, "1.21.1").as_deref(), Some("21.1.100"));
        assert_eq!(pick(&versions, "26.3").as_deref(), Some("26.3.0.23-beta"));
        assert_eq!(pick(&versions, "1.20.4"), None);
    }

    #[test]
    fn finds_the_newest_args_file() {
        let dir = std::env::temp_dir().join(format!("ingot-neoforge-{}", std::process::id()));
        let name = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
        assert_eq!(args_file(&dir), None);
        for v in ["21.1.99", "21.1.100"] {
            let d = dir.join("libraries/net/neoforged/neoforge").join(v);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(name), "").unwrap();
        }
        assert_eq!(args_file(&dir), Some(format!("libraries/net/neoforged/neoforge/21.1.100/{name}")));
        uninstall(&dir).unwrap();
        assert_eq!(args_file(&dir), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
