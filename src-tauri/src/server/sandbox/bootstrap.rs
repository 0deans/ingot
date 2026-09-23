use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tar::Archive;
use tauri::{Manager, Runtime};

/// Default URL for pre-built Alpine aarch64 rootfs with OpenJDK 21 headless
pub const DEFAULT_ALPINE_ROOTFS_URL: &str =
    "https://github.com/0deans/ingot/releases/download/v0.3.0/alpine-java21-rootfs-aarch64.tar.gz";

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

/// Checks if the rootfs is fully extracted and ready
pub fn is_rootfs_installed(rootfs_dir: &Path) -> bool {
    let java_bin = rootfs_dir.join("usr").join("bin").join("java");
    let java_alt = rootfs_dir
        .join("usr")
        .join("lib")
        .join("jvm")
        .join("java-21-openjdk")
        .join("bin")
        .join("java");
    let busybox = rootfs_dir.join("bin").join("busybox");
    (java_bin.exists() || java_alt.exists()) && busybox.exists()
}

/// Downloads and extracts the Linux rootfs on-demand
pub async fn ensure_sandbox_rootfs<R: Runtime, FProgress>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    on_progress: FProgress,
) -> Result<PathBuf, String>
where
    FProgress: Fn(&str, f32) + Send + Sync + 'static,
{
    let rootfs_dir = get_rootfs_dir(app)?;
    if is_rootfs_installed(&rootfs_dir) {
        return Ok(rootfs_dir);
    }

    let sandbox_dir = get_sandbox_dir(app)?;
    let archive_path = sandbox_dir.join("rootfs.tar.gz");

    on_progress("Downloading unprivileged Java Linux runtime...", 0.05);

    // Download rootfs archive
    let response = client
        .get(DEFAULT_ALPINE_ROOTFS_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to download rootfs: {e}"))?;

    let total_size = response.content_length().unwrap_or(30 * 1024 * 1024);
    let mut downloaded = 0u64;

    let mut file = File::create(&archive_path)
        .map_err(|e| format!("Failed to create temporary rootfs archive: {e}"))?;

    let mut response = response;
    while let Ok(Some(chunk)) = response.chunk().await {
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("Error writing chunk: {e}"))?;

        downloaded += chunk.len() as u64;
        let progress = 0.05 + ((downloaded as f32 / total_size as f32) * 0.70);
        on_progress("Downloading Java Linux runtime...", progress.min(0.75));
    }

    drop(file);

    on_progress("Extracting sandbox filesystem...", 0.80);

    // Extract tar.gz into rootfs_dir
    let tar_file = File::open(&archive_path)
        .map_err(|e| format!("Failed to open rootfs archive for extraction: {e}"))?;
    let tar = GzDecoder::new(tar_file);
    let mut archive = Archive::new(tar);

    if !rootfs_dir.exists() {
        fs::create_dir_all(&rootfs_dir)
            .map_err(|e| format!("Failed to create rootfs directory: {e}"))?;
    }

    archive
        .unpack(&rootfs_dir)
        .map_err(|e| format!("Failed to extract rootfs archive: {e}"))?;

    // Cleanup downloaded archive
    let _ = fs::remove_file(archive_path);

    on_progress("Sandbox initialization complete!", 1.0);
    Ok(rootfs_dir)
}
