use super::access::{
    add_to_server_whitelist, offline_uuid, read_server_whitelist, remove_from_server_whitelist,
    repair_offline_uuids,
};
use super::icons::{get_server_icon_base64, save_server_icon};
use super::models::{ServerConfig, ServerCoreType, ServerProperties};
use super::ports::{check_port, free_port, internal_port_for};
use super::properties::{read_server_properties_from_dir, write_server_properties_to_dir};

#[test]
fn offline_uuid_matches_java() {
    // UUID.nameUUIDFromBytes("OfflinePlayer:Notch".getBytes(UTF_8))
    assert_eq!(
        offline_uuid("Notch"),
        "b50ad385-829d-3141-a216-7e7d7539ba7f"
    );
}

fn make_server(id: &str, port: u16, internal: Option<u16>) -> ServerConfig {
    ServerConfig {
        id: id.into(),
        name: format!("Server {id}"),
        core: ServerCoreType::Paper,
        game_version: "26.2".into(),
        build_number: None,
        port,
        memory_min_mb: 1024,
        memory_max_mb: 2048,
        icon: None,
        java_path: None,
        jvm_args: None,
        auto_start: None,
        sleep_enabled: None,
        idle_timeout_seconds: None,
        internal_port: internal,
        playit_enabled: None,
        playit_secret_key: None,
        created_at: 0,
        last_run_at: None,
    }
}

#[test]
fn ports_account_for_the_internal_port() {
    let servers = vec![
        make_server("a", 25565, Some(35565)),
        make_server("b", 25570, Some(25580)),
    ];
    assert_eq!(
        free_port(&servers, "new"),
        25566,
        "25566 is now free right after 25565"
    );
    assert!(check_port(&servers, "new", 25565).is_err());
    assert!(check_port(&servers, "new", 25566).is_ok());
    assert!(
        check_port(&servers, "new", 35565).is_err(),
        "35565 is a's internal port"
    );
    // A server never conflicts with itself
    assert!(check_port(&servers, "a", 25565).is_ok());
    assert!(check_port(&servers, "new", 80).is_err());
    assert!(check_port(&servers, "new", u16::MAX).is_err());
}

#[test]
fn test_internal_port_bounds() {
    assert_eq!(internal_port_for(25565), 35565);
    assert_eq!(internal_port_for(60000), 50000);
}

#[test]
fn repairs_legacy_whitelist_uuid() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let legacy = uuid::Uuid::new_v3(&uuid::Uuid::nil(), b"OfflinePlayer:Notch").to_string();
    std::fs::write(
        dir.join("whitelist.json"),
        format!(r#"[{{"uuid":"{legacy}","name":"Notch"}}]"#),
    )
    .unwrap();
    repair_offline_uuids(dir).unwrap();
    let fixed = std::fs::read_to_string(dir.join("whitelist.json")).unwrap();
    assert!(
        fixed.contains("b50ad385-829d-3141-a216-7e7d7539ba7f"),
        "{fixed}"
    );
}

#[test]
fn test_server_properties_read_write_and_preservation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let mut props = ServerProperties {
        server_port: 25580,
        motd: "Custom Server MOTD".to_string(),
        max_players: 50,
        ..Default::default()
    };

    write_server_properties_to_dir(dir, &props).unwrap();

    let read_back = read_server_properties_from_dir(dir).unwrap();
    assert_eq!(read_back.server_port, 25580);
    assert_eq!(read_back.motd, "Custom Server MOTD");
    assert_eq!(read_back.max_players, 50);

    // Modify a field and rewrite, ensuring lines are preserved
    props.max_players = 100;
    write_server_properties_to_dir(dir, &props).unwrap();

    let second_read = read_server_properties_from_dir(dir).unwrap();
    assert_eq!(second_read.max_players, 100);
}

#[test]
fn test_whitelist_management() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    assert!(read_server_whitelist(dir).unwrap().is_empty());

    add_to_server_whitelist(dir, "PlayerOne").unwrap();
    let list1 = read_server_whitelist(dir).unwrap();
    assert_eq!(list1.len(), 1);
    assert_eq!(list1[0].name, "PlayerOne");

    // Adding duplicate (case-insensitive) should not add a second entry
    add_to_server_whitelist(dir, "playerone").unwrap();
    let list2 = read_server_whitelist(dir).unwrap();
    assert_eq!(list2.len(), 1);

    remove_from_server_whitelist(dir, "PLAYERONE").unwrap();
    let list3 = read_server_whitelist(dir).unwrap();
    assert!(list3.is_empty());
}

