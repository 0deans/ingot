//! Platform filesystem discovery and launcher detection.
pub(super) mod rules;

use super::{
    formats::{
        count_instance_stats, scan_atlauncher_instances, scan_curseforge_instances,
        scan_ftb_instances, scan_legacy_instances, scan_modrinth_instances, scan_technic_instances,
        scan_vanilla_instances,
    },
    models::{DetectedLauncher, ImportError, ImportableInstance},
};
pub use rules::DiscoveryRoots;
use rules::{
    build_custom_fallback_instance, custom_probe_order, CustomInstanceMarkers, CustomProbeKind,
    LauncherKind,
};
use std::path::{Path, PathBuf};

fn scan_launcher_instances(kind: LauncherKind, dir: &Path) -> Vec<ImportableInstance> {
    match kind {
        LauncherKind::CurseForge => scan_curseforge_instances(dir),
        LauncherKind::Modrinth => scan_modrinth_instances(dir),
        LauncherKind::AtLauncher => scan_atlauncher_instances(dir),
        LauncherKind::Ftb => scan_ftb_instances(dir),
        LauncherKind::Technic => scan_technic_instances(dir),
        LauncherKind::LegacyLauncher => scan_legacy_instances(dir),
        LauncherKind::Vanilla => scan_vanilla_instances(dir),
    }
}

pub fn get_detected_launchers(roots: &DiscoveryRoots) -> Vec<DetectedLauncher> {
    LauncherKind::ALL
        .into_iter()
        .map(|kind| {
            let found_dir = kind
                .candidate_dirs(roots)
                .into_iter()
                .find(|dir| dir.exists());
            let count = found_dir
                .as_deref()
                .map(|dir| scan_launcher_instances(kind, dir).len() as u32)
                .unwrap_or(0);
            kind.to_detected_launcher(found_dir.as_deref(), count)
        })
        .collect()
}

pub fn get_launcher_instances(
    roots: &DiscoveryRoots,
    launcher_id: &str,
    custom_path: Option<&str>,
) -> Result<Vec<ImportableInstance>, ImportError> {
    let kind = LauncherKind::from_id(launcher_id)
        .ok_or_else(|| ImportError::UnknownLauncher(launcher_id.to_string()))?;

    let path = custom_path
        .map(PathBuf::from)
        .or_else(|| kind.candidate_dirs(roots).into_iter().find(|p| p.exists()));

    Ok(path
        .map(|dir| scan_launcher_instances(kind, &dir))
        .unwrap_or_default())
}

fn probe_custom_markers(path: &Path) -> CustomInstanceMarkers {
    CustomInstanceMarkers {
        has_minecraftinstance_json: path.join("minecraftinstance.json").exists(),
        has_profile_json: path.join("profile.json").exists(),
        has_metadata_json: path.join("metadata.json").exists(),
        has_instance_json: path.join("instance.json").exists(),
        has_bin_dir: path.join("bin").exists(),
        has_versions_dir: path.join("versions").exists(),
    }
}

fn run_custom_probe(
    kind: CustomProbeKind,
    path: &Path,
    path_str: &str,
) -> Option<ImportableInstance> {
    let parent = path.parent().unwrap_or(path);
    match kind {
        CustomProbeKind::CurseForge => scan_curseforge_instances(parent)
            .into_iter()
            .find(|i| i.source_path == path_str),
        CustomProbeKind::Modrinth => scan_modrinth_instances(parent)
            .into_iter()
            .find(|i| i.source_path == path_str),
        CustomProbeKind::Ftb => scan_ftb_instances(parent)
            .into_iter()
            .find(|i| i.source_path == path_str),
        CustomProbeKind::AtLauncher => scan_atlauncher_instances(parent)
            .into_iter()
            .find(|i| i.source_path == path_str),
        CustomProbeKind::Technic => scan_technic_instances(parent)
            .into_iter()
            .find(|i| i.source_path == path_str),
        CustomProbeKind::Legacy => scan_legacy_instances(path).into_iter().next(),
    }
}

pub fn detect_custom_instance(path_str: &str) -> Result<ImportableInstance, ImportError> {
    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Err(ImportError::SourceNotFound(path_str.to_string()));
    }

    let markers = probe_custom_markers(&path);
    for probe in custom_probe_order(markers) {
        if let Some(inst) = run_custom_probe(probe, &path, path_str) {
            return Ok(inst);
        }
    }

    let (mods_count, saves_count, screenshots_count) = count_instance_stats(&path);
    Ok(build_custom_fallback_instance(
        &path,
        path_str,
        format!("custom-{}", uuid::Uuid::new_v4()),
        mods_count,
        saves_count,
        screenshots_count,
    ))
}
