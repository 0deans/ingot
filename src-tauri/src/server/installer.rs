//! Mod loader servers set up by their loader's own installer: NeoForge, Forge and Quilt.
//! There's no single jar to download; on the first start the installer (a Java program)
//! downloads Minecraft's server and the libraries, and for NeoForge and Forge patches them.
//! NeoForge and Forge are then started from the argument file their installer writes,
//! Quilt from its launch jar.

use super::config::ServerCoreType;
use crate::minecraft::downloader::download_file_chunked;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const FORGE_PROMOTIONS: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const QUILT_META: &str = "https://meta.quiltmc.org/v3/versions";
const QUILT_LAUNCHER: &str = "quilt-server-launch.jar";

/// Whether this core is set up by an installer (instead of downloading server.jar)
pub fn uses_installer(core: &ServerCoreType) -> bool {
    matches!(core, ServerCoreType::NeoForge | ServerCoreType::Forge | ServerCoreType::Quilt)
}

/// Where NeoForge and Forge install themselves (one folder per version)
fn versions_dir(core: &ServerCoreType) -> Option<&'static str> {
    match core {
        ServerCoreType::NeoForge => Some("libraries/net/neoforged/neoforge"),
        ServerCoreType::Forge => Some("libraries/net/minecraftforge/forge"),
        _ => None,
    }
}

/// Compares versions number by number ("21.1.100" > "21.1.99")
fn version_key(version: &str) -> Vec<u32> {
    version.split(['.', '-']).filter_map(|p| p.parse().ok()).collect()
}

fn newest_first(mut games: Vec<String>) -> Vec<String> {
    games.sort_by_key(|g| std::cmp::Reverse(version_key(g)));
    games.dedup();
    games
}

async fn get(client: &reqwest::Client, url: &str) -> Result<reqwest::Response, String> {
    client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Failed to reach {url}: {e}"))
}

// ─── Versions ─────────────────────────────────────────────────────────────────

