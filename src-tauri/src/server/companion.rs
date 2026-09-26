//! The Ingot companion plugin (Paper, Purpur, Folia): feeds the live map from server
//! memory so it never has to save the world. Built by `scripts/build-companion.mjs`
//! and embedded here. Shown as a system plugin the user can remove and install again;
//! only installed automatically when a server is created.

use serde::{Deserialize, Serialize};
use std::path::Path;

use super::config::ServerCoreType;
use super::plugins;

const JAR: &[u8] = include_bytes!("../../companion/ingot-companion.jar");
/// Must match VERSION in scripts/build-companion.mjs (checked by a test)
pub const BUNDLED_VERSION: &str = "1.0.0";
/// `name` in the plugin's plugin.yml; identifies it however the jar is named
pub const PLUGIN_NAME: &str = "Ingot";
const FILE_NAME: &str = "ingot-companion.jar";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompanionStatus {
    /// Paper, Purpur or Folia on 1.20.1 or newer
    pub supported: bool,
    /// Jar file name when installed (may be disabled)
    pub file_name: Option<String>,
    pub enabled: bool,
    pub installed_version: Option<String>,
    pub bundled_version: String,
}

/// Paper-family servers new enough for the plugin's API (region schedulers, 1.20.1+)
pub fn supported(core: &ServerCoreType, game_version: &str) -> bool {
    if !matches!(core, ServerCoreType::Paper | ServerCoreType::Purpur | ServerCoreType::Folia) {
        return false;
    }
    let parts: Vec<u32> = game_version.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let part = |i: usize| parts.get(i).copied().unwrap_or(0);
    // Year-based versions (26.1+) come after 1.x
    part(0) > 1 || (part(0) == 1 && (part(1) > 20 || (part(1) == 20 && part(2) >= 1)))
}

pub fn status(server_dir: &Path, core: &ServerCoreType, game_version: &str) -> CompanionStatus {
    let installed = plugins::list_installed(server_dir, core)
        .unwrap_or_default()
        .into_iter()
        .find(|p| p.system);
    CompanionStatus {
        supported: supported(core, game_version),
        file_name: installed.as_ref().map(|p| p.file_name.clone()),
        enabled: installed.as_ref().is_some_and(|p| p.enabled),
        installed_version: installed.and_then(|p| p.version),
        bundled_version: BUNDLED_VERSION.to_string(),
    }
}

/// Installs or updates the plugin. Replaces an existing copy (enabled or not) so there's
/// only ever one. Takes effect on the next server start.
pub fn install(server_dir: &Path, core: &ServerCoreType, game_version: &str) -> Result<(), String> {
    if !supported(core, game_version) {
        return Err("The Ingot plugin needs Paper, Purpur or Folia 1.20.1 or newer".into());
    }
    let dir = server_dir.join("plugins");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create plugins folder: {e}"))?;
    for existing in plugins::list_installed(server_dir, core)?.into_iter().filter(|p| p.system) {
        let _ = std::fs::remove_file(dir.join(&existing.file_name));
    }
    let tmp = dir.join(format!("{FILE_NAME}.part"));
    std::fs::write(&tmp, JAR).map_err(|e| format!("Failed to install the Ingot plugin: {e}"))?;
    std::fs::rename(&tmp, dir.join(FILE_NAME)).map_err(|e| format!("Failed to install the Ingot plugin: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_jar_matches_version() {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(JAR)).unwrap();
        let mut yml = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("plugin.yml").unwrap(), &mut yml).unwrap();
        assert!(yml.contains(&format!("name: {PLUGIN_NAME}\n")));
        assert!(yml.contains(&format!("version: \"{BUNDLED_VERSION}\"")), "rebuild with scripts/build-companion.mjs");
    }

    #[test]
    fn supports_paper_family_from_1_20_1() {
        assert!(supported(&ServerCoreType::Paper, "26.2"));
        assert!(supported(&ServerCoreType::Folia, "1.21.4"));
        assert!(supported(&ServerCoreType::Purpur, "1.20.1"));
        assert!(!supported(&ServerCoreType::Paper, "1.20"));
        assert!(!supported(&ServerCoreType::Paper, "1.19.4"));
        assert!(!supported(&ServerCoreType::Fabric, "26.2"));
        assert!(!supported(&ServerCoreType::Vanilla, "26.2"));
    }
}
