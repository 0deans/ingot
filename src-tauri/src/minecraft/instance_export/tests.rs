use std::fs::{self, File};
use std::io::Read;
use tempfile::tempdir;
use zip::ZipArchive;

use super::archive::{write_archive, ArchivePlanItem, ArchiveSource};
use super::hashing::hash_file;
use super::models::ExportFormat;
use super::selection::{
    collect_preview_entries, is_default_included, is_permanently_excluded, validate_included_paths,
};
use crate::version_change::models::sha1_hex;
use crate::version_change::sources::cf_fingerprint;

#[test]
fn test_export_format_extension() {
    assert_eq!(ExportFormat::Modrinth.file_extension(), "mrpack");
    assert_eq!(ExportFormat::CurseForge.file_extension(), "zip");
    assert_eq!(ExportFormat::Prism.file_extension(), "zip");
}

#[test]
fn test_default_exclusions_and_inclusions() {
    let permanent = [
        ".ingot/content.json",
        ".ingot-trash/old.jar",
        "logs/latest.log",
        "crash-reports/crash-2026.txt",
        "natives/libsomething.dll",
        ".fabric/remap.jar",
        ".mixin.out/trace.log",
        ".cache/libraries.jar",
        "downloads/mod.jar",
        "backups/world.zip",
        ".staging_abc/test",
        ".qp_instance/data",
        "session.lock",
        "usercache.json",
        "config/temp.tmp",
        ".DS_Store",
        "Thumbs.db",
    ];

    for path in permanent {
        assert!(
            is_permanently_excluded(path),
            "expected {path} to be permanently excluded"
        );
        assert!(
            !is_default_included(path),
            "expected {path} to not be default included"
        );
    }

    let default_excluded = [
        "saves/world/level.dat",
        "screenshots/2026-11-08.png",
        "options.txt",
        "servers.dat",
    ];

    for path in default_excluded {
        assert!(
            !is_permanently_excluded(path),
            "expected {path} to not be permanently excluded"
        );
        assert!(
            !is_default_included(path),
            "expected {path} to be default excluded"
        );
    }

    let default_included = [
        "mods/fabric-api.jar",
        "config/mod.json",
        "resourcepacks/faithful.zip",
        "shaderpacks/bsl.zip",
        "defaultconfigs/ftb.snbt",
    ];

    for path in default_included {
        assert!(
            !is_permanently_excluded(path),
            "expected {path} to not be permanently excluded"
        );
        assert!(
            is_default_included(path),
            "expected {path} to be default included"
        );
    }
}

#[test]
fn test_validate_included_paths_rejections() {
    let dir = tempdir().unwrap();
    let instance_path = dir.path();

    // Create a valid file
    fs::create_dir_all(instance_path.join("mods")).unwrap();
    fs::write(instance_path.join("mods").join("valid.jar"), b"jar content").unwrap();

    let valid_paths = vec!["mods/valid.jar".to_string()];
    let res = validate_included_paths(instance_path, &valid_paths);
    assert!(res.is_ok());

    let invalid_cases = [
        "../escape.jar",
        "mods/../../escape.jar",
        "/etc/passwd",
        "C:\\Windows\\System32\\calc.exe",
        "session.lock",
        ".ingot/content.json",
        "logs/latest.log",
        "",
        "   ",
        "mods/nonexistent.jar",
    ];

    for invalid in invalid_cases {
        let paths = vec![invalid.to_string()];
        let res = validate_included_paths(instance_path, &paths);
        assert!(
            res.is_err(),
            "expected {invalid} to be rejected, got: {res:?}"
        );
    }
}

