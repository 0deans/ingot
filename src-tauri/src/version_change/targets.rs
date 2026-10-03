//! What a version change covers for an instance and for a server

use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::sources::Scope;
use super::{ContentKind, Folder, Target};
use crate::server::config::ServerCoreType;

/// The lowercase name a loader or core is stored as ("fabric", "neoforge", "paper")
pub fn loader_id<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

pub fn parse_loader<T: DeserializeOwned>(id: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(id.to_string())).map_err(|_| format!("Unknown loader: {id}"))
}

/// Mods for this loader on Modrinth and CurseForge (Quilt also runs Fabric mods)
fn mod_scope(loader: &str) -> Scope {
    let (modrinth, cf_loader, cf_loader_names): (Vec<&'static str>, Option<u32>, Vec<&'static str>) = match loader {
        "fabric" => (vec!["fabric"], Some(4), vec!["Fabric"]),
        "quilt" => (vec!["quilt", "fabric"], None, vec!["Quilt", "Fabric"]),
        "neoforge" => (vec!["neoforge"], Some(6), vec!["NeoForge"]),
        "forge" => (vec!["forge"], Some(1), vec!["Forge"]),
        // Vanilla loads no mods: an empty scope turns them all off
        _ => return Scope::default(),
    };
    Scope { modrinth, curseforge: true, cf_loader, cf_loader_names }
}

/// An instance moving to `to_loader` (which may be the loader it has now)
pub fn instance(root: &Path, from_loader: &str, to_loader: &str) -> Target {
    let mut folders = Vec::new();
    if from_loader != "vanilla" || to_loader != "vanilla" {
        folders.push(Folder { kind: ContentKind::Mod, dir: "mods", scope: mod_scope(to_loader) });
    }
    folders.push(Folder {
        kind: ContentKind::ResourcePack,
        dir: "resourcepacks",
        scope: Scope { modrinth: vec!["minecraft"], curseforge: true, ..Default::default() },
    });
    folders.push(Folder {
        kind: ContentKind::Shader,
        dir: "shaderpacks",
        scope: Scope { modrinth: vec!["iris", "optifine", "canvas", "vanilla"], curseforge: true, ..Default::default() },
    });
    let mut worlds: Vec<String> = fs::read_dir(root.join("saves"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().join("level.dat").is_file())
                .map(|e| format!("saves/{}", e.file_name().to_string_lossy()))
                .collect()
        })
        .unwrap_or_default();
    worlds.sort();
    Target {
        root: root.to_path_buf(),
        folders,
        worlds,
        configs: vec!["config".into(), "options.txt".into()],
        core_files: Vec::new(),
        hangar: HashMap::new(),
    }
}

/// Cores that can replace each other without starting over: the plugin servers, and the
/// mod loaders (mods are looked up again for the new one)
pub fn can_switch(from: &ServerCoreType, to: &ServerCoreType) -> bool {
    use ServerCoreType::*;
    let family = |core: &ServerCoreType| match core {
        Paper | Purpur | Folia => 1,
        Fabric | Quilt | NeoForge | Forge => 2,
        Vanilla => 3,
        Pumpkin => 4,
        Bedrock => 5,
    };
    from == to || (family(from) == family(to) && family(from) <= 2)
}

/// A server moving to `to_core`
pub fn server(root: &Path, to_core: &ServerCoreType) -> Target {
    let mut folders = Vec::new();
    if let Some(platform) = crate::server::plugins::platform(to_core) {
        let (kind, scope) = if platform.folder == "plugins" {
            (ContentKind::Plugin, Scope { modrinth: platform.modrinth_loaders.to_vec(), ..Default::default() })
        } else {
            (ContentKind::Mod, mod_scope(&loader_id(to_core)))
        };
        folders.push(Folder { kind, dir: platform.folder, scope });
    }

    // The world and its dimensions (Paper keeps them in separate folders)
    let level = crate::server::map::level_name(root);
    let worlds: Vec<String> = [level.clone(), format!("{level}_nether"), format!("{level}_the_end")]
        .into_iter()
        .filter(|w| root.join(w).is_dir())
        .collect();

    // Servers rewrite their config files when they upgrade
    let mut configs: Vec<String> = fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|name| {
                    let lower = name.to_ascii_lowercase();
                    [".yml", ".yaml", ".toml", ".properties", ".json", ".json5"].iter().any(|ext| lower.ends_with(ext))
                })
                .collect()
        })
        .unwrap_or_default();
    configs.push("config".into());
    configs.push(".ingot/plugins.json".into());
    // Each plugin's own config folder (not the jars)
    if let Ok(entries) = fs::read_dir(root.join("plugins")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() && !name.starts_with('.') {
                configs.push(format!("plugins/{name}"));
            }
        }
    }
    configs.sort();

    // The jar and the installed loader are downloaded again for the new version
    let mut core_files: Vec<String> = [
        "server.jar",
        "quilt-server-launch.jar",
        "libraries/net/neoforged/neoforge",
        "libraries/net/minecraftforge/forge",
    ]
    .map(str::to_string)
    .into();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("forge-") && name.ends_with("-shim.jar") {
                core_files.push(name);
            }
        }
    }

    Target {
        root: root.to_path_buf(),
        folders,
        worlds,
        configs,
        core_files,
        hangar: crate::server::plugins::tracked_hangar(root),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_cores() {
        use ServerCoreType::*;
        assert!(can_switch(&Paper, &Purpur) && can_switch(&Purpur, &Folia));
        assert!(can_switch(&Fabric, &NeoForge) && can_switch(&Quilt, &Forge));
        assert!(!can_switch(&Paper, &Fabric) && !can_switch(&Vanilla, &Paper) && !can_switch(&Pumpkin, &Vanilla));
        assert!(can_switch(&Vanilla, &Vanilla));
    }

    #[test]
    fn loader_ids() {
        assert_eq!(loader_id(&ServerCoreType::NeoForge), "neoforge");
        assert_eq!(parse_loader::<ServerCoreType>("purpur").unwrap(), ServerCoreType::Purpur);
        assert!(parse_loader::<ServerCoreType>("bukkit").is_err());
    }

    #[test]
    fn server_backup_covers_configs_and_core() {
        let root = std::env::temp_dir().join(format!("ingot-vc-target-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["world/region", "world_nether", "plugins/LuckPerms", "plugins/.paper-remapped", "config"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in ["server.properties", "bukkit.yml", "server.jar", "eula.txt", "forge-1.20.1-47.4.10-shim.jar"] {
            fs::write(root.join(file), "x").unwrap();
        }
        let target = server(&root, &ServerCoreType::Paper);
        assert_eq!(target.worlds, ["world", "world_nether"]);
        for rel in ["bukkit.yml", "server.properties", "config", "plugins/LuckPerms", ".ingot/plugins.json"] {
            assert!(target.configs.contains(&rel.to_string()), "{rel}");
        }
        assert!(!target.configs.iter().any(|c| c.contains(".paper-remapped") || c == "eula.txt"));
        assert!(target.core_files.contains(&"server.jar".to_string()));
        assert!(target.core_files.contains(&"forge-1.20.1-47.4.10-shim.jar".to_string()));
        assert_eq!(target.folders[0].dir, "plugins");
        fs::remove_dir_all(&root).unwrap();
    }
}