/// NeoForge's version for a Minecraft version: "21.1.77" is 1.21.1, "21.0.x" is 1.21 and,
/// since the year-based versions, "26.1.2.111" is 26.1.2 and "26.3.0.5-beta" is 26.3.
/// None for snapshots, April Fools versions and alphas.
fn neoforge_game_version(neoforge: &str) -> Option<String> {
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

async fn neoforge_versions(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let xml = get(client, &format!("{NEOFORGE_MAVEN}/maven-metadata.xml"))
        .await?
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

/// NeoForge for this Minecraft version: newest stable, else newest beta
fn pick_neoforge(versions: &[String], game_version: &str) -> Option<String> {
    versions
        .iter()
        .filter(|v| neoforge_game_version(v).as_deref() == Some(game_version))
        .max_by_key(|v| (!v.contains('-'), version_key(v)))
        .cloned()
}

/// Forge's promoted builds: "1.20.1-recommended" -> "47.4.10"
async fn forge_promotions(client: &reqwest::Client) -> Result<serde_json::Map<String, Value>, String> {
    let json: Value = get(client, FORGE_PROMOTIONS)
        .await?
        .json()
        .await
        .map_err(|e| format!("Failed to read Forge versions: {e}"))?;
    Ok(json.get("promos").and_then(Value::as_object).cloned().unwrap_or_default())
}

/// Forge from 1.17, the first version started from an argument file
fn forge_supported(game_version: &str) -> bool {
    let v = version_key(game_version);
    v.first().is_some_and(|&major| major > 1) || (v.first() == Some(&1) && v.get(1).is_some_and(|&minor| minor >= 17))
}

/// Forge for this Minecraft version: the recommended build, else the latest ("1.20.1-47.4.10")
fn pick_forge(promos: &serde_json::Map<String, Value>, game_version: &str) -> Option<String> {
    ["recommended", "latest"]
        .iter()
        .find_map(|kind| promos.get(&format!("{game_version}-{kind}")).and_then(Value::as_str))
        .map(|build| format!("{game_version}-{build}"))
}

/// Minecraft versions this loader supports, newest first
pub async fn fetch_game_versions(client: &reqwest::Client, core: &ServerCoreType) -> Result<Vec<String>, String> {
    let games = match core {
        ServerCoreType::NeoForge => neoforge_versions(client).await?.iter().filter_map(|v| neoforge_game_version(v)).collect(),
        ServerCoreType::Forge => forge_promotions(client)
            .await?
            .keys()
            .filter_map(|k| k.rsplit_once('-').map(|(game, _)| game.to_string()))
            .filter(|g| forge_supported(g))
            .collect(),
        ServerCoreType::Quilt => {
            let list: Vec<Value> = get(client, &format!("{QUILT_META}/game"))
                .await?
                .json()
                .await
                .map_err(|e| format!("Failed to read Quilt versions: {e}"))?;
            list.iter()
                .filter(|v| v.get("stable").and_then(Value::as_bool) == Some(true))
                .filter_map(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                .collect()
        }
        _ => return Err(format!("{core} isn't installed by an installer")),
    };
    Ok(newest_first(games))
}

// ─── Installing ───────────────────────────────────────────────────────────────

/// What to start the installed server with, after the JVM options: an argument file for
/// NeoForge and Forge ("@libraries/.../unix_args.txt"), the launch jar for Quilt.
/// None if it isn't installed.
pub fn launch_args(server_dir: &Path, core: &ServerCoreType) -> Option<Vec<String>> {
    if *core == ServerCoreType::Quilt {
        let installed = server_dir.join(QUILT_LAUNCHER).is_file() && server_dir.join("server.jar").is_file();
        return installed.then(|| vec!["-jar".into(), QUILT_LAUNCHER.into()]);
    }
    let name = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
    let mut found: Vec<PathBuf> = std::fs::read_dir(server_dir.join(versions_dir(core)?))
        .ok()?
        .flatten()
        .map(|e| e.path().join(name))
        .filter(|p| p.is_file())
        .collect();
    found.sort_by_key(|p| version_key(&p.parent().unwrap_or(p).file_name().unwrap_or_default().to_string_lossy()));
    let rel = found.pop()?.strip_prefix(server_dir).ok()?.to_string_lossy().replace('\\', "/");
    Some(vec![format!("@{rel}")])
}

/// Forgets the installed loader so the next start installs it again (after a version change)
pub fn uninstall(server_dir: &Path, core: &ServerCoreType) -> Result<(), String> {
    let remove = |path: PathBuf| -> Result<(), String> {
        let result = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
        match result {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("Failed to remove the old {core}: {e}")),
            _ => Ok(()),
        }
    };
    match versions_dir(core) {
        Some(dir) => remove(server_dir.join(dir)),
        None if *core == ServerCoreType::Quilt => {
            remove(server_dir.join(QUILT_LAUNCHER))?;
            remove(server_dir.join("server.jar"))
        }
        None => Ok(()),
    }
}

/// Installs the loader into the server folder if it isn't yet, and returns what to start
/// it with (see [`launch_args`]). `java` builds a command running Java with the given
/// arguments in the server folder (natively, or inside the sandbox on Android).
/// `pinned` is a loader version to use instead of the newest.
pub async fn ensure_installed(
    client: &reqwest::Client,
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    pinned: Option<&str>,
    java: impl Fn(&[String]) -> Result<Command, String>,
) -> Result<Vec<String>, String> {
    if let Some(args) = launch_args(server_dir, core) {
        return Ok(args);
    }
    let unavailable = || format!("{core} isn't available for Minecraft {game_version}");
    let (url, installer, args): (String, &str, Vec<String>) = match core {
        ServerCoreType::NeoForge => {
            let version = match pinned {
                Some(v) => v.to_string(),
                None => pick_neoforge(&neoforge_versions(client).await?, game_version).ok_or_else(unavailable)?,
            };
            let url = format!("{NEOFORGE_MAVEN}/{version}/neoforge-{version}-installer.jar");
            (url, "neoforge-installer.jar", vec!["--installServer".into()])
        }
        ServerCoreType::Forge => {
            if !forge_supported(game_version) {
                return Err(format!("Ingot runs Forge servers from Minecraft 1.17; this one is {game_version}"));
            }
            let version = match pinned {
                Some(v) => format!("{game_version}-{v}"),
                None => pick_forge(&forge_promotions(client).await?, game_version).ok_or_else(unavailable)?,
            };
            let url = format!("{FORGE_MAVEN}/{version}/forge-{version}-installer.jar");
            (url, "forge-installer.jar", vec!["--installServer".into()])
        }
        ServerCoreType::Quilt => {
            let list: Vec<Value> = get(client, &format!("{QUILT_META}/installer"))
                .await?
                .json()
                .await
                .map_err(|e| format!("Failed to read Quilt installer versions: {e}"))?;
            let url = list
                .first()
                .and_then(|v| v.get("url"))
                .and_then(Value::as_str)
                .ok_or("No Quilt installer found")?
                .to_string();
            let mut args: Vec<String> = vec!["install".into(), "server".into(), game_version.into()];
            args.extend(pinned.map(str::to_string));
            args.extend(["--download-server".into(), "--install-dir=.".into()]);
            (url, "quilt-installer.jar", args)
        }
        _ => return Err(format!("{core} isn't installed by an installer")),
    };

    let path = server_dir.join(installer);
    download_file_chunked(client, &url, &path, None, None).await?;
    let mut java_args: Vec<String> = vec!["-jar".into(), installer.into()];
    java_args.extend(args);
    let mut cmd = java(&java_args)?;
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let output = cmd.output().await.map_err(|e| format!("Failed to run the {core} installer: {e}"))?;

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(server_dir.join(format!("{installer}.log")));
    if !output.status.success() {
        let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let tail = lines[lines.len().saturating_sub(8)..].join("\n");
        return Err(format!("The {core} installer failed:\n{tail}"));
    }
    launch_args(server_dir, core).ok_or_else(|| format!("The {core} installer didn't set the server up"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_neoforge_versions() {
        assert_eq!(neoforge_game_version("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(neoforge_game_version("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(neoforge_game_version("20.2.3-beta").as_deref(), Some("1.20.2"));
        assert_eq!(neoforge_game_version("26.1.2.111").as_deref(), Some("26.1.2"));
        assert_eq!(neoforge_game_version("26.3.0.23-beta").as_deref(), Some("26.3"));
        assert_eq!(neoforge_game_version("26.1.0.0-alpha.7+snapshot-3"), None);
        assert_eq!(neoforge_game_version("0.25w14craftmine.3-beta"), None);
    }

    #[test]
    fn picks_newest_stable_neoforge_then_beta() {
        let versions: Vec<String> = ["21.1.9", "21.1.100", "21.1.99", "21.1.101-beta", "26.3.0.5-beta", "26.3.0.23-beta"]
            .map(str::to_string)
            .into();
        assert_eq!(pick_neoforge(&versions, "1.21.1").as_deref(), Some("21.1.100"));
        assert_eq!(pick_neoforge(&versions, "26.3").as_deref(), Some("26.3.0.23-beta"));
        assert_eq!(pick_neoforge(&versions, "1.20.4"), None);
    }

    #[test]
    fn picks_recommended_forge_then_latest() {
        let promos: serde_json::Map<String, Value> = serde_json::from_str(
            r#"{"1.20.1-latest":"47.4.12","1.20.1-recommended":"47.4.10","26.3-latest":"66.0.5"}"#,
        )
        .unwrap();
        assert_eq!(pick_forge(&promos, "1.20.1").as_deref(), Some("1.20.1-47.4.10"));
        assert_eq!(pick_forge(&promos, "26.3").as_deref(), Some("26.3-66.0.5"));
        assert_eq!(pick_forge(&promos, "1.19.2"), None);
        assert!(forge_supported("1.17.1") && forge_supported("26.1") && !forge_supported("1.16.5"));
    }

    #[test]
    fn finds_what_to_launch() {
        let dir = std::env::temp_dir().join(format!("ingot-installer-{}", std::process::id()));
        let name = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
        assert_eq!(launch_args(&dir, &ServerCoreType::NeoForge), None);
        for v in ["21.1.99", "21.1.100"] {
            let d = dir.join("libraries/net/neoforged/neoforge").join(v);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(name), "").unwrap();
        }
        let expected = format!("@libraries/net/neoforged/neoforge/21.1.100/{name}");
        assert_eq!(launch_args(&dir, &ServerCoreType::NeoForge), Some(vec![expected]));
        assert_eq!(launch_args(&dir, &ServerCoreType::Forge), None);
        uninstall(&dir, &ServerCoreType::NeoForge).unwrap();
        assert_eq!(launch_args(&dir, &ServerCoreType::NeoForge), None);

        std::fs::write(dir.join(QUILT_LAUNCHER), "").unwrap();
        assert_eq!(launch_args(&dir, &ServerCoreType::Quilt), None, "needs Minecraft's server.jar too");
        std::fs::write(dir.join("server.jar"), "").unwrap();
        assert!(launch_args(&dir, &ServerCoreType::Quilt).is_some());
        uninstall(&dir, &ServerCoreType::Quilt).unwrap();
        assert_eq!(launch_args(&dir, &ServerCoreType::Quilt), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
