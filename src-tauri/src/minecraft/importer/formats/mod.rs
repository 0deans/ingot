//! Format parsers for external Minecraft launcher installations.
pub(crate) mod atlauncher;
pub(crate) mod curseforge;
pub(crate) mod legacy;
pub(crate) mod modrinth;
pub(crate) mod vanilla;

use super::models::ImportableInstance;
use std::fs;
use std::path::Path;

pub(crate) use atlauncher::scan_atlauncher_instances;
pub(crate) use curseforge::scan_curseforge_instances;
pub(crate) use legacy::scan_legacy_instances;
pub(crate) use modrinth::scan_modrinth_instances;
pub(crate) use vanilla::scan_vanilla_instances;

/// Counts files directly inside `dir`, optionally filtering by extension.
pub(crate) fn count_files_in_dir(dir: &Path, extension: Option<&str>) -> u32 {
    if !dir.exists() || !dir.is_dir() {
        return 0;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        entries
            .flatten()
            .filter(|e| {
                if let Ok(ft) = e.file_type() {
                    if ft.is_file() {
                        if let Some(ext) = extension {
                            return e
                                .path()
                                .extension()
                                .and_then(|s| s.to_str())
                                .map(|s| s.eq_ignore_ascii_case(ext))
                                .unwrap_or(false);
                        }
                        return true;
                    }
                }
                false
            })
            .count() as u32
    } else {
        0
    }
}

/// Counts direct subdirectories inside `dir`.
pub(crate) fn count_subdirectories(dir: &Path) -> u32 {
    if !dir.exists() || !dir.is_dir() {
        return 0;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        entries
            .flatten()
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .count() as u32
    } else {
        0
    }
}

/// Helper to count mods (jars), saves (subdirectories), and screenshots (pngs).
pub(crate) fn count_instance_stats(dir: &Path) -> (u32, u32, u32) {
    let mods = count_files_in_dir(&dir.join("mods"), Some("jar"));
    let saves = count_subdirectories(&dir.join("saves"));
    let screenshots = count_files_in_dir(&dir.join("screenshots"), Some("png"));
    (mods, saves, screenshots)
}
