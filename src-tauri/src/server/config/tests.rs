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
    repair_offline_uuids(dir);
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
