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
