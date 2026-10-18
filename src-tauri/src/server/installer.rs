//! Mod loader servers set up by their loader's own installer: NeoForge, Forge and Quilt.
//! There's no single jar to download; on the first start the installer (a Java program)
//! downloads Minecraft's server and the libraries, and for NeoForge and Forge patches them.
//! NeoForge and Forge are then started from the argument file their installer writes,
//! Quilt from its launch jar.

use super::config::ServerCoreType;
use crate::minecraft::downloader::download_file_chunked;
use serde_json::Value;
use std::path::{Path, PathBuf};
mod execution;
use tokio::process::Command;

const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const QUILT_META: &str = "https://meta.quiltmc.org/v3/versions";
const QUILT_LAUNCHER: &str = "quilt-server-launch.jar";

/// Whether this core is set up by an installer (instead of downloading server.jar)
pub fn uses_installer(core: &ServerCoreType) -> bool {
    matches!(
        core,
        ServerCoreType::NeoForge | ServerCoreType::Forge | ServerCoreType::Quilt
    )
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
    version
        .split(['.', '-'])
        .filter_map(|p| p.parse().ok())
        .collect()
}

fn newest_first(mut games: Vec<String>) -> Vec<String> {
    games.sort_by_key(|g| std::cmp::Reverse(version_key(g)));
    games.dedup();
    games
}

#[derive(Debug, thiserror::Error)]
pub enum InstallerError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0} is not installed by an installer")]
    UnsupportedCore(ServerCoreType),

    #[error("{core} is not available for Minecraft {game_version}")]
    VersionUnavailable {
        core: ServerCoreType,
        game_version: String,
    },

    #[error("Ingot runs Forge servers from Minecraft 1.17; this one is {0}")]
    ForgeVersionUnsupported(String),

    #[error("No Quilt installer found")]
    NoQuiltInstaller,

    #[error("Download error: {0}")]
    Download(#[from] crate::minecraft::downloader::DownloadError),

    #[error("Failed to run {core} installer: {source}")]
    Process {
        core: ServerCoreType,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("The {core} installer failed:\n{details}")]
    InstallationFailed {
        core: ServerCoreType,
        details: String,
    },

    #[error("The {0} installer didn't set the server up")]
    NotConfigured(ServerCoreType),

    #[error("The {core} installer exceeded its execution time limit")]
    ExecutionTimeout { core: ServerCoreType },

    #[error("Installer cleanup failed: {source}")]
    Cleanup {
        primary: Option<Box<InstallerError>>,
        #[source]
        source: std::io::Error,
    },
}

async fn get(client: &reqwest::Client, url: &str) -> Result<reqwest::Response, InstallerError> {
    client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(InstallerError::Network)
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
    let parts: Vec<u32> = numbers
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    match parts.as_slice() {
        [major, minor, patch, _] if *major >= 25 => Some(if *patch == 0 {
            format!("{major}.{minor}")
        } else {
            format!("{major}.{minor}.{patch}")
        }),
        [major, minor, _] if (20..25).contains(major) => Some(if *minor == 0 {
            format!("1.{major}")
        } else {
            format!("1.{major}.{minor}")
        }),
        _ => None,
    }
}