#[test]
fn test_server_icon_save_and_load() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    assert!(get_server_icon_base64(dir).is_none());

    let dummy_b64 = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    save_server_icon(dir, dummy_b64).unwrap();

    let loaded = get_server_icon_base64(dir);
    assert!(loaded.is_some());
    assert!(loaded.unwrap().starts_with("data:image/png;base64,"));
}

#[test]
fn corrupt_whitelist_is_preserved_by_failed_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("whitelist.json");
    for invalid in ["", "{", r#"[{"uuid":"known"}]"#] {
        std::fs::write(&path, invalid).unwrap();
        assert!(add_to_server_whitelist(directory.path(), "Notch").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    }
}

#[test]
fn properties_failures_do_not_replace_existing_bytes() {
    use crate::server::files::ServerFileError;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.properties");
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(matches!(
        write_server_properties_to_dir(directory.path(), &ServerProperties::default()),
        Err(ServerFileError::Io(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), [0xff, 0xfe]);
    std::fs::write(&path, "server-port=not-a-number\n").unwrap();
    assert!(matches!(
        read_server_properties_from_dir(directory.path()),
        Err(ServerFileError::InvalidProperty(_))
    ));
}

#[test]
fn bedrock_aliases_and_custom_properties_survive_atomic_updates() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("server.properties"),
        "# keep\nserver-name=old\nallow-list=false\nserver-portv6=19133\ncustom=retained\n",
    )
    .unwrap();
    let properties = ServerProperties {
        server_port: 19140,
        motd: "A §6name".into(),
        white_list: true,
        ..Default::default()
    };
    write_server_properties_to_dir(directory.path(), &properties).unwrap();
    let entries = crate::server::files::read_properties(directory.path()).unwrap();
    let value = |key: &str| {
        entries
            .iter()
            .find(|entry| entry.key == key)
            .unwrap()
            .value
            .as_str()
    };
    assert_eq!(value("server-name"), properties.motd);
    assert_eq!(value("server-portv6"), "19141");
    assert_eq!(value("allow-list"), "true");
    assert_eq!(value("custom"), "retained");
}

#[test]
fn property_updates_reject_missing_server_and_invalid_port_before_writing() {
    use super::service::{update_properties, ConfigError};
    use crate::minecraft::metadata::MetadataRepository;
    use crate::server::files::PropertyEntry;
    let directory = tempfile::tempdir().unwrap();
    let repository = MetadataRepository::<ServerConfig>::new(directory.path().join("servers.json"));
    let properties = directory.path().join("server.properties");
    std::fs::write(&properties, "server-port=25565\n").unwrap();
    let invalid = [PropertyEntry {
        key: "server-port".into(),
        value: "invalid".into(),
    }];
    assert!(matches!(
        update_properties(&repository, "missing", directory.path(), &invalid),
        Err(ConfigError::InvalidPort)
    ));
    let valid = [PropertyEntry {
        key: "server-port".into(),
        value: "25566".into(),
    }];
    assert!(update_properties(&repository, "missing", directory.path(), &valid).is_err());
    assert_eq!(
        std::fs::read_to_string(properties).unwrap(),
        "server-port=25565\n"
    );
    assert!(repository.load().unwrap().is_empty());
}

#[test]
fn property_port_update_publishes_metadata_and_preserves_unrelated_fields() {
    use super::service::update_properties;
    use crate::minecraft::metadata::MetadataRepository;
    use crate::server::files::PropertyEntry;
    let directory = tempfile::tempdir().unwrap();
    let repository = MetadataRepository::<ServerConfig>::new(directory.path().join("servers.json"));
    let server = make_server("a", 25565, Some(35565));
    repository
        .update(|records| {
            records.push(server);
            Ok(())
        })
        .unwrap();
    update_properties(
        &repository,
        "a",
        directory.path(),
        &[PropertyEntry {
            key: "server-port".into(),
            value: "25566".into(),
        }],
    )
    .unwrap();
    let stored = repository.load().unwrap();
    assert_eq!(stored[0].port, 25566);
    assert_eq!(stored[0].internal_port, Some(35566));
    assert_eq!(stored[0].game_version, "26.2");
    assert_eq!(
        read_server_properties_from_dir(directory.path())
            .unwrap()
            .server_port,
        25566
    );
}

