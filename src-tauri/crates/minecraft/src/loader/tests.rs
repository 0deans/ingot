use super::*;

#[test]
fn test_normalize_loader_version() {
    assert_eq!(
        normalize_loader_version("1.7.10", "10.13.4.1614-1.7.10"),
        "10.13.4.1614"
    );
    assert_eq!(
        normalize_loader_version("1.16.5", "forge-36.2.39"),
        "36.2.39"
    );
    assert_eq!(
        normalize_loader_version("1.16.5", "1.16.5-36.2.39"),
        "36.2.39"
    );
    assert_eq!(
        normalize_loader_version("1.20.1", "neoforge-47.1.106"),
        "47.1.106"
    );
    assert_eq!(normalize_loader_version("1.20.1", "47.1.106"), "47.1.106");
    assert_eq!(
        normalize_loader_version("1.12.2", "14.23.5.2860-1.12.2"),
        "14.23.5.2860"
    );
}

#[test]
fn test_prism_library_deserialization_with_downloads_and_rules() {
    let json = r#"{
        "name": "net.minecraftforge:forge:1.16.5-36.2.39:launcher",
        "downloads": {
            "artifact": {
                "path": "net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar",
                "url": "https://files.prismlauncher.org/maven/net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar",
                "sha1": "abcdef1234567890",
                "size": 12345
            }
        },
        "rules": [
            {
                "action": "allow",
                "os": { "name": "windows" }
            }
        ]
    }"#;

    let prism_lib: PrismLibrary = serde_json::from_str(json).unwrap();
    assert_eq!(
        prism_lib.name,
        "net.minecraftforge:forge:1.16.5-36.2.39:launcher"
    );
    assert!(prism_lib.downloads.is_some());
    assert!(prism_lib.rules.is_some());

    let converted = convert_prism_library(prism_lib, "https://maven.minecraftforge.net/");
    assert_eq!(
        converted.name,
        "net.minecraftforge:forge:1.16.5-36.2.39:launcher"
    );
    let dl = converted.downloads.unwrap();
    let art = dl.artifact.unwrap();
    assert_eq!(
        art.path.as_deref(),
        Some("net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar")
    );
    assert_eq!(
        art.url,
        "https://files.prismlauncher.org/maven/net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar"
    );
    assert_eq!(art.sha1.as_deref(), Some("abcdef1234567890"));
    assert_eq!(art.size, Some(12345));
    assert!(converted.rules.is_some());
}

#[test]
fn test_prism_library_conversion_fills_missing_path() {
    let json = r#"{
        "name": "net.minecraftforge:forge:1.16.5-36.2.39:launcher",
        "downloads": {
            "artifact": {
                "url": "https://files.prismlauncher.org/maven/test.jar"
            }
        }
    }"#;

    let prism_lib: PrismLibrary = serde_json::from_str(json).unwrap();
    let converted = convert_prism_library(prism_lib, "https://maven.minecraftforge.net/");
    let dl = converted.downloads.unwrap();
    let art = dl.artifact.unwrap();
    assert_eq!(
        art.path.as_deref(),
        Some("net/minecraftforge/forge/1.16.5-36.2.39/forge-1.16.5-36.2.39-launcher.jar")
    );
    assert_eq!(art.url, "https://files.prismlauncher.org/maven/test.jar");
}

#[test]
fn test_prism_library_conversion_fallback_without_downloads() {
    let prism_lib = PrismLibrary {
        name: "cpw.mods:modlauncher:8.0.9".to_string(),
        url: Some("https://maven.minecraftforge.net/".to_string()),
        downloads: None,
        rules: None,
    };

    let converted = convert_prism_library(prism_lib, "https://maven.minecraftforge.net/");
    let dl = converted.downloads.unwrap();
    let art = dl.artifact.unwrap();
    assert_eq!(
        art.path.as_deref(),
        Some("cpw/mods/modlauncher/8.0.9/modlauncher-8.0.9.jar")
    );
    assert_eq!(
        art.url,
        "https://maven.minecraftforge.net/cpw/mods/modlauncher/8.0.9/modlauncher-8.0.9.jar"
    );
}

#[test]
fn test_prism_component_deserialization_with_tweakers_and_minecraft_arguments() {
    let json = r#"{
        "mainClass": "net.minecraft.launchwrapper.Launch",
        "+tweakers": ["net.minecraftforge.fml.common.launcher.FMLTweaker"],
        "minecraftArguments": "--username ${auth_player_name} --version ${version_name}"
    }"#;

    let comp: PrismComponent = serde_json::from_str(json).unwrap();
    assert_eq!(
        comp.main_class.as_deref(),
        Some("net.minecraft.launchwrapper.Launch")
    );
    assert_eq!(
        comp.tweakers.as_deref(),
        Some(&["net.minecraftforge.fml.common.launcher.FMLTweaker".to_string()][..])
    );
    assert_eq!(
        comp.minecraft_arguments.as_deref(),
        Some("--username ${auth_player_name} --version ${version_name}")
    );
}

