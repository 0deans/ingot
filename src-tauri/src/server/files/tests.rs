use super::access::chrono_like_now;
use super::config::config_path;
use super::*;
use std::{fs, path::Path};

#[test]
fn property_escaping_round_trips() {
    let motd = "§6§lMy Server§r \\ ok";
    let escaped = escape_property(motd);
    assert_eq!(escaped, "\\u00A76\\u00A7lMy Server\\u00A7r \\\\ ok");
    assert_eq!(unescape_property(&escaped), motd);
    assert_eq!(unescape_property("\\u00a7aHi\\:there"), "§aHi:there");
}

#[test]
fn rejects_paths_outside_server() {
    let dir = Path::new("/srv");
    assert!(config_path(dir, "../secret.yml").is_err());
    assert!(config_path(dir, "/etc/passwd.yml").is_err());
    assert!(config_path(dir, "world/level.dat").is_err());
    assert!(config_path(dir, "whitelist.json").is_err());
    assert!(config_path(dir, "config/paper-global.yml").is_ok());
}

#[test]
fn ban_date_format() {
    let d = chrono_like_now();
    assert_eq!(d.len(), 25);
    assert!(d.ends_with(" +0000"));
}

#[test]
fn corrupt_access_lists_block_edits_and_preserve_recovery_data() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ops.json");
    for corrupt in ["{", "[42]", "[{}]"] {
        fs::write(&path, corrupt).unwrap();
        assert!(read_access_list(directory.path(), AccessListKind::Ops).is_err());
        assert!(add_access_entry(directory.path(), AccessListKind::Ops, "Notch", "uuid").is_err());
        assert!(remove_access_entry(directory.path(), AccessListKind::Ops, "Notch").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), corrupt);
    }
}

#[test]
fn atomic_config_replacement_retains_old_file_on_invalid_request() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("paper.yml"), "original").unwrap();
    assert!(write_config_file(directory.path(), "../paper.yml", "replacement").is_err());
    assert_eq!(
        fs::read_to_string(directory.path().join("paper.yml")).unwrap(),
        "original"
    );
    write_config_file(directory.path(), "paper.yml", "replacement").unwrap();
    assert_eq!(
        read_config_file(directory.path(), "paper.yml").unwrap(),
        "replacement"
    );
}

#[test]
fn property_updates_cannot_leave_duplicate_ports_disagreeing_with_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.properties");
    fs::write(&path, "server-port=25565\nserver-port=25567\n").unwrap();
    write_properties(
        directory.path(),
        &[PropertyEntry {
            key: "server-port".into(),
            value: "25566".into(),
        }],
    )
    .unwrap();
    let entries = read_properties(directory.path()).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|entry| entry.value == "25566"));
    let duplicate = [
        PropertyEntry {
            key: "server-port".into(),
            value: "25568".into(),
        },
        PropertyEntry {
            key: "server-port".into(),
            value: "25569".into(),
        },
    ];
    assert!(write_properties(directory.path(), &duplicate).is_err());
    assert!(read_properties(directory.path())
        .unwrap()
        .iter()
        .all(|entry| entry.value == "25566"));
}
