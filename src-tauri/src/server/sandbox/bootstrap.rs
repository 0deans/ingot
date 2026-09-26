use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tar::Archive;
use tauri::{Manager, Runtime};

/// Alpine release used for the sandbox. Its community repo ships OpenJDK 8/11/17/21/25.
const ALPINE_BRANCH: &str = "v3.24";
const ALPINE_VERSION: &str = "3.24.2";

/// Written after the base rootfs is fully extracted, so partial extractions get redone
const BASE_MARKER: &str = ".ingot-rootfs";

fn alpine_rootfs_url() -> String {
    format!(
        "https://dl-cdn.alpinelinux.org/alpine/{ALPINE_BRANCH}/releases/aarch64/alpine-minirootfs-{ALPINE_VERSION}-aarch64.tar.gz"
    )
}

/// Resolves the sandbox directory inside app data
pub fn get_sandbox_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;
    let sandbox_dir = data_dir.join("sandbox");
    if !sandbox_dir.exists() {
        fs::create_dir_all(&sandbox_dir)
            .map_err(|e| format!("Failed to create sandbox directory: {e}"))?;
    }
    Ok(sandbox_dir)
}

/// Resolves the rootfs directory
pub fn get_rootfs_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let sandbox_dir = get_sandbox_dir(app)?;
    Ok(sandbox_dir.join("rootfs"))
}

/// Checks if the base Alpine rootfs is fully extracted and ready
pub fn is_rootfs_installed(rootfs_dir: &Path) -> bool {
    rootfs_dir.join(BASE_MARKER).exists() && rootfs_dir.join("sbin").join("apk").exists()
}

/// Maps a required Java major version to the closest Alpine package that satisfies it
fn java_package(major: u32) -> (u32, &'static str) {
    match major {
        0..=8 => (8, "openjdk8-jre"),
        9..=11 => (11, "openjdk11-jre-headless"),
        12..=17 => (17, "openjdk17-jre-headless"),
        18..=21 => (21, "openjdk21-jre-headless"),
        _ => (25, "openjdk25-jre-headless"),
    }
}

/// Guest paths where Alpine installs the java binary for a given major version
fn guest_java_candidates(major: u32) -> Vec<String> {
    if major == 8 {
        vec![
            "/usr/lib/jvm/java-1.8-openjdk/bin/java".into(),
            "/usr/lib/jvm/java-1.8-openjdk/jre/bin/java".into(),
        ]
    } else {
        vec![format!("/usr/lib/jvm/java-{major}-openjdk/bin/java")]
    }
}

fn find_guest_java(rootfs_dir: &Path, major: u32) -> Option<String> {
    guest_java_candidates(major)
        .into_iter()
        // symlink_metadata: absolute symlinks inside the rootfs don't resolve on the host
        .find(|p| fs::symlink_metadata(rootfs_dir.join(p.trim_start_matches('/'))).is_ok())
}