fn creation_request(core: ServerCoreType) -> super::creation::ServerCreation {
    super::creation::ServerCreation {
        name: "New server".into(),
        core,
        game_version: "1.21.1".into(),
        build_number: None,
        port: None,
        memory_min_mb: None,
        memory_max_mb: None,
    }
}

#[test]
fn creation_preserves_java_and_bedrock_defaults_and_avoids_occupied_ports() {
    use super::creation::prepare_server_config;
    let existing = [make_server("existing", 25565, Some(35565))];
    let java = prepare_server_config(
        &existing,
        creation_request(ServerCoreType::Paper),
        "new-java".into(),
        123,
    )
    .unwrap();
    assert_eq!(java.port, 25566);
    assert_eq!(java.internal_port, Some(35566));
    assert_eq!((java.memory_min_mb, java.memory_max_mb), (2048, 4096));
    assert_eq!(java.sleep_enabled, Some(true));
    assert_eq!(java.created_at, 123);
    let bedrock = prepare_server_config(
        &existing,
        creation_request(ServerCoreType::Bedrock),
        "new-bedrock".into(),
        456,
    )
    .unwrap();
    assert_eq!(bedrock.port, 19132);
    assert_eq!((bedrock.memory_min_mb, bedrock.memory_max_mb), (1024, 2048));
    assert_eq!(bedrock.sleep_enabled, Some(false));
}

#[test]
fn creation_rejects_empty_names_invalid_memory_and_occupied_explicit_ports() {
    use super::{creation::prepare_server_config, error::ConfigError, models::PortError};
    let mut request = creation_request(ServerCoreType::Paper);
    request.name = " \t".into();
    assert!(matches!(
        prepare_server_config(&[], request, "new".into(), 0),
        Err(ConfigError::EmptyName)
    ));
    for (minimum, maximum) in [(0, 1024), (2048, 1024)] {
        let mut request = creation_request(ServerCoreType::Paper);
        request.memory_min_mb = Some(minimum);
        request.memory_max_mb = Some(maximum);
        assert!(matches!(
            prepare_server_config(&[], request, "new".into(), 0),
            Err(ConfigError::InvalidMemory { .. })
        ));
    }
    let mut request = creation_request(ServerCoreType::Paper);
    request.port = Some(25565);
    let existing = [make_server("existing", 25565, Some(35565))];
    assert!(matches!(
        prepare_server_config(&existing, request, "new".into(), 0),
        Err(ConfigError::Port(PortError::PortInUse { .. }))
    ));
}

#[test]
fn invalid_server_update_preserves_metadata_and_properties() {
    use crate::minecraft::metadata::MetadataRepository;
    let root = tempfile::tempdir().unwrap();
    let metadata_path = root.path().join("servers.json");
    let repository = MetadataRepository::new(metadata_path.clone());
    let server = make_server("a", 25565, Some(35565));
    repository
        .update(|records| {
            records.push(server.clone());
            Ok(())
        })
        .unwrap();
    let original_metadata = std::fs::read(&metadata_path).unwrap();
    let properties_path = root.path().join("server.properties");
    std::fs::write(&properties_path, "server-port=25565\ncustom=keep\n").unwrap();
    let mut invalid = server;
    invalid.memory_max_mb = 1;
    invalid.port = 25566;
    let result = super::service::update_server_record(&repository, invalid, root.path());
    assert!(matches!(
        result,
        Err(super::error::ConfigError::InvalidMemory { .. })
    ));
    assert_eq!(std::fs::read(&metadata_path).unwrap(), original_metadata);
    assert_eq!(
        std::fs::read_to_string(properties_path).unwrap(),
        "server-port=25565\ncustom=keep\n"
    );
}
