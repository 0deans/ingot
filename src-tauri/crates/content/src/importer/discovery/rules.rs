//! Pure launcher discovery roots, metadata descriptors, and custom probe rules.
use super::super::models::{DetectedLauncher, ImportableInstance};
use ingot_minecraft::instance::ModLoaderType;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRoots {
    pub app_data: Option<PathBuf>,
    pub home: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LauncherKind {
    CurseForge,
    Modrinth,
    AtLauncher,
    Ftb,
    Technic,
    LegacyLauncher,
    Vanilla,
}

impl LauncherKind {
    pub(super) const ALL: [Self; 7] = [
        Self::CurseForge,
        Self::Modrinth,
        Self::AtLauncher,
        Self::Ftb,
        Self::Technic,
        Self::LegacyLauncher,
        Self::Vanilla,
    ];

    pub(super) fn from_id(id: &str) -> Option<Self> {
        match id {
            "curseforge" => Some(Self::CurseForge),
            "modrinth" => Some(Self::Modrinth),
            "atlauncher" => Some(Self::AtLauncher),
            "ftb" => Some(Self::Ftb),
            "technic" => Some(Self::Technic),
            "legacylauncher" => Some(Self::LegacyLauncher),
            "vanilla" => Some(Self::Vanilla),
            _ => None,
        }
    }

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::CurseForge => "curseforge",
            Self::Modrinth => "modrinth",
            Self::AtLauncher => "atlauncher",
            Self::Ftb => "ftb",
            Self::Technic => "technic",
            Self::LegacyLauncher => "legacylauncher",
            Self::Vanilla => "vanilla",
        }
    }

    pub(super) fn display_name(self) -> &'static str {
        match self {
            Self::CurseForge => "CurseForge",
            Self::Modrinth => "Modrinth App",
            Self::AtLauncher => "ATLauncher",
            Self::Ftb => "FTB App",
            Self::Technic => "Technic Launcher",
            Self::LegacyLauncher => "LegacyLauncher",
            Self::Vanilla => "Vanilla Minecraft Launcher",
        }
    }

    pub(super) fn icon(self) -> &'static str {
        self.id()
    }

    pub(super) fn candidate_dirs(self, roots: &DiscoveryRoots) -> Vec<PathBuf> {
        match self {
            Self::CurseForge => get_curseforge_dirs(roots),
            Self::Modrinth => get_modrinth_dirs(roots),
            Self::AtLauncher => get_atlauncher_dirs(roots),
            Self::Ftb => get_ftb_dirs(roots),
            Self::Technic => get_technic_dirs(roots),
            Self::LegacyLauncher => get_legacylauncher_dirs(roots),
            Self::Vanilla => get_vanilla_dirs(roots),
        }
    }

    pub(super) fn to_detected_launcher(
        self,
        found_dir: Option<&Path>,
        instances_count: u32,
    ) -> DetectedLauncher {
        DetectedLauncher {
            id: self.id().to_string(),
            name: self.display_name().to_string(),
            icon: self.icon().to_string(),
            path: found_dir
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            instances_count,
            available: found_dir.is_some(),
        }
    }
}

fn get_curseforge_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = roots.home.as_ref() {
        dirs.push(home.join("curseforge").join("minecraft").join("Instances"));
        dirs.push(home.join("CurseForge").join("minecraft").join("Instances"));
        dirs.push(
            home.join("Documents")
                .join("curseforge")
                .join("minecraft")
                .join("Instances"),
        );
        dirs.push(
            home.join("Documents")
                .join("CurseForge")
                .join("minecraft")
                .join("Instances"),
        );
    }
    dirs
}

fn get_modrinth_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.as_ref() {
        dirs.push(appdata.join("com.modrinth.theseus").join("profiles"));
        dirs.push(appdata.join("Modrinth App").join("profiles"));
    }
    if let Some(home) = roots.home.as_ref() {
        dirs.push(
            home.join(".local")
                .join("share")
                .join("com.modrinth.theseus")
                .join("profiles"),
        );
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("com.modrinth.theseus")
                .join("profiles"),
        );
    }
    dirs
}

fn get_atlauncher_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.as_ref() {
        dirs.push(appdata.join("ATLauncher").join("instances"));
    }
    if let Some(home) = roots.home.as_ref() {
        dirs.push(home.join(".atlauncher").join("instances"));
        dirs.push(home.join("ATLauncher").join("instances"));
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("ATLauncher")
                .join("instances"),
        );
    }
    dirs
}