/// Downloads and extracts the Alpine base rootfs
async fn install_base_rootfs<FProgress>(
    client: &reqwest::Client,
    sandbox_dir: &Path,
    rootfs_dir: &Path,
    on_progress: &FProgress,
) -> Result<(), String>
where
    FProgress: Fn(&str, f32) + Send + Sync + 'static,
{
    let archive_path = sandbox_dir.join("rootfs.tar.gz");

    on_progress("Downloading Linux sandbox...", 0.05);

    let mut response = client
        .get(alpine_rootfs_url())
        .send()
        .await
        .map_err(|e| format!("Failed to download rootfs: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Linux rootfs download failed (HTTP {}). On mobile, you can use PumpkinMC (Rust) which runs natively without requiring Java or rootfs.",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(4 * 1024 * 1024);
    let mut downloaded = 0u64;

    let mut file = File::create(&archive_path)
        .map_err(|e| format!("Failed to create temporary rootfs archive: {e}"))?;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Rootfs download interrupted: {e}"))?
    {
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("Error writing chunk: {e}"))?;

        downloaded += chunk.len() as u64;
        let progress = 0.05 + ((downloaded as f32 / total_size as f32) * 0.15);
        on_progress("Downloading Linux sandbox...", progress.min(0.20));
    }

    drop(file);

    on_progress("Extracting sandbox filesystem...", 0.20);

    // Start from a clean directory so a previously interrupted extraction can't linger
    if rootfs_dir.exists() {
        let _ = fs::remove_dir_all(rootfs_dir);
    }
    fs::create_dir_all(rootfs_dir).map_err(|e| format!("Failed to create rootfs directory: {e}"))?;

    let archive_path_clone = archive_path.clone();
    let rootfs_clone = rootfs_dir.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let tar_file = File::open(&archive_path_clone)
            .map_err(|e| format!("Failed to open rootfs archive for extraction: {e}"))?;
        let mut archive = Archive::new(GzDecoder::new(tar_file));
        archive
            .unpack(&rootfs_clone)
            .map_err(|e| format!("Failed to extract rootfs archive: {e}"))
    })
    .await
    .map_err(|e| format!("Rootfs extraction task failed: {e}"))??;

    let _ = fs::remove_file(archive_path);

    // Android has no /etc/resolv.conf, so the guest needs its own DNS config
    let etc = rootfs_dir.join("etc");
    fs::write(etc.join("resolv.conf"), "nameserver 1.1.1.1\nnameserver 8.8.8.8\n")
        .map_err(|e| format!("Failed to write resolv.conf: {e}"))?;
    fs::write(etc.join("hosts"), "127.0.0.1 localhost\n::1 localhost ip6-localhost\n")
        .map_err(|e| format!("Failed to write hosts: {e}"))?;
    fs::write(
        etc.join("apk").join("repositories"),
        format!(
            "https://dl-cdn.alpinelinux.org/alpine/{ALPINE_BRANCH}/main\nhttps://dl-cdn.alpinelinux.org/alpine/{ALPINE_BRANCH}/community\n"
        ),
    )
    .map_err(|e| format!("Failed to write apk repositories: {e}"))?;

    fs::write(rootfs_dir.join(BASE_MARKER), ALPINE_VERSION)
        .map_err(|e| format!("Failed to write rootfs marker: {e}"))?;
    Ok(())
}

/// Installs an OpenJDK package inside the sandbox using apk (run through PRoot)
async fn install_java(sandbox_dir: &Path, rootfs_dir: &Path, package: &str) -> Result<(), String> {
    let mut cmd = super::proot::proot_command(sandbox_dir, Some(rootfs_dir), true, &[], "/root")?;
    cmd.args(["/sbin/apk", "add", "--no-cache", "--no-progress", package]);
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to launch PRoot: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("[Sandbox] apk add {package}:\n{stdout}{stderr}");

    if !output.status.success() {
        let tail: Vec<&str> = stderr.lines().chain(stdout.lines()).rev().take(15).collect();
        let tail: Vec<&str> = tail.into_iter().rev().collect();
        return Err(format!(
            "Failed to install {package} in the Linux sandbox ({}):\n{}",
            output.status,
            tail.join("\n")
        ));
    }
    Ok(())
}

/// Ensures the Linux rootfs and a matching Java runtime are installed.
/// Returns `(sandbox_dir, rootfs_dir, guest_java_path)`.
pub async fn ensure_sandbox_rootfs<R: Runtime, FProgress>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    java_major: u32,
    on_progress: FProgress,
) -> Result<(PathBuf, PathBuf, String), String>
where
    FProgress: Fn(&str, f32) + Send + Sync + 'static,
{
    // Serialize setup: concurrent starts would race on extraction and on apk's db lock
    static SETUP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _setup_guard = SETUP_LOCK.lock().await;

    let sandbox_dir = get_sandbox_dir(app)?;
    let rootfs_dir = get_rootfs_dir(app)?;

    if !is_rootfs_installed(&rootfs_dir) {
        install_base_rootfs(client, &sandbox_dir, &rootfs_dir, &on_progress).await?;
    }

    let (major, package) = java_package(java_major);
    if let Some(java) = find_guest_java(&rootfs_dir, major) {
        return Ok((sandbox_dir, rootfs_dir, java));
    }

    on_progress(&format!("Installing Java {major} in sandbox..."), 0.30);
    install_java(&sandbox_dir, &rootfs_dir, package).await?;

    let java = find_guest_java(&rootfs_dir, major).ok_or_else(|| {
        format!("{package} was installed but no java binary was found in the sandbox")
    })?;

    on_progress("Sandbox initialization complete!", 1.0);
    Ok((sandbox_dir, rootfs_dir, java))
}
