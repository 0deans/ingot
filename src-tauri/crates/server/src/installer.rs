//! Mod loader servers set up by their loader's own installer: NeoForge, Forge and Quilt.
//! There's no single jar to download; on the first start the installer (a Java program)
//! downloads Minecraft's server and the libraries, and for NeoForge and Forge patches them.
//! NeoForge and Forge are then started from the argument file their installer writes,
//! Quilt from its launch jar.

use super::config::ServerCoreType;
use ingot_artifacts::downloader::download_file_chunked;
use std::path::{Path, PathBuf};
mod execution;
mod providers;
mod selection;
use providers::InstallerEndpoints;
use selection::InstallerDownloadPlan;
use tokio::process::Command;

pub(super) const QUILT_LAUNCHER: &str = "quilt-server-launch.jar";

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
    Download(#[from] ingot_artifacts::downloader::DownloadError),

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

/// Minecraft versions this loader supports, newest first
pub async fn fetch_game_versions(
    client: &reqwest::Client,
    core: &ServerCoreType,
) -> Result<Vec<String>, InstallerError> {
    fetch_game_versions_with_endpoints(client, core, &InstallerEndpoints::production()).await
}

async fn fetch_game_versions_with_endpoints(
    client: &reqwest::Client,
    core: &ServerCoreType,
    endpoints: &InstallerEndpoints,
) -> Result<Vec<String>, InstallerError> {
    match core {
        ServerCoreType::NeoForge => {
            let versions = providers::fetch_neoforge_versions(client, endpoints).await?;
            Ok(selection::select_neoforge_game_versions(&versions))
        }
        ServerCoreType::Forge => {
            let promos = providers::fetch_forge_promotions(client, endpoints).await?;
            Ok(selection::select_forge_game_versions(&promos))
        }
        ServerCoreType::Quilt => {
            let versions = providers::fetch_quilt_game_versions(client, endpoints).await?;
            Ok(selection::select_quilt_game_versions(&versions))
        }
        _ => Err(InstallerError::UnsupportedCore(core.clone())),
    }
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
        selection::version_key(
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
    ensure_installed_with_endpoints(
        client,
        server_dir,
        core,
        game_version,
        pinned,
        &InstallerEndpoints::production(),
        java,
    )
    .await
}

async fn ensure_installed_with_endpoints<E: std::error::Error + Send + Sync + 'static>(
    client: &reqwest::Client,
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    pinned: Option<&str>,
    endpoints: &InstallerEndpoints,
    java: impl Fn(&[String]) -> Result<Command, E>,
) -> Result<Vec<String>, InstallerError> {
    if let Some(args) = launch_args(server_dir, core)? {
        return Ok(args);
    }
    let plan = resolve_install_plan(client, core, game_version, pinned, endpoints).await?;
    let path = server_dir.join(plan.installer_name);
    download_file_chunked(client, &plan.url, &path, None, None).await?;
    execution::run_downloaded_installer(server_dir, core, plan.installer_name, plan.args, java)
        .await?;
    launch_args(server_dir, core)?.ok_or_else(|| InstallerError::NotConfigured(core.clone()))
}

async fn resolve_install_plan(
    client: &reqwest::Client,
    core: &ServerCoreType,
    game_version: &str,
    pinned: Option<&str>,
    endpoints: &InstallerEndpoints,
) -> Result<InstallerDownloadPlan, InstallerError> {
    match core {
        ServerCoreType::NeoForge => {
            let version = match pinned {
                Some(v) => v.to_string(),
                None => {
                    let versions = providers::fetch_neoforge_versions(client, endpoints).await?;
                    selection::pick_neoforge(&versions, game_version).ok_or_else(|| {
                        InstallerError::VersionUnavailable {
                            core: core.clone(),
                            game_version: game_version.to_string(),
                        }
                    })?
                }
            };
            Ok(selection::neoforge_install_plan(endpoints, &version))
        }
        ServerCoreType::Forge => {
            if !selection::forge_supported(game_version) {
                return Err(InstallerError::ForgeVersionUnsupported(
                    game_version.to_string(),
                ));
            }
            let version = match pinned {
                Some(v) => format!("{game_version}-{v}"),
                None => {
                    let promos = providers::fetch_forge_promotions(client, endpoints).await?;
                    selection::pick_forge(&promos, game_version).ok_or_else(|| {
                        InstallerError::VersionUnavailable {
                            core: core.clone(),
                            game_version: game_version.to_string(),
                        }
                    })?
                }
            };
            Ok(selection::forge_install_plan(endpoints, &version))
        }
        ServerCoreType::Quilt => {
            let installer_url = providers::fetch_quilt_installer_url(client, endpoints).await?;
            Ok(selection::quilt_install_plan(
                installer_url,
                game_version,
                pinned,
            ))
        }
        _ => Err(InstallerError::UnsupportedCore(core.clone())),
    }
}

#[cfg(test)]
mod tests;