fn get_ftb_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.as_ref() {
        dirs.push(appdata.join("FTBApp").join("instances"));
        dirs.push(appdata.join(".ftba").join("instances"));
    }
    if let Some(home) = roots.home.as_ref() {
        dirs.push(home.join(".ftba").join("instances"));
        dirs.push(home.join(".ftbapp").join("instances"));
        dirs.push(home.join("FTBApp").join("instances"));
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("FTBApp")
                .join("instances"),
        );
    }
    dirs
}

fn get_technic_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.as_ref() {
        dirs.push(appdata.join(".technic").join("modpacks"));
        dirs.push(appdata.join("technic").join("modpacks"));
    }
    if let Some(home) = roots.home.as_ref() {
        dirs.push(home.join(".technic").join("modpacks"));
        dirs.push(home.join("technic").join("modpacks"));
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("technic")
                .join("modpacks"),
        );
    }
    dirs
}

fn get_legacylauncher_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.as_ref() {
        dirs.push(
            appdata
                .join(".tlauncher")
                .join("legacy")
                .join("Minecraft")
                .join("game"),
        );
        dirs.push(appdata.join(".minecraft"));
    }
    if let Some(home) = roots.home.as_ref() {
        dirs.push(
            home.join(".tlauncher")
                .join("legacy")
                .join("Minecraft")
                .join("game"),
        );
        dirs.push(home.join(".minecraft"));
    }
    dirs
}

