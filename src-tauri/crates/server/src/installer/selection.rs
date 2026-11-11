use super::providers::{InstallerEndpoints, QuiltGameVersion};
use std::collections::BTreeMap;

pub(super) struct InstallerDownloadPlan {
    pub(super) url: String,
    pub(super) installer_name: &'static str,
    pub(super) args: Vec<String>,
}

/// Compares versions number by number ("21.1.100" > "21.1.99").
pub(super) fn version_key(version: &str) -> Vec<u32> {
    version
        .split(['.', '-'])
        .filter_map(|part| part.parse().ok())
        .collect()
}

pub(super) fn newest_first(mut games: Vec<String>) -> Vec<String> {
    games.sort_by_key(|game| std::cmp::Reverse(version_key(game)));
    games.dedup();
    games
}

/// NeoForge's version for a Minecraft version: "21.1.77" is 1.21.1, "21.0.x" is 1.21 and,
/// since the year-based versions, "26.1.2.111" is 26.1.2 and "26.3.0.5-beta" is 26.3.
/// None for snapshots, April Fools versions and alphas.
pub(super) fn neoforge_game_version(neoforge: &str) -> Option<String> {
    if neoforge.contains("alpha") || neoforge.contains('+') {
        return None;
    }
    let numbers = neoforge.split('-').next()?;
    let parts: Vec<u32> = numbers
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    match parts.as_slice() {
        [major, minor, patch, _] if *major >= 25 => Some(if *patch == 0 {
            format!("{major}.{minor}")
        } else {
            format!("{major}.{minor}.{patch}")
        }),
        [major, minor, _] if (20..25).contains(major) => Some(if *minor == 0 {
            format!("1.{major}")
        } else {
            format!("1.{major}.{minor}")
        }),
        _ => None,
    }
}

/// NeoForge for this Minecraft version: newest stable, else newest beta.
pub(super) fn pick_neoforge(versions: &[String], game_version: &str) -> Option<String> {
    versions
        .iter()
        .filter(|version| neoforge_game_version(version).as_deref() == Some(game_version))
        .max_by_key(|version| (!version.contains('-'), version_key(version)))
        .cloned()
}

/// Forge from 1.17, the first version started from an argument file.
pub(super) fn forge_supported(game_version: &str) -> bool {
    let key = version_key(game_version);
    key.first().is_some_and(|&major| major > 1)
        || (key.first() == Some(&1) && key.get(1).is_some_and(|&minor| minor >= 17))
}

/// Forge for this Minecraft version: the recommended build, else the latest ("1.20.1-47.4.10").
pub(super) fn pick_forge(promos: &BTreeMap<String, String>, game_version: &str) -> Option<String> {
    ["recommended", "latest"]
        .iter()
        .find_map(|kind| promos.get(&format!("{game_version}-{kind}")))
        .map(|build| format!("{game_version}-{build}"))
}

pub(super) fn select_neoforge_game_versions(versions: &[String]) -> Vec<String> {
    newest_first(
        versions
            .iter()
            .filter_map(|version| neoforge_game_version(version))
            .collect(),
    )
}

pub(super) fn select_forge_game_versions(promos: &BTreeMap<String, String>) -> Vec<String> {
    newest_first(
        promos
            .keys()
            .filter_map(|key| key.rsplit_once('-').map(|(game, _)| game.to_string()))
            .filter(|game| forge_supported(game))
            .collect(),
    )
}

pub(super) fn select_quilt_game_versions(versions: &[QuiltGameVersion]) -> Vec<String> {
    newest_first(
        versions
            .iter()
            .filter(|entry| entry.stable)
            .map(|entry| entry.version.clone())
            .collect(),
    )
}

pub(super) fn neoforge_install_plan(
    endpoints: &InstallerEndpoints,
    version: &str,
) -> InstallerDownloadPlan {
    InstallerDownloadPlan {
        url: endpoints.neoforge_installer_url(version),
        installer_name: "neoforge-installer.jar",
        args: vec!["--installServer".into()],
    }
}

pub(super) fn forge_install_plan(
    endpoints: &InstallerEndpoints,
    version: &str,
) -> InstallerDownloadPlan {
    InstallerDownloadPlan {
        url: endpoints.forge_installer_url(version),
        installer_name: "forge-installer.jar",
        args: vec!["--installServer".into()],
    }
}

pub(super) fn quilt_install_plan(
    installer_url: String,
    game_version: &str,
    pinned: Option<&str>,
) -> InstallerDownloadPlan {
    let mut args = vec!["install".into(), "server".into(), game_version.to_owned()];
    args.extend(pinned.map(str::to_owned));
    args.extend(["--download-server".into(), "--install-dir=.".into()]);
    InstallerDownloadPlan {
        url: installer_url,
        installer_name: "quilt-installer.jar",
        args,
    }
}
