use crate::minecraft::downloader::download_file_chunked;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tauri::{Manager, Runtime};

/// Resolves the required Java major version for a given Minecraft version.
pub fn get_required_java_version(game_version: &str, meta_major_version: Option<u32>) -> u32 {
    if let Some(v) = meta_major_version {
        if v > 0 {
            return v;
        }
    }

    let parts: Vec<&str> = game_version.split('.').collect();
    if !parts.is_empty() {
        if let Ok(first) = parts[0].parse::<u32>() {
            if first >= 25 {
                return 25;
            } else if first >= 20 {
                return 21;
            } else if first == 1 && parts.len() >= 2 {
                if let Ok(minor) = parts[1].parse::<u32>() {
                    if minor >= 21 {
                        return 21;
                    } else if minor == 20 {
                        let patch = parts.get(2).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
                        if patch >= 5 {
                            return 21;
                        } else {
                            return 17;
                        }
                    } else if minor >= 18 {
                        return 17;
                    } else if minor == 17 {
                        return 16;
                    } else {
                        return 8;
                    }
                }
            }
        }
    }

    // Check for snapshot naming like "26w04a"
    if game_version.starts_with("26w") || game_version.starts_with("25w") {
        return 25;
    }

    21
}

/// Helper to get the java binary name depending on the OS
pub fn get_java_executable_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "javaw.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "java"
    }
}

/// Recursively searches for `javaw.exe` (or `java`) inside a directory
pub fn find_java_in_dir(dir: &Path) -> Option<PathBuf> {
    let target_name = get_java_executable_name();
    if !dir.exists() {
        return None;
    }

    // Check direct bin folder first
    let direct = dir.join("bin").join(target_name);
    if direct.exists() {
        return Some(direct);
    }

    // Check nested directory (e.g. jdk-21.0.6+7/bin/javaw.exe)
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested_bin = path.join("bin").join(target_name);
                if nested_bin.exists() {
                    return Some(nested_bin);
                }
            }
        }
    }

    None
}

/// Probes a java executable and extracts its major version number
pub fn probe_java_version(java_bin: &Path) -> Option<u32> {
    if !java_bin.exists() {
        return None;
    }

    // Run java -version
    let output = Command::new(java_bin).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    parse_java_version_str(&text)
}

fn parse_java_version_str(text: &str) -> Option<u32> {
    for line in text.lines() {
        if line.contains("version") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let ver = &line[start + 1..start + 1 + end];
                    if ver.starts_with("1.") {
                        // e.g. 1.8.0_351 -> 8
                        let parts: Vec<&str> = ver.split('.').collect();
                        if parts.len() > 1 {
                            return parts[1].parse::<u32>().ok();
                        }
                    } else {
                        // e.g. 17.0.2 or 21.0.6
                        let parts: Vec<&str> = ver.split('.').collect();
                        if let Some(first) = parts.first() {
                            return first.parse::<u32>().ok();
                        }
                    }
                }
            }
        }
    }
    None
}

/// Scans the local system for a matching Java major version
pub fn find_system_java(major_version: u32) -> Option<PathBuf> {
    // Check JAVA_HOME
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let p = PathBuf::from(java_home);
        if let Some(bin) = find_java_in_dir(&p) {
            if probe_java_version(&bin) == Some(major_version) {
                return Some(bin);
            }
        }
    }

    // Check standard Windows directories
    #[cfg(target_os = "windows")]
    {
        let search_dirs = [
            PathBuf::from("C:\\Program Files\\Eclipse Adoptium"),
            PathBuf::from("C:\\Program Files\\Java"),
            PathBuf::from("C:\\Program Files\\Microsoft"),
            PathBuf::from("C:\\Program Files\\BellSoft"),
            PathBuf::from("C:\\Program Files\\Amazon Corretto"),
        ];

        for base in search_dirs {
            if let Ok(entries) = fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let dir = entry.path();
                    if dir.is_dir() {
                        if let Some(bin) = find_java_in_dir(&dir) {
                            if probe_java_version(&bin) == Some(major_version) {
                                return Some(bin);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Returns the runtime folder for a specified Java major version in app data
pub fn get_runtime_dir<R: Runtime>(
    app: &tauri::AppHandle<R>,
    major_version: u32,
) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?;
    let runtimes_dir = data_dir.join("runtimes").join(format!("java-{}", major_version));
    if !runtimes_dir.exists() {
        fs::create_dir_all(&runtimes_dir)
            .map_err(|e| format!("Failed to create runtimes directory: {e}"))?;
    }
    Ok(runtimes_dir)
}

/// Ensures a compatible Java runtime exists, downloading it automatically from Adoptium if needed.
pub async fn ensure_java_runtime<R: Runtime>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    major_version: u32,
    custom_path: Option<&str>,
    progress: Option<Arc<dyn Fn(u64, u64, &str) + Send + Sync>>,
) -> Result<PathBuf, String> {
    // 1. Check custom path if provided
    if let Some(custom) = custom_path {
        let p = PathBuf::from(custom);
        if p.exists() {
            if let Some(bin) = find_java_in_dir(&p) {
                return Ok(bin);
            }
            if p.is_file() {
                return Ok(p);
            }
        }
    }

    // 2. Check cached runtimes directory in app data
    let runtime_dir = get_runtime_dir(app, major_version)?;
    if let Some(cached_bin) = find_java_in_dir(&runtime_dir) {
        return Ok(cached_bin);
    }

    // 3. Check system PATH or standard installed directories
    if let Some(sys_bin) = find_system_java(major_version) {
        return Ok(sys_bin);
    }

    // 4. Not found: Automatically download from Eclipse Adoptium
    if let Some(ref cb) = progress {
        cb(0, 100, &format!("Downloading Java {} runtime...", major_version));
    }

    #[cfg(target_os = "windows")]
    let os = "windows";
    #[cfg(target_os = "macos")]
    let os = "mac";
    #[cfg(target_os = "linux")]
    let os = "linux";

    #[cfg(target_arch = "x86_64")]
    let arch = "x64";
    #[cfg(target_arch = "aarch64")]
    let arch = "aarch64";

    let download_url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jdk/hotspot/normal/eclipse",
        major_version, os, arch
    );

    let cache_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))?
        .join("cache");
    let _ = fs::create_dir_all(&cache_dir);
    let zip_dest = cache_dir.join(format!("java-{}-{}-{}.zip", major_version, os, arch));

    let dl_cb = progress.clone().map(|cb| {
        Arc::new(move |current: u64, total: u64| {
            cb(
                current,
                total,
                &format!(
                    "Downloading Java {} runtime ({:.1}MB / {:.1}MB)...",
                    major_version,
                    current as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                ),
            );
        }) as Arc<dyn Fn(u64, u64) + Send + Sync>
    });

    download_file_chunked(client, &download_url, &zip_dest, None, dl_cb).await?;

    // Extract archive
    if let Some(ref cb) = progress {
        cb(100, 100, &format!("Extracting Java {} runtime...", major_version));
    }

    let zip_file = fs::File::open(&zip_dest)
        .map_err(|e| format!("Failed to open downloaded java zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| format!("Failed to read java zip archive: {e}"))?;

    archive
        .extract(&runtime_dir)
        .map_err(|e| format!("Failed to extract java archive: {e}"))?;

    // Remove zip file to free disk space
    let _ = fs::remove_file(zip_dest);

    // Find the newly extracted java binary
    if let Some(bin) = find_java_in_dir(&runtime_dir) {
        Ok(bin)
    } else {
        Err(format!(
            "Failed to find java binary in extracted directory: {}",
            runtime_dir.display()
        ))
    }
}