#[test]
fn test_collect_preview_entries_and_symlinks() {
    let dir = tempdir().unwrap();
    let instance_path = dir.path();

    // Create directories and files
    fs::create_dir_all(instance_path.join("mods")).unwrap();
    fs::create_dir_all(instance_path.join("logs")).unwrap();
    fs::create_dir_all(instance_path.join("saves").join("world")).unwrap();

    fs::write(instance_path.join("mods").join("mod-a.jar"), b"12345").unwrap();
    fs::write(instance_path.join("logs").join("latest.log"), b"log").unwrap();
    fs::write(
        instance_path.join("saves").join("world").join("level.dat"),
        b"data",
    )
    .unwrap();
    fs::write(instance_path.join("options.txt"), b"options").unwrap();

    // Try creating a symlink if supported
    #[cfg(windows)]
    let symlink_created = {
        let target = instance_path.join("mods").join("mod-a.jar");
        let link = instance_path.join("mods").join("symlink.jar");
        std::os::windows::fs::symlink_file(&target, &link).is_ok()
    };
    #[cfg(unix)]
    let symlink_created = {
        let target = instance_path.join("mods").join("mod-a.jar");
        let link = instance_path.join("mods").join("symlink.jar");
        std::os::unix::fs::symlink(&target, &link).is_ok()
    };

    let (entries, warnings) = collect_preview_entries(instance_path).unwrap();

    // logs/ should not be visited
    assert!(!entries.iter().any(|e| e.path.starts_with("logs")));

    let mod_entry = entries.iter().find(|e| e.path == "mods/mod-a.jar").unwrap();
    assert_eq!(mod_entry.size_bytes, 5);
    assert!(mod_entry.default_included);

    let opt_entry = entries.iter().find(|e| e.path == "options.txt").unwrap();
    assert!(!opt_entry.default_included);

    let world_entry = entries
        .iter()
        .find(|e| e.path == "saves/world/level.dat")
        .unwrap();
    assert!(!world_entry.default_included);

    if symlink_created {
        assert!(warnings
            .iter()
            .any(|w| w.contains("Skipped symlink or junction")));
        assert!(!entries.iter().any(|e| e.path == "mods/symlink.jar"));
    }
}

#[test]
fn test_hashing_sha1_sha512_and_cf_fingerprint() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    let content = b"a b\tc\r\nd";
    fs::write(&file_path, content).unwrap();

    let hashes = hash_file(&file_path, true).unwrap();
    assert_eq!(hashes.size_bytes, content.len() as u64);
    assert_eq!(hashes.sha1, sha1_hex(content));
    assert_eq!(hashes.cf_fingerprint, Some(cf_fingerprint(content)));
    assert!(!hashes.sha512.is_empty());

    // Without fingerprint
    let hashes_no_fp = hash_file(&file_path, false).unwrap();
    assert_eq!(hashes_no_fp.sha1, hashes.sha1);
    assert_eq!(hashes_no_fp.sha512, hashes.sha512);
    assert_eq!(hashes_no_fp.cf_fingerprint, None);
}

