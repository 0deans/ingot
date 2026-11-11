//! Version and loader parser for vanilla and legacy profile version strings.
use ingot_minecraft::instance::ModLoaderType;

/// Cleans and normalizes a raw loader version string by trimming whitespace,
/// stripping redundant game version prefix/suffix, and stripping redundant loader prefixes.
pub(crate) fn clean_loader_version(loader_ver: &str, game_version: &str) -> String {
    ingot_minecraft::loader::normalize_loader_version(game_version, loader_ver)
}

/// Parses a version ID string (from launcher profiles or legacy version JSONs)
/// into (game_version, loader_type, loader_version).
pub(crate) fn parse_vanilla_version_id(
    version_id: &str,
) -> (String, ModLoaderType, Option<String>) {
    let lower = version_id.to_ascii_lowercase();

    // 1. Fabric: e.g. "fabric-loader-0.15.0-1.20.1" or "fabric-loader-1.20.1"
    if let Some(pos) = lower.find("fabric-loader-") {
        let rest = &version_id[pos + "fabric-loader-".len()..];
        if let Some((ldr_ver, mc_ver)) = rest.split_once('-') {
            let cleaned = clean_loader_version(ldr_ver, mc_ver);
            return (
                mc_ver.to_string(),
                ModLoaderType::Fabric,
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                },
            );
        }
        return (rest.to_string(), ModLoaderType::Fabric, None);
    }

    // 2. Quilt: e.g. "quilt-loader-0.20.0-1.20.1" or "quilt-loader-1.20.1"
    if let Some(pos) = lower.find("quilt-loader-") {
        let rest = &version_id[pos + "quilt-loader-".len()..];
        if let Some((ldr_ver, mc_ver)) = rest.split_once('-') {
            let cleaned = clean_loader_version(ldr_ver, mc_ver);
            return (
                mc_ver.to_string(),
                ModLoaderType::Quilt,
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                },
            );
        }
        return (rest.to_string(), ModLoaderType::Quilt, None);
    }

    // 3. Forge:
    // e.g. "1.7.10-Forge10.13.4.1614-1.7.10", "1.12.2-forge14.23.5.2860", "1.20.1-forge-47.2.0"
    if let Some(pos) = lower.find("-forge") {
        let mc_ver = &version_id[..pos];
        let after = &version_id[pos + "-forge".len()..];
        let raw_ldr = after.trim_start_matches('-');
        let cleaned = clean_loader_version(raw_ldr, mc_ver);
        return (
            mc_ver.to_string(),
            ModLoaderType::Forge,
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            },
        );
    }

    // e.g. "forge-1.20.1-47.2.0" or "forge-1.20.1"
    if lower.starts_with("forge-") {
        let rest = &version_id["forge-".len()..];
        if let Some((mc_ver, ldr_ver)) = rest.split_once('-') {
            let cleaned = clean_loader_version(ldr_ver, mc_ver);
            return (
                mc_ver.to_string(),
                ModLoaderType::Forge,
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                },
            );
        }
        return (rest.to_string(), ModLoaderType::Forge, None);
    }

    // 4. NeoForge:
    // e.g. "1.20.4-neoforge-20.4.80" or "1.20.4-neoforge20.4.80"
    if let Some(pos) = lower.find("-neoforge") {
        let mc_ver = &version_id[..pos];
        let after = &version_id[pos + "-neoforge".len()..];
        let raw_ldr = after.trim_start_matches('-');
        let cleaned = clean_loader_version(raw_ldr, mc_ver);
        return (
            mc_ver.to_string(),
            ModLoaderType::NeoForge,
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            },
        );
    }

    // e.g. "neoforge-20.4.80" or "neoforge-1.21.1-21.1.0"
    if lower.starts_with("neoforge-") {
        let rest = &version_id["neoforge-".len()..];
        if let Some((mc_ver, ldr_ver)) = rest.split_once('-') {
            if mc_ver.starts_with("1.") {
                let cleaned = clean_loader_version(ldr_ver, mc_ver);
                return (
                    mc_ver.to_string(),
                    ModLoaderType::NeoForge,
                    if cleaned.is_empty() {
                        None
                    } else {
                        Some(cleaned)
                    },
                );
            }
        }
        let cleaned = clean_loader_version(rest, "1.21.1");
        return (
            "1.21.1".to_string(),
            ModLoaderType::NeoForge,
            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            },
        );
    }

    (version_id.to_string(), ModLoaderType::Vanilla, None)
}