async fn neoforge_versions(client: &reqwest::Client) -> Result<Vec<String>, InstallerError> {
    let xml = get(client, &format!("{NEOFORGE_MAVEN}/maven-metadata.xml"))
        .await?
        .text()
        .await
        .map_err(InstallerError::Network)?;
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
async fn forge_promotions(
    client: &reqwest::Client,
) -> Result<serde_json::Map<String, Value>, InstallerError> {
    let json: Value = get(client, FORGE_PROMOTIONS)
        .await?
        .json()
        .await
        .map_err(InstallerError::Network)?;
    Ok(json
        .get("promos")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default())
}

/// Forge from 1.17, the first version started from an argument file
fn forge_supported(game_version: &str) -> bool {
    let v = version_key(game_version);
    v.first().is_some_and(|&major| major > 1)
        || (v.first() == Some(&1) && v.get(1).is_some_and(|&minor| minor >= 17))
}

/// Forge for this Minecraft version: the recommended build, else the latest ("1.20.1-47.4.10")
fn pick_forge(promos: &serde_json::Map<String, Value>, game_version: &str) -> Option<String> {
    ["recommended", "latest"]
        .iter()
        .find_map(|kind| {
            promos
                .get(&format!("{game_version}-{kind}"))
                .and_then(Value::as_str)
        })
        .map(|build| format!("{game_version}-{build}"))
}

/// Minecraft versions this loader supports, newest first
pub async fn fetch_game_versions(
    client: &reqwest::Client,
    core: &ServerCoreType,
) -> Result<Vec<String>, InstallerError> {
    let games = match core {
        ServerCoreType::NeoForge => neoforge_versions(client)
            .await?
            .iter()
            .filter_map(|v| neoforge_game_version(v))
            .collect(),
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
                .map_err(InstallerError::Network)?;
            list.iter()
                .filter(|v| v.get("stable").and_then(Value::as_bool) == Some(true))
                .filter_map(|v| v.get("version").and_then(Value::as_str).map(str::to_string))
                .collect()
        }
        _ => return Err(InstallerError::UnsupportedCore(core.clone())),
    };
    Ok(newest_first(games))
}

// ─── Installing ───────────────────────────────────────────────────────────────

fn default_version_dirs(vdir: &Path) -> std::io::Result<Vec<std::io::Result<PathBuf>>> {
    let entries = std::fs::read_dir(vdir)?;
    let mut out = Vec::new();
    for entry in entries {
        match entry {
            Ok(e) => match e.file_type() {
                Ok(ft) => {
                    if ft.is_dir() {
                        out.push(Ok(e.path()));
                    }
                }
                Err(err) => out.push(Err(err)),
            },
            Err(err) => out.push(Err(err)),
        }
    }
    Ok(out)
}

/// What to start the installed server with, after the JVM options: an argument file for
/// NeoForge and Forge ("@libraries/.../unix_args.txt"), the launch jar for Quilt.
/// Returns Ok(None) if it isn't installed, or an error if installation cannot be inspected.
pub fn launch_args(
    server_dir: &Path,
    core: &ServerCoreType,
) -> Result<Option<Vec<String>>, InstallerError> {
    launch_args_with(server_dir, core, default_version_dirs, |p| {
        std::fs::metadata(p)
    })
}