#[test]
fn test_atomic_archive_keeps_existing_on_failure() {
    let dir = tempdir().unwrap();
    let dest_path = dir.path().join("output.zip");

    // Write original file
    fs::write(&dest_path, b"ORIGINAL DATA").unwrap();

    // Injected failure: reference non-existent file
    let failing_items = vec![ArchivePlanItem {
        zip_path: "corrupt.txt".to_string(),
        source: ArchiveSource::File(dir.path().join("does_not_exist.bin")),
    }];

    let res = write_archive(&dest_path, failing_items);
    assert!(res.is_err());

    // Verify original file is intact
    let preserved = fs::read(&dest_path).unwrap();
    assert_eq!(preserved, b"ORIGINAL DATA");

    // Now write a valid archive
    let text_file = dir.path().join("input.txt");
    fs::write(&text_file, b"File content").unwrap();

    let valid_items = vec![
        ArchivePlanItem {
            zip_path: "manifest.json".to_string(),
            source: ArchiveSource::Bytes(b"{\"ok\":true}".to_vec()),
        },
        ArchivePlanItem {
            zip_path: "overrides/sub/input.txt".to_string(),
            source: ArchiveSource::File(text_file),
        },
    ];

    let success = write_archive(&dest_path, valid_items);
    assert!(success.is_ok());

    // Verify zip contents
    let file = File::open(&dest_path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();
    assert_eq!(zip.len(), 2);

    {
        let mut item1 = zip.by_name("manifest.json").unwrap();
        let mut item1_bytes = Vec::new();
        item1.read_to_end(&mut item1_bytes).unwrap();
        assert_eq!(item1_bytes, b"{\"ok\":true}");
    }

    {
        let mut item2 = zip.by_name("overrides/sub/input.txt").unwrap();
        let mut item2_bytes = Vec::new();
        item2.read_to_end(&mut item2_bytes).unwrap();
        assert_eq!(item2_bytes, b"File content");
    }
}

#[test]
fn test_prism_export_all_loaders_and_horizon_rejection() {
    use super::error::InstanceExportError;
    use super::prism::{build_prism_plan, generate_instance_cfg, generate_mmc_pack};
    use crate::minecraft::instance::{InstanceConfig, ModLoaderType};

    let base_config = InstanceConfig {
        id: "test-id".to_string(),
        name: "Test Instance".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Vanilla,
        loader_version: None,
        java_path: None,
        memory_min_mb: Some(1024),
        memory_max_mb: Some(4096),
        icon: Some("creeper".to_string()),
        created_at: 0,
        last_played: None,
        total_play_time_seconds: 0,
        jvm_args: Some(vec!["-XX:+UseG1GC".to_string(), "-Xss2M".to_string()]),
        fullscreen: None,
        window_width: None,
        window_height: None,
        sync_options: None,
        sync_servers: None,
        sync_resource_packs: None,
        sync_command_history: None,
        sync_creative_hotbars: None,
        last_synced_at: None,
        group: None,
        color_tag: None,
    };

    // 1. instance.cfg verification
    let cfg = generate_instance_cfg(&base_config, "My Pack");
    assert!(cfg.contains("InstanceType=OneSix"));
    assert!(cfg.contains("name=My Pack"));
    assert!(cfg.contains("iconKey=creeper"));
    assert!(cfg.contains("OverrideJavaArgs=true"));
    assert!(cfg.contains("JvmArgs=-XX:+UseG1GC -Xss2M"));
    assert!(cfg.contains("OverrideMemory=true"));
    assert!(cfg.contains("MinMemAlloc=1024"));
    assert!(cfg.contains("MaxMemAlloc=4096"));

    // 2. Vanilla mmc-pack.json
    let vanilla_pack = generate_mmc_pack(&base_config).unwrap();
    assert_eq!(vanilla_pack.components.len(), 1);
    assert_eq!(vanilla_pack.components[0].uid, "net.minecraft");
    assert_eq!(vanilla_pack.components[0].version, "1.20.1");

    // 3. Fabric
    let mut fabric_config = base_config.clone();
    fabric_config.loader = ModLoaderType::Fabric;
    fabric_config.loader_version = Some("0.15.11".to_string());
    let fabric_pack = generate_mmc_pack(&fabric_config).unwrap();
    assert_eq!(fabric_pack.components.len(), 3);
    assert_eq!(fabric_pack.components[0].uid, "net.minecraft");
    assert_eq!(fabric_pack.components[1].uid, "net.fabricmc.intermediary");
    assert_eq!(fabric_pack.components[2].uid, "net.fabricmc.fabric-loader");
    assert_eq!(fabric_pack.components[2].version, "0.15.11");

    // 4. Quilt
    let mut quilt_config = base_config.clone();
    quilt_config.loader = ModLoaderType::Quilt;
    quilt_config.loader_version = Some("0.25.0".to_string());
    let quilt_pack = generate_mmc_pack(&quilt_config).unwrap();
    assert_eq!(quilt_pack.components.len(), 3);
    assert_eq!(quilt_pack.components[0].uid, "net.minecraft");
    assert_eq!(quilt_pack.components[1].uid, "org.quiltmc.hashed");
    assert_eq!(quilt_pack.components[2].uid, "org.quiltmc.quilt-loader");
    assert_eq!(quilt_pack.components[2].version, "0.25.0");

    // 5. Forge
    let mut forge_config = base_config.clone();
    forge_config.loader = ModLoaderType::Forge;
    forge_config.loader_version = Some("47.2.0".to_string());
    let forge_pack = generate_mmc_pack(&forge_config).unwrap();
    assert_eq!(forge_pack.components.len(), 2);
    assert_eq!(forge_pack.components[0].uid, "net.minecraft");
    assert_eq!(forge_pack.components[1].uid, "net.minecraftforge");
    assert_eq!(forge_pack.components[1].version, "47.2.0");

    // 6. NeoForge
    let mut neoforge_config = base_config.clone();
    neoforge_config.loader = ModLoaderType::NeoForge;
    neoforge_config.loader_version = Some("20.4.80".to_string());
    let neoforge_pack = generate_mmc_pack(&neoforge_config).unwrap();
    assert_eq!(neoforge_pack.components.len(), 2);
    assert_eq!(neoforge_pack.components[0].uid, "net.minecraft");
    assert_eq!(neoforge_pack.components[1].uid, "net.neoforged");
    assert_eq!(neoforge_pack.components[1].version, "20.4.80");

    // 7. Horizon must be rejected
    let mut horizon_config = base_config.clone();
    horizon_config.loader = ModLoaderType::Horizon;
    let horizon_err = generate_mmc_pack(&horizon_config).unwrap_err();
    assert!(matches!(
        horizon_err,
        InstanceExportError::UnsupportedLoader(ModLoaderType::Horizon)
    ));

    // 8. build_prism_plan puts files under .minecraft/ and checks empty name
    let empty_name_err = build_prism_plan(&fabric_config, "  ", &[]).unwrap_err();
    assert!(matches!(empty_name_err, InstanceExportError::EmptyName));

    let dummy_file = std::path::PathBuf::from("/fake/path/mod.jar");
    let files = vec![
        ("mods/example.jar".to_string(), dummy_file.clone()),
        ("config/test.json".to_string(), dummy_file),
    ];
    let plan = build_prism_plan(&fabric_config, "My Pack", &files).unwrap();
    assert_eq!(plan.len(), 4); // instance.cfg + mmc-pack.json + 2 files
    assert_eq!(plan[0].zip_path, "instance.cfg");
    assert_eq!(plan[1].zip_path, "mmc-pack.json");
    assert_eq!(plan[2].zip_path, ".minecraft/mods/example.jar");
    assert_eq!(plan[3].zip_path, ".minecraft/config/test.json");
}

use super::resolve::{ContentResolver, CurseForgeMatch};
use crate::version_change::sources::{ModrinthVersionFileMatch, Source, SourceError};

#[derive(Default)]
struct FakeContentResolver {
    modrinth_matches: std::collections::HashMap<String, ModrinthVersionFileMatch>,
    modrinth_error: bool,
    cf_matches: std::collections::HashMap<u32, CurseForgeMatch>,
    cf_error: bool,
}

impl ContentResolver for FakeContentResolver {
    async fn resolve_modrinth(
        &self,
        hashes: &[String],
    ) -> Result<
        std::collections::HashMap<String, ModrinthVersionFileMatch>,
        super::error::InstanceExportError,
    > {
        if self.modrinth_error {
            return Err(super::error::InstanceExportError::Provider(
                SourceError::InvalidPayload {
                    provider: Source::Modrinth,
                    field: "network error",
                },
            ));
        }
        let mut results = std::collections::HashMap::new();
        for h in hashes {
            if let Some(m) = self.modrinth_matches.get(h) {
                results.insert(h.clone(), m.clone());
            }
        }
        Ok(results)
    }

    async fn resolve_curseforge(
        &self,
        fingerprints: &[u32],
    ) -> Result<std::collections::HashMap<u32, CurseForgeMatch>, super::error::InstanceExportError>
    {
        if self.cf_error {
            return Err(super::error::InstanceExportError::Provider(
                SourceError::InvalidPayload {
                    provider: Source::Curseforge,
                    field: "network error",
                },
            ));
        }
        let mut results = std::collections::HashMap::new();
        for fp in fingerprints {
            if let Some(m) = self.cf_matches.get(fp) {
                results.insert(*fp, m.clone());
            }
        }
        Ok(results)
    }
}

#[tokio::test]
async fn test_mrpack_resolved_unknown_disallowed_and_disabled_files() {
    use super::models::{EnvSupport, ExportFormat, ExportOptions, FileSide};
    use super::mrpack::build_mrpack_plan;
    use super::resolve::classify_modrinth_files;
    use crate::minecraft::instance::{InstanceConfig, ModLoaderType};

    let dir = tempdir().unwrap();
    let instance_dir = dir.path();

    fs::create_dir_all(instance_dir.join("mods")).unwrap();
    fs::create_dir_all(instance_dir.join("config")).unwrap();

    let resolved_path = instance_dir.join("mods").join("resolved.jar");
    fs::write(&resolved_path, b"resolved-mod-content").unwrap();
    let resolved_hash = sha1_hex(b"resolved-mod-content");

    let unknown_path = instance_dir.join("mods").join("unknown.jar");
    fs::write(&unknown_path, b"unknown-mod-content").unwrap();

    let badhost_path = instance_dir.join("mods").join("badhost.jar");
    fs::write(&badhost_path, b"badhost-mod-content").unwrap();
    let badhost_hash = sha1_hex(b"badhost-mod-content");

    let client_only_path = instance_dir.join("mods").join("client_only.jar");
    fs::write(&client_only_path, b"client-only-mod-content").unwrap();

    let server_only_path = instance_dir.join("mods").join("server_only.jar");
    fs::write(&server_only_path, b"server-only-mod-content").unwrap();

    let disabled_path = instance_dir.join("mods").join("disabled.jar.disabled");
    fs::write(&disabled_path, b"disabled-content").unwrap();

    let config_path = instance_dir.join("config").join("settings.toml");
    fs::write(&config_path, b"some-settings = true").unwrap();

    let mut fake_resolver = FakeContentResolver::default();
    fake_resolver.modrinth_matches.insert(
        resolved_hash,
        ModrinthVersionFileMatch {
            url: "https://cdn.modrinth.com/resolved.jar".to_string(),
            sha512: Some("sha512_resolved".to_string()),
            size: 20,
        },
    );
    fake_resolver.modrinth_matches.insert(
        badhost_hash,
        ModrinthVersionFileMatch {
            url: "https://evil.example.com/badhost.jar".to_string(),
            sha512: None,
            size: 19,
        },
    );

    let options = ExportOptions {
        format: ExportFormat::Modrinth,
        name: "Test Mrpack".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Author".to_string()),
        summary: Some("Summary".to_string()),
        included_paths: vec![],
        file_sides: vec![
            FileSide {
                path: "mods/client_only.jar".to_string(),
                client: EnvSupport::Required,
                server: EnvSupport::Unsupported,
            },
            FileSide {
                path: "mods/server_only.jar".to_string(),
                client: EnvSupport::Unsupported,
                server: EnvSupport::Required,
            },
        ],
    };

    let included_files = vec![
        ("mods/resolved.jar".to_string(), resolved_path),
        ("mods/unknown.jar".to_string(), unknown_path),
        ("mods/badhost.jar".to_string(), badhost_path),
        ("mods/client_only.jar".to_string(), client_only_path),
        ("mods/server_only.jar".to_string(), server_only_path),
        ("mods/disabled.jar.disabled".to_string(), disabled_path),
        ("config/settings.toml".to_string(), config_path),
    ];

    let classified = classify_modrinth_files(&fake_resolver, &options, &included_files)
        .await
        .unwrap();

    let config = InstanceConfig {
        id: "inst-1".to_string(),
        name: "Inst 1".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Fabric,
        loader_version: Some("0.15.11".to_string()),
        java_path: None,
        memory_min_mb: None,
        memory_max_mb: None,
        icon: None,
        created_at: 0,
        last_played: None,
        total_play_time_seconds: 0,
        jvm_args: None,
        fullscreen: None,
        window_width: None,
        window_height: None,
        sync_options: None,
        sync_servers: None,
        sync_resource_packs: None,
        sync_command_history: None,
        sync_creative_hotbars: None,
        last_synced_at: None,
        group: None,
        color_tag: None,
    };

    let res = build_mrpack_plan(&config, &options, classified).unwrap();

    assert_eq!(res.resolved_count, 1);
    assert_eq!(res.embedded_count, 6);
    // embedded warnings should contain candidate mods that failed to resolve, but not disabled or config
    assert_eq!(res.embedded_warnings.len(), 4);
    assert!(res
        .embedded_warnings
        .iter()
        .any(|w| w.contains("unknown.jar")));
    assert!(res
        .embedded_warnings
        .iter()
        .any(|w| w.contains("badhost.jar")));
    assert!(res
        .embedded_warnings
        .iter()
        .any(|w| w.contains("client_only.jar")));
    assert!(res
        .embedded_warnings
        .iter()
        .any(|w| w.contains("server_only.jar")));
    assert!(!res
        .embedded_warnings
        .iter()
        .any(|w| w.contains("disabled.jar.disabled")));
    assert!(!res
        .embedded_warnings
        .iter()
        .any(|w| w.contains("settings.toml")));

    // Check archive plan items
    let zip_paths: Vec<&str> = res.plan.iter().map(|item| item.zip_path.as_str()).collect();
    assert!(zip_paths.contains(&"modrinth.index.json"));
    assert!(zip_paths.contains(&"overrides/mods/unknown.jar"));
    assert!(zip_paths.contains(&"overrides/mods/badhost.jar"));
    assert!(zip_paths.contains(&"client-overrides/mods/client_only.jar"));
    assert!(zip_paths.contains(&"server-overrides/mods/server_only.jar"));
    assert!(zip_paths.contains(&"overrides/mods/disabled.jar.disabled"));
    assert!(zip_paths.contains(&"overrides/config/settings.toml"));
    // Resolved mod should NOT be in zip payload!
    assert!(!zip_paths.contains(&"overrides/mods/resolved.jar"));
}

#[tokio::test]
async fn test_mrpack_provider_error_propagates() {
    use super::models::{ExportFormat, ExportOptions};
    use super::resolve::classify_modrinth_files;

    let dir = tempdir().unwrap();
    let mod_path = dir.path().join("test.jar");
    fs::write(&mod_path, b"test").unwrap();

    let fake_resolver = FakeContentResolver {
        modrinth_error: true,
        ..Default::default()
    };

    let options = ExportOptions {
        format: ExportFormat::Modrinth,
        name: "Test".to_string(),
        version: "1.0".to_string(),
        author: None,
        summary: None,
        included_paths: vec![],
        file_sides: vec![],
    };

    let files = vec![("mods/test.jar".to_string(), mod_path)];
    let res = classify_modrinth_files(&fake_resolver, &options, &files).await;
    assert!(matches!(
        res,
        Err(super::error::InstanceExportError::Provider(_))
    ));
}

#[test]
fn test_mrpack_round_trip_with_modrinth_index_parser() {
    use super::models::{EnvSupport, ExportFormat, ExportOptions};
    use super::mrpack::build_modrinth_index;
    use super::resolve::ResolvedModrinthFile;
    use crate::minecraft::content::install::{parse_modrinth_loader, ModrinthIndex};
    use crate::minecraft::instance::{InstanceConfig, ModLoaderType};

    let config = InstanceConfig {
        id: "inst-1".to_string(),
        name: "My Pack".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Fabric,
        loader_version: Some("0.15.11".to_string()),
        java_path: None,
        memory_min_mb: None,
        memory_max_mb: None,
        icon: None,
        created_at: 0,
        last_played: None,
        total_play_time_seconds: 0,
        jvm_args: None,
        fullscreen: None,
        window_width: None,
        window_height: None,
        sync_options: None,
        sync_servers: None,
        sync_resource_packs: None,
        sync_command_history: None,
        sync_creative_hotbars: None,
        last_synced_at: None,
        group: None,
        color_tag: None,
    };

    let options = ExportOptions {
        format: ExportFormat::Modrinth,
        name: "My Pack".to_string(),
        version: "1.2.3".to_string(),
        author: Some("Author".to_string()),
        summary: Some("Summary".to_string()),
        included_paths: vec![],
        file_sides: vec![],
    };

    let resolved = vec![ResolvedModrinthFile {
        path: "mods/fabric-api.jar".to_string(),
        sha1: "abc1234".to_string(),
        sha512: "def5678".to_string(),
        size_bytes: 12345,
        download_url: "https://cdn.modrinth.com/fabric-api.jar".to_string(),
        client_env: EnvSupport::Required,
        server_env: EnvSupport::Required,
    }];

    let index = build_modrinth_index(&config, &options, &resolved).unwrap();
    let index_json = serde_json::to_vec_pretty(&index).unwrap();

    // Round-trip into installer parser
    let parsed: ModrinthIndex = serde_json::from_slice(&index_json).unwrap();
    assert_eq!(parsed.name.as_deref(), Some("My Pack"));
    assert_eq!(parsed.game.as_deref(), Some("minecraft"));

    let deps = parsed.dependencies.as_ref().unwrap();
    let (loader, loader_ver) = parse_modrinth_loader(deps);
    assert_eq!(loader, ModLoaderType::Fabric);
    assert_eq!(loader_ver, Some("0.15.11".to_string()));
}

#[tokio::test]
async fn test_curseforge_export_tracking_fingerprint_modlist_and_roundtrip() {
    use super::curseforge::{
        build_curseforge_modloaders, build_curseforge_plan, classify_curseforge_files, html_escape,
    };
    use super::models::{ExportFormat, ExportOptions};
    use crate::minecraft::content::install::{parse_curseforge_loader, CurseForgeManifest};
    use crate::minecraft::instance::{InstanceConfig, ModLoaderType};
    use crate::minecraft::instance_content::models::{ProviderSource, TrackedOrigin};

    let dir = tempdir().unwrap();
    let instance_dir = dir.path();
    fs::create_dir_all(instance_dir.join("mods")).unwrap();
    fs::create_dir_all(instance_dir.join("config")).unwrap();

    // 1. Tracked file (should resolve directly from tracking, no resolver call)
    let tracked_file = instance_dir.join("mods").join("tracked.jar");
    fs::write(&tracked_file, b"tracked-jar-bytes").unwrap();

    let mut tracking = std::collections::HashMap::new();
    tracking.insert(
        "tracked.jar".to_string(),
        TrackedOrigin {
            source: ProviderSource::CurseForge,
            project_id: "12345".to_string(),
            project_slug: Some("tracked".to_string()),
            version_id: "67890".to_string(),
            version_number: "1.0.0".to_string(),
            title: "Tracked Mod <Safe & Cool>".to_string(),
            icon_url: None,
        },
    );

    // 2. Untracked file (should fall back to fingerprint lookup)
    let untracked_file = instance_dir.join("mods").join("untracked.jar");
    let untracked_bytes = b"untracked-jar-bytes";
    fs::write(&untracked_file, untracked_bytes).unwrap();
    let fp = cf_fingerprint(untracked_bytes);

    // 3. Unknown file (fails fingerprint lookup -> embedded in overrides)
    let unknown_file = instance_dir.join("mods").join("unknown.jar");
    fs::write(&unknown_file, b"unknown-jar-bytes").unwrap();

    // 4. Config file (non-candidate -> embedded in overrides without warning)
    let config_file = instance_dir.join("config").join("options.txt");
    fs::write(&config_file, b"setting=true").unwrap();

    let mut fake_resolver = FakeContentResolver::default();
    fake_resolver.cf_matches.insert(
        fp,
        CurseForgeMatch {
            project_id: 99999,
            file_id: 88888,
        },
    );

    let included_files = vec![
        ("mods/tracked.jar".to_string(), tracked_file),
        ("mods/untracked.jar".to_string(), untracked_file),
        ("mods/unknown.jar".to_string(), unknown_file),
        ("config/options.txt".to_string(), config_file),
    ];

    let classification = classify_curseforge_files(&fake_resolver, &tracking, &included_files)
        .await
        .unwrap();

    assert_eq!(classification.resolved.len(), 2);
    assert_eq!(classification.embedded.len(), 2);

    let config = InstanceConfig {
        id: "inst-1".to_string(),
        name: "CF Pack".to_string(),
        game_version: "1.20.1".to_string(),
        loader: ModLoaderType::Fabric,
        loader_version: Some("0.15.11".to_string()),
        java_path: None,
        memory_min_mb: None,
        memory_max_mb: None,
        icon: None,
        created_at: 0,
        last_played: None,
        total_play_time_seconds: 0,
        jvm_args: None,
        fullscreen: None,
        window_width: None,
        window_height: None,
        sync_options: None,
        sync_servers: None,
        sync_resource_packs: None,
        sync_command_history: None,
        sync_creative_hotbars: None,
        last_synced_at: None,
        group: None,
        color_tag: None,
    };

    let options = ExportOptions {
        format: ExportFormat::CurseForge,
        name: "CF Pack".to_string(),
        version: "1.0.0".to_string(),
        author: Some("Tester & Author <dev>".to_string()),
        summary: Some("Summary".to_string()),
        included_paths: vec![],
        file_sides: vec![],
    };

    let res = build_curseforge_plan(&config, &options, classification).unwrap();
    assert_eq!(res.resolved_count, 2);
    assert_eq!(res.embedded_count, 2);
    assert_eq!(res.embedded_warnings.len(), 1);
    assert!(res.embedded_warnings[0].contains("unknown.jar"));

    // Check archive plan items
    let zip_paths: Vec<&str> = res.plan.iter().map(|i| i.zip_path.as_str()).collect();
    assert!(zip_paths.contains(&"manifest.json"));
    assert!(zip_paths.contains(&"modlist.html"));
    assert!(zip_paths.contains(&"overrides/mods/unknown.jar"));
    assert!(zip_paths.contains(&"overrides/config/options.txt"));

    // Check modlist.html content and escaping
    let html_item = res
        .plan
        .iter()
        .find(|i| i.zip_path == "modlist.html")
        .unwrap();
    if let super::archive::ArchiveSource::Bytes(b) = &html_item.source {
        let html_str = String::from_utf8(b.clone()).unwrap();
        assert!(html_str.contains("https://www.curseforge.com/projects/12345"));
        assert!(html_str.contains("https://www.curseforge.com/projects/99999"));
        assert!(html_str.contains("mods/unknown.jar"));
    } else {
        panic!("expected Bytes source for modlist.html");
    }

    // Check HTML escaping helper
    assert_eq!(
        html_escape("A & B < C > \"D\" 'E'"),
        "A &amp; B &lt; C &gt; &quot;D&quot; &#39;E&#39;"
    );

    // Round-trip manifest.json into installer parser
    let manifest_item = res
        .plan
        .iter()
        .find(|i| i.zip_path == "manifest.json")
        .unwrap();
    if let super::archive::ArchiveSource::Bytes(b) = &manifest_item.source {
        let parsed: CurseForgeManifest = serde_json::from_slice(b).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("CF Pack"));
        assert_eq!(parsed.minecraft.version, "1.20.1");

        let files = parsed.files.as_ref().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .any(|f| f.project_id == 12345 && f.file_id == 67890));
        assert!(files
            .iter()
            .any(|f| f.project_id == 99999 && f.file_id == 88888));

        let (loader, loader_ver) =
            parse_curseforge_loader(parsed.minecraft.mod_loaders.as_ref().unwrap());
        assert_eq!(loader, ModLoaderType::Fabric);
        assert_eq!(loader_ver, Some("0.15.11".to_string()));
    } else {
        panic!("expected Bytes source for manifest.json");
    }

    // Test loader mapping for all loaders on CurseForge
    let mut quilt_config = config.clone();
    quilt_config.loader = ModLoaderType::Quilt;
    quilt_config.loader_version = Some("0.25.0".to_string());
    let quilt_loaders = build_curseforge_modloaders(&quilt_config).unwrap();
    assert_eq!(quilt_loaders[0].id, "quilt-0.25.0");

    let mut forge_config = config.clone();
    forge_config.loader = ModLoaderType::Forge;
    forge_config.loader_version = Some("47.2.0".to_string());
    let forge_loaders = build_curseforge_modloaders(&forge_config).unwrap();
    assert_eq!(forge_loaders[0].id, "forge-47.2.0");

    let mut neoforge_config = config.clone();
    neoforge_config.loader = ModLoaderType::NeoForge;
    neoforge_config.loader_version = Some("20.4.80".to_string());
    let neoforge_loaders = build_curseforge_modloaders(&neoforge_config).unwrap();
    assert_eq!(neoforge_loaders[0].id, "neoforge-20.4.80");

    let mut vanilla_config = config.clone();
    vanilla_config.loader = ModLoaderType::Vanilla;
    let vanilla_loaders = build_curseforge_modloaders(&vanilla_config).unwrap();
    assert!(vanilla_loaders.is_empty());

    let mut horizon_config = config;
    horizon_config.loader = ModLoaderType::Horizon;
    assert!(build_curseforge_modloaders(&horizon_config).is_err());
}