#[test]
fn test_forge_log4j_library_helper() {
    let lib = forge_log4j_library(
        "org.apache.logging.log4j:log4j-core:2.15.0",
        "ba55c13d7ac2fd44df9cc8074455719a33f375b9",
        1789769,
    );
    assert_eq!(lib.name, "org.apache.logging.log4j:log4j-core:2.15.0");
    let dl = lib.downloads.unwrap();
    let art = dl.artifact.unwrap();
    assert_eq!(
        art.path.as_deref(),
        Some("org/apache/logging/log4j/log4j-core/2.15.0/log4j-core-2.15.0.jar")
    );
    assert_eq!(
        art.url,
        "https://libraries.minecraft.net/org/apache/logging/log4j/log4j-core/2.15.0/log4j-core-2.15.0.jar"
    );
    assert_eq!(
        art.sha1.as_deref(),
        Some("ba55c13d7ac2fd44df9cc8074455719a33f375b9")
    );
    assert_eq!(art.size, Some(1789769));
}

#[tokio::test]
async fn test_resolve_loader_profile_reads_v5_cache() {
    let temp_dir = tempfile::tempdir().unwrap();
    let profiles_dir = temp_dir.path().join("loader_profiles");
    fs::create_dir_all(&profiles_dir).unwrap();

    let cached_profile = LoaderProfile {
        id: "forge-1.16.5-36.2.39".into(),
        main_class: "io.github.zekerzhayard.forgewrapper.installer.Main".into(),
        libraries: vec![forge_log4j_library(
            "org.apache.logging.log4j:log4j-core:2.15.0",
            "ba55c13d7ac2fd44df9cc8074455719a33f375b9",
            1789769,
        )],
        maven_files: vec![LibraryEntry {
            name: "com.google.guava:guava:20.0".into(),
            downloads: None,
            rules: None,
            natives: None,
            url: None,
        }],
        jvm_args: Vec::new(),
        game_args: Vec::new(),
        minecraft_arguments: None,
    };
    fs::write(
        profiles_dir.join("v5_Forge_1.16.5_36.2.39.json"),
        serde_json::to_string(&cached_profile).unwrap(),
    )
    .unwrap();

    let client = reqwest::Client::new();
    let loaded = resolve_loader_profile(
        &client,
        temp_dir.path(),
        &ModLoaderType::Forge,
        "1.16.5",
        "36.2.39",
    )
    .await
    .unwrap();

    assert_eq!(loaded.id, "forge-1.16.5-36.2.39");
    assert_eq!(loaded.libraries.len(), 1);
    assert_eq!(
        loaded.libraries[0].name,
        "org.apache.logging.log4j:log4j-core:2.15.0"
    );
    assert_eq!(loaded.maven_files.len(), 1);
    assert_eq!(loaded.maven_files[0].name, "com.google.guava:guava:20.0");
}

#[tokio::test]
#[ignore = "requires public network to fetch Prism meta"]
async fn test_resolve_loader_profile_forge_1_16_5_live() {
    let temp_dir = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();
    let profile = resolve_loader_profile(
        &client,
        temp_dir.path(),
        &ModLoaderType::Forge,
        "1.16.5",
        "36.2.39",
    )
    .await
    .unwrap();

    assert_eq!(
        profile.main_class,
        "io.github.zekerzhayard.forgewrapper.installer.Main"
    );
    assert!(!profile
        .libraries
        .iter()
        .any(|l| l.name.starts_with("com.google.guava:guava:20.0")));
    assert!(profile
        .maven_files
        .iter()
        .any(|l| l.name.starts_with("com.google.guava:guava:20.0")));
    assert!(profile
        .libraries
        .iter()
        .any(|l| l.name == "org.apache.logging.log4j:log4j-core:2.15.0"));
    assert!(profile
        .libraries
        .iter()
        .any(|l| l.name == "org.apache.logging.log4j:log4j-api:2.15.0"));
    assert!(profile
        .libraries
        .iter()
        .any(|l| l.name == "org.apache.logging.log4j:log4j-slf4j18-impl:2.15.0"));
}

#[tokio::test]
#[ignore = "requires public network to download Log4j jars"]
async fn test_download_log4j_libraries_live() {
    use ingot_artifacts::downloader::{download_files_concurrent, DownloadTask};

    let temp_dir = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();
    let libs = vec![
        forge_log4j_library(
            "org.apache.logging.log4j:log4j-api:2.15.0",
            "4a5aa7e55a29391c6f66e0b259d5189aa11e45d0",
            301804,
        ),
        forge_log4j_library(
            "org.apache.logging.log4j:log4j-core:2.15.0",
            "ba55c13d7ac2fd44df9cc8074455719a33f375b9",
            1789769,
        ),
        forge_log4j_library(
            "org.apache.logging.log4j:log4j-slf4j18-impl:2.15.0",
            "88f72ad364bfc3a7cf43186fc17212f2b4bb8d97",
            21223,
        ),
    ];

    let mut tasks = Vec::new();
    for lib in libs {
        let dl = lib.downloads.unwrap();
        let art = dl.artifact.unwrap();
        let dest = temp_dir.path().join(art.path.unwrap());
        tasks.push(DownloadTask {
            url: art.url,
            dest,
            sha1: art.sha1,
            size: art.size,
        });
    }

    download_files_concurrent(&client, tasks, 3, None)
        .await
        .expect("Log4j libraries download failed");
}