pub(crate) fn launch_args_with<DirFn, MetaFn>(
    server_dir: &Path,
    core: &ServerCoreType,
    mut read_dirs_fn: DirFn,
    mut meta_fn: MetaFn,
) -> Result<Option<Vec<String>>, InstallerError>
where
    DirFn: FnMut(&Path) -> std::io::Result<Vec<std::io::Result<PathBuf>>>,
    MetaFn: FnMut(&Path) -> std::io::Result<std::fs::Metadata>,
{
    if *core == ServerCoreType::Quilt {
        let launcher = server_dir.join(QUILT_LAUNCHER);
        let server_jar = server_dir.join("server.jar");
        let launcher_exists = match meta_fn(&launcher) {
            Ok(m) => m.is_file(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(InstallerError::Io(e)),
        };
        let server_exists = match meta_fn(&server_jar) {
            Ok(m) => m.is_file(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(InstallerError::Io(e)),
        };
        if launcher_exists && server_exists {
            return Ok(Some(vec!["-jar".into(), QUILT_LAUNCHER.into()]));
        }
        return Ok(None);
    }
    let Some(rel_dir) = versions_dir(core) else {
        return Ok(None);
    };
    let vdir = server_dir.join(rel_dir);
    let dirs = match read_dirs_fn(&vdir) {
        Ok(dirs) => dirs,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(InstallerError::Io(e)),
    };
    let name = if cfg!(windows) {
        "win_args.txt"
    } else {
        "unix_args.txt"
    };
    let mut found: Vec<PathBuf> = Vec::new();
    for dir_res in dirs {
        let dir_path = dir_res.map_err(InstallerError::Io)?;
        let arg_path = dir_path.join(name);
        match meta_fn(&arg_path) {
            Ok(m) if m.is_file() => found.push(arg_path),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(InstallerError::Io(e)),
        }
    }
    if found.is_empty() {
        return Ok(None);
    }
    found.sort_by_key(|p| {
        version_key(
            &p.parent()
                .unwrap_or(p)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
        )
    });
    let rel = found
        .pop()
        .and_then(|p| p.strip_prefix(server_dir).ok().map(|s| s.to_path_buf()))
        .ok_or_else(|| InstallerError::NotConfigured(core.clone()))?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(Some(vec![format!("@{rel}")]))
}

/// Forgets the installed loader so the next start installs it again (after a version change)
pub fn uninstall(server_dir: &Path, core: &ServerCoreType) -> Result<(), InstallerError> {
    let remove = |path: PathBuf| -> Result<(), InstallerError> {
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match result {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(InstallerError::Io(e)),
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
pub async fn ensure_installed<E: std::error::Error + Send + Sync + 'static>(
    client: &reqwest::Client,
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    pinned: Option<&str>,
    java: impl Fn(&[String]) -> Result<Command, E>,
) -> Result<Vec<String>, InstallerError> {
    if let Some(args) = launch_args(server_dir, core)? {
        return Ok(args);
    }
    let (url, installer, args): (String, &str, Vec<String>) =
        match core {
            ServerCoreType::NeoForge => {
                let version = match pinned {
                    Some(v) => v.to_string(),
                    None => pick_neoforge(&neoforge_versions(client).await?, game_version)
                        .ok_or_else(|| InstallerError::VersionUnavailable {
                            core: core.clone(),
                            game_version: game_version.to_string(),
                        })?,
                };
                let url = format!("{NEOFORGE_MAVEN}/{version}/neoforge-{version}-installer.jar");
                (
                    url,
                    "neoforge-installer.jar",
                    vec!["--installServer".into()],
                )
            }
            ServerCoreType::Forge => {
                if !forge_supported(game_version) {
                    return Err(InstallerError::ForgeVersionUnsupported(
                        game_version.to_string(),
                    ));
                }
                let version = match pinned {
                    Some(v) => format!("{game_version}-{v}"),
                    None => pick_forge(&forge_promotions(client).await?, game_version).ok_or_else(
                        || InstallerError::VersionUnavailable {
                            core: core.clone(),
                            game_version: game_version.to_string(),
                        },
                    )?,
                };
                let url = format!("{FORGE_MAVEN}/{version}/forge-{version}-installer.jar");
                (url, "forge-installer.jar", vec!["--installServer".into()])
            }
            ServerCoreType::Quilt => {
                let list: Vec<Value> = get(client, &format!("{QUILT_META}/installer"))
                    .await?
                    .json()
                    .await
                    .map_err(InstallerError::Network)?;
                let url = list
                    .first()
                    .and_then(|v| v.get("url"))
                    .and_then(Value::as_str)
                    .ok_or(InstallerError::NoQuiltInstaller)?
                    .to_string();
                let mut args: Vec<String> =
                    vec!["install".into(), "server".into(), game_version.into()];
                args.extend(pinned.map(str::to_string));
                args.extend(["--download-server".into(), "--install-dir=.".into()]);
                (url, "quilt-installer.jar", args)
            }
            _ => return Err(InstallerError::UnsupportedCore(core.clone())),
        };

    let path = server_dir.join(installer);
    download_file_chunked(client, &url, &path, None, None).await?;
    execution::run_downloaded_installer(server_dir, core, installer, args, java).await?;
    launch_args(server_dir, core)?.ok_or_else(|| InstallerError::NotConfigured(core.clone()))
}

#[cfg(test)]
mod tests;
