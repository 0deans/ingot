//! Version and loader parser for vanilla and legacy profile version strings.
use crate::minecraft::instance::ModLoaderType;

/// Parses a version ID string (from launcher profiles or legacy version JSONs)
/// into (game_version, loader_type, loader_version).
pub(crate) fn parse_vanilla_version_id(
    version_id: &str,
) -> (String, ModLoaderType, Option<String>) {
    if let Some(rest) = version_id.strip_prefix("fabric-loader-") {
        if let Some((ldr_ver, mc_ver)) = rest.split_once('-') {
            return (
                mc_ver.to_string(),
                ModLoaderType::Fabric,
                Some(ldr_ver.to_string()),
            );
        }
        return (rest.to_string(), ModLoaderType::Fabric, None);
    }

    if let Some(rest) = version_id.strip_prefix("quilt-loader-") {
        if let Some((ldr_ver, mc_ver)) = rest.split_once('-') {
            return (
                mc_ver.to_string(),
                ModLoaderType::Quilt,
                Some(ldr_ver.to_string()),
            );
        }
        return (rest.to_string(), ModLoaderType::Quilt, None);
    }

    if let Some((mc_ver, ldr_ver)) = version_id.split_once("-forge-") {
        return (
            mc_ver.to_string(),
            ModLoaderType::Forge,
            Some(ldr_ver.to_string()),
        );
    }

    if let Some(rest) = version_id.strip_prefix("forge-") {
        if let Some((mc_ver, ldr_ver)) = rest.split_once('-') {
            return (
                mc_ver.to_string(),
                ModLoaderType::Forge,
                Some(ldr_ver.to_string()),
            );
        }
        return (rest.to_string(), ModLoaderType::Forge, None);
    }

    if let Some(rest) = version_id.strip_prefix("neoforge-") {
        return (
            "1.21.1".to_string(),
            ModLoaderType::NeoForge,
            Some(rest.to_string()),
        );
    }

    (version_id.to_string(), ModLoaderType::Vanilla, None)
}