#[tokio::test]
async fn test_fetch_loader_versions_with_endpoints_handles_success_non_success_and_malformed_json()
{
    use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};

    let server = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/fabric/v2/versions/loader/1.21.4",
            (
                200,
                r#"[{"loader":{"version":"0.16.9"}},{"loader":{"version":"0.16.8"}}]"#,
            ),
        ),
        ScriptedExchange::expect_path(
            "/prism/v1/net.neoforged/index.json",
            ResponseSpec::status(404),
        ),
        ScriptedExchange::expect_path(
            "/quilt/v3/versions/loader/1.21.4",
            ResponseSpec::json(200, b"{invalid-json".to_vec()),
        ),
    ])
    .await;

    let endpoints = LoaderEndpoints::from_base(server.base_url());
    let client = reqwest::Client::new();

    let fabric =
        fetch_loader_versions_with_endpoints(&client, &endpoints, &ModLoaderType::Fabric, "1.21.4")
            .await
            .unwrap();
    assert_eq!(fabric, vec!["0.16.9", "0.16.8"]);

    let neoforge_missing = fetch_loader_versions_with_endpoints(
        &client,
        &endpoints,
        &ModLoaderType::NeoForge,
        "1.21.4",
    )
    .await
    .unwrap();
    assert!(neoforge_missing.is_empty());

    let quilt_malformed =
        fetch_loader_versions_with_endpoints(&client, &endpoints, &ModLoaderType::Quilt, "1.21.4")
            .await
            .unwrap_err();
    assert!(matches!(quilt_malformed, LoaderError::Json(_)));

    let requests = server.finish().await;
    assert_eq!(requests.len(), 3);
}

#[tokio::test]
async fn test_resolve_loader_profile_with_endpoints_caches_recovers_corrupt_cache_and_errors_on_500(
) {
    use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};

    let temp_dir = tempfile::tempdir().unwrap();
    let cache_dir = temp_dir.path();
    let profiles_dir = cache_dir.join("loader_profiles");
    fs::create_dir_all(&profiles_dir).unwrap();

    // Seed corrupt cache entry so resolve refreshes from loopback server and overwrites it.
    let cache_path = profiles_dir.join("v5_Fabric_1.21.4_0.16.9.json");
    fs::write(&cache_path, b"corrupt-cache").unwrap();

    let server = LoopbackServer::exact_script(vec![
        ScriptedExchange::expect_path(
            "/fabric/v2/versions/loader/1.21.4/0.16.9/profile/json",
            (
                200,
                r#"{"mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"org.ow2.asm:asm:9.7"}]}"#,
            ),
        ),
        ScriptedExchange::expect_path(
            "/prism/v1/net.minecraftforge/47.3.0.json",
            ResponseSpec::text(500, "Upstream error"),
        ),
    ])
    .await;

    let endpoints = LoaderEndpoints::from_base(server.base_url());
    let client = reqwest::Client::new();

    let resolved = resolve_loader_profile_with_endpoints(
        &client,
        &endpoints,
        cache_dir,
        &ModLoaderType::Fabric,
        "1.21.4",
        "0.16.9",
    )
    .await
    .unwrap();
    assert_eq!(resolved.id, "fabric-1.21.4-0.16.9");
    assert_eq!(resolved.libraries.len(), 1);

    // Second lookup must hit the freshly written cache without making an HTTP request.
    let cached = resolve_loader_profile_with_endpoints(
        &client,
        &endpoints,
        cache_dir,
        &ModLoaderType::Fabric,
        "1.21.4",
        "0.16.9",
    )
    .await
    .unwrap();
    assert_eq!(cached.id, "fabric-1.21.4-0.16.9");

    let forge_err = resolve_loader_profile_with_endpoints(
        &client,
        &endpoints,
        cache_dir,
        &ModLoaderType::Forge,
        "1.20.1",
        "forge-1.20.1-47.3.0",
    )
    .await
    .unwrap_err();
    assert!(matches!(forge_err, LoaderError::Network(_)));

    let vanilla_err = resolve_loader_profile_with_endpoints(
        &client,
        &endpoints,
        cache_dir,
        &ModLoaderType::Vanilla,
        "1.21.4",
        "",
    )
    .await
    .unwrap_err();
    assert!(matches!(vanilla_err, LoaderError::VanillaNotSupported));

    let requests = server.finish().await;
    assert_eq!(requests.len(), 2);
}