fn get_vanilla_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.as_ref() {
        dirs.push(appdata.join(".minecraft"));
    }
    if let Some(home) = roots.home.as_ref() {
        dirs.push(home.join(".minecraft"));
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("minecraft"),
        );
    }
    dirs
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct CustomInstanceMarkers {
    pub(super) has_minecraftinstance_json: bool,
    pub(super) has_profile_json: bool,
    pub(super) has_metadata_json: bool,
    pub(super) has_instance_json: bool,
    pub(super) has_bin_dir: bool,
    pub(super) has_versions_dir: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CustomProbeKind {
    CurseForge,
    Modrinth,
    Ftb,
    AtLauncher,
    Technic,
    Legacy,
}

pub(super) fn custom_probe_order(markers: CustomInstanceMarkers) -> Vec<CustomProbeKind> {
    let mut order = Vec::new();
    if markers.has_minecraftinstance_json {
        order.push(CustomProbeKind::CurseForge);
    }
    if markers.has_profile_json || markers.has_metadata_json {
        order.push(CustomProbeKind::Modrinth);
    }
    if markers.has_instance_json {
        order.push(CustomProbeKind::Ftb);
        order.push(CustomProbeKind::AtLauncher);
    }
    if markers.has_bin_dir {
        order.push(CustomProbeKind::Technic);
    }
    if markers.has_versions_dir {
        order.push(CustomProbeKind::Legacy);
    }
    order
}

pub(super) fn build_custom_fallback_instance(
    path: &Path,
    source_path: &str,
    id: String,
    mods_count: u32,
    saves_count: u32,
    screenshots_count: u32,
) -> ImportableInstance {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Custom Instance")
        .to_string();

    ImportableInstance {
        id,
        launcher_id: "custom".to_string(),
        name,
        game_version: "1.20.1".to_string(),
        loader: if mods_count > 0 {
            ModLoaderType::Fabric
        } else {
            ModLoaderType::Vanilla
        },
        loader_version: None,
        icon: None,
        source_path: source_path.to_string(),
        mods_count,
        saves_count,
        screenshots_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_roots_produce_expected_candidate_dirs() {
        let empty = DiscoveryRoots {
            app_data: None,
            home: None,
        };
        assert!(get_curseforge_dirs(&empty).is_empty());
        assert!(get_modrinth_dirs(&empty).is_empty());
        assert!(get_atlauncher_dirs(&empty).is_empty());
        assert!(get_ftb_dirs(&empty).is_empty());
        assert!(get_technic_dirs(&empty).is_empty());
        assert!(get_legacylauncher_dirs(&empty).is_empty());
        assert!(get_vanilla_dirs(&empty).is_empty());

        let app_only = DiscoveryRoots {
            app_data: Some(PathBuf::from("/appdata")),
            home: None,
        };
        assert!(get_curseforge_dirs(&app_only).is_empty());
        assert_eq!(
            get_modrinth_dirs(&app_only),
            vec![
                PathBuf::from("/appdata/com.modrinth.theseus/profiles"),
                PathBuf::from("/appdata/Modrinth App/profiles"),
            ]
        );
        assert_eq!(
            get_vanilla_dirs(&app_only),
            vec![PathBuf::from("/appdata/.minecraft")]
        );

        let full = DiscoveryRoots {
            app_data: Some(PathBuf::from("/appdata")),
            home: Some(PathBuf::from("/home/user")),
        };
        assert_eq!(get_curseforge_dirs(&full).len(), 4);
        assert_eq!(get_modrinth_dirs(&full).len(), 4);
        assert_eq!(get_atlauncher_dirs(&full).len(), 4);
        assert_eq!(get_ftb_dirs(&full).len(), 6);
        assert_eq!(get_technic_dirs(&full).len(), 5);
        assert_eq!(get_legacylauncher_dirs(&full).len(), 4);
        assert_eq!(get_vanilla_dirs(&full).len(), 3);
    }

    #[test]
    fn launcher_kind_lookup_and_descriptor_mapping() {
        assert_eq!(LauncherKind::ALL.len(), 7);
        for kind in LauncherKind::ALL {
            assert_eq!(LauncherKind::from_id(kind.id()), Some(kind));
            assert_eq!(kind.icon(), kind.id());
        }
        assert_eq!(LauncherKind::from_id("unknown"), None);

        let unavailable = LauncherKind::Vanilla.to_detected_launcher(None, 0);
        assert_eq!(unavailable.id, "vanilla");
        assert_eq!(unavailable.name, "Vanilla Minecraft Launcher");
        assert!(!unavailable.available);
        assert_eq!(unavailable.path, "");
        assert_eq!(unavailable.instances_count, 0);

        let path = Path::new("/tmp/instances");
        let available = LauncherKind::CurseForge.to_detected_launcher(Some(path), 3);
        assert_eq!(available.id, "curseforge");
        assert_eq!(available.name, "CurseForge");
        assert!(available.available);
        assert_eq!(available.path, path.to_string_lossy());
        assert_eq!(available.instances_count, 3);
    }

    #[test]
    fn custom_probe_order_respects_markers_and_precedence() {
        assert!(custom_probe_order(CustomInstanceMarkers::default()).is_empty());

        let instance_only = CustomInstanceMarkers {
            has_instance_json: true,
            ..Default::default()
        };
        assert_eq!(
            custom_probe_order(instance_only),
            vec![CustomProbeKind::Ftb, CustomProbeKind::AtLauncher]
        );

        let metadata_only = CustomInstanceMarkers {
            has_metadata_json: true,
            ..Default::default()
        };
        assert_eq!(
            custom_probe_order(metadata_only),
            vec![CustomProbeKind::Modrinth]
        );

        let all_markers = CustomInstanceMarkers {
            has_minecraftinstance_json: true,
            has_profile_json: true,
            has_metadata_json: true,
            has_instance_json: true,
            has_bin_dir: true,
            has_versions_dir: true,
        };
        assert_eq!(
            custom_probe_order(all_markers),
            vec![
                CustomProbeKind::CurseForge,
                CustomProbeKind::Modrinth,
                CustomProbeKind::Ftb,
                CustomProbeKind::AtLauncher,
                CustomProbeKind::Technic,
                CustomProbeKind::Legacy,
            ]
        );
    }

    #[test]
    fn custom_fallback_instance_sets_loader_and_fallback_name() {
        let modded = build_custom_fallback_instance(
            Path::new("/packs/MyPack"),
            "/packs/MyPack",
            "custom-1".to_string(),
            5,
            2,
            1,
        );
        assert_eq!(modded.id, "custom-1");
        assert_eq!(modded.launcher_id, "custom");
        assert_eq!(modded.name, "MyPack");
        assert_eq!(modded.game_version, "1.20.1");
        assert_eq!(modded.loader, ModLoaderType::Fabric);
        assert_eq!(modded.mods_count, 5);
        assert_eq!(modded.saves_count, 2);
        assert_eq!(modded.screenshots_count, 1);

        let empty_name = build_custom_fallback_instance(
            Path::new(""),
            "/tmp/fallback",
            "custom-2".to_string(),
            0,
            0,
            0,
        );
        assert_eq!(empty_name.name, "Custom Instance");
        assert_eq!(empty_name.loader, ModLoaderType::Vanilla);
    }
}
