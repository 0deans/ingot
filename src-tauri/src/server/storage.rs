//! Disk space used by a server's folder, split into what the user cares about
//! (worlds, plugins, logs...), plus the free space left on the device.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StorageCategory {
    /// "worlds", "plugins", "logs", "backups", "server" or "other"
    pub id: String,
    #[specta(type = i32)]
    pub bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerStorage {
    #[specta(type = i32)]
    pub total_bytes: f64,
    /// Largest first, empty ones left out
    pub categories: Vec<StorageCategory>,
    /// Individual worlds (overworld, nether...) by size
    pub worlds: Vec<WorldSize>,
    #[specta(type = Option<i32>)]
    pub device_free_bytes: Option<f64>,
    #[specta(type = Option<i32>)]
    pub device_total_bytes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldSize {
    pub name: String,
    #[specta(type = i32)]
    pub bytes: f64,
}

/// Total size of a file or directory tree (symlinks not followed)
fn size_of(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if !meta.is_dir() {
        return meta.len();
    }
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                total += meta.len();
            }
        }
    }
    total
}

fn category_of(name: &str, path: &Path) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if path.join("level.dat").is_file() {
        return "worlds";
    }
    match lower.as_str() {
        "plugins" | "mods" | "config" | "datapacks" => "plugins",
        "logs" | "crash-reports" | "debug" => "logs",
        "backups" | "world-backups" => "backups",
        "libraries" | "versions" | "cache" | "bundler" | ".fabric" | ".paper-remapped" => "server",
        _ if lower.ends_with(".jar") => "server",
        _ => "other",
    }
}

/// Free/total space of the disk holding `path` (longest matching mount point)
fn device_space(path: &Path) -> Option<(u64, u64)> {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    // Windows canonical paths start with \\?\, mount points don't
    let path_str = path.to_string_lossy().trim_start_matches(r"\\?\").to_ascii_lowercase();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|d| {
            let mount = d.mount_point().to_string_lossy().to_ascii_lowercase();
            path_str.starts_with(&mount)
        })
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| (d.available_space(), d.total_space()))
}

pub fn storage(server_dir: &Path) -> ServerStorage {
    let mut sums: Vec<(&'static str, u64)> = Vec::new();
    let mut worlds = Vec::new();
    if let Ok(entries) = std::fs::read_dir(server_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let bytes = size_of(&path);
            let category = category_of(&name, &path);
            if category == "worlds" {
                worlds.push(WorldSize { name: name.clone(), bytes: bytes as f64 });
            }
            match sums.iter_mut().find(|(id, _)| *id == category) {
                Some((_, sum)) => *sum += bytes,
                None => sums.push((category, bytes)),
            }
        }
    }
    sums.retain(|(_, bytes)| *bytes > 0);
    sums.sort_by(|a, b| b.1.cmp(&a.1));
    worlds.sort_by(|a, b| b.bytes.total_cmp(&a.bytes));
    let device = device_space(server_dir);
    ServerStorage {
        total_bytes: sums.iter().map(|(_, b)| *b).sum::<u64>() as f64,
        categories: sums
            .into_iter()
            .map(|(id, bytes)| StorageCategory { id: id.to_string(), bytes: bytes as f64 })
            .collect(),
        worlds,
        device_free_bytes: device.map(|(free, _)| free as f64),
        device_total_bytes: device.map(|(_, total)| total as f64),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sizes_categories() {
        let dir = std::env::temp_dir().join(format!("ingot-storage-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("world/region")).unwrap();
        std::fs::write(dir.join("world/level.dat"), [0u8; 10]).unwrap();
        std::fs::write(dir.join("world/region/r.0.0.mca"), [0u8; 1000]).unwrap();
        std::fs::create_dir_all(dir.join("plugins")).unwrap();
        std::fs::write(dir.join("plugins/a.jar"), [0u8; 300]).unwrap();
        std::fs::write(dir.join("server.jar"), [0u8; 500]).unwrap();
        let s = super::storage(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(s.total_bytes, 1810.0);
        assert_eq!(s.categories[0].id, "worlds");
        assert_eq!(s.worlds[0].bytes, 1010.0);
        assert!(s.device_total_bytes.unwrap_or(0.0) > 0.0);
    }
}
