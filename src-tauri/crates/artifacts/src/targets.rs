use crate::sources::Scope;

/// Mods for this loader on Modrinth and CurseForge (Quilt also runs Fabric mods)
pub fn mod_scope(loader: &str) -> Scope {
    let (modrinth, cf_loader, cf_loader_names): (
        Vec<&'static str>,
        Option<u32>,
        Vec<&'static str>,
    ) = match loader {
        "fabric" => (vec!["fabric"], Some(4), vec!["Fabric"]),
        "quilt" => (vec!["quilt", "fabric"], None, vec!["Quilt", "Fabric"]),
        "neoforge" => (vec!["neoforge"], Some(6), vec!["NeoForge"]),
        "forge" => (vec!["forge"], Some(1), vec!["Forge"]),
        // Vanilla loads no mods: an empty scope turns them all off
        _ => return Scope::default(),
    };
    Scope {
        modrinth,
        curseforge: true,
        cf_loader,
        cf_loader_names,
    }
}
