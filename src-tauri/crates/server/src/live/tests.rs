use super::items::parse_item;
use super::models::{bedrock_player_details, is_valid_player_name, uuid_from_ints};
use super::players::player_from_nbt;
use super::query::parse_list_reply;
use super::snbt::parse_snbt;

#[tokio::test]
async fn listing_stopped_server_preserves_process_failure() {
    use crate::server::process::{ProcessError, ServerProcessManager};
    use std::error::Error;

    let manager = ServerProcessManager::new();
    let error = super::query::list_online(&manager, "stopped-server")
        .await
        .unwrap_err();
    let source = error
        .source()
        .unwrap()
        .downcast_ref::<Box<ProcessError>>()
        .unwrap();
    assert!(matches!(source.as_ref(), ProcessError::NotRunning(id) if id == "stopped-server"));
}

#[tokio::test]
async fn querying_player_on_stopped_server_preserves_process_failure() {
    use crate::server::process::{ProcessError, ServerProcessManager};
    use std::error::Error;

    let manager = ServerProcessManager::new();
    let error = super::query::query_player(&manager, "stopped-server", "Alex")
        .await
        .unwrap_err();
    let source = error
        .source()
        .unwrap()
        .downcast_ref::<Box<ProcessError>>()
        .unwrap();
    assert!(matches!(source.as_ref(), ProcessError::NotRunning(id) if id == "stopped-server"));
}

#[tokio::test]
async fn invalid_player_name_is_rejected_before_querying_stopped_server() {
    let manager = crate::server::process::ServerProcessManager::new();
    let error = super::query::query_player(&manager, "stopped-server", "invalid player")
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        super::models::LiveError::InvalidPlayerName(name) if name == "invalid player"
    ));
}

#[test]
fn parses_entity_data() {
    let snbt = r#"{Health: 18.5f, foodLevel: 17, XpLevel: 3, XpP: 0.25f, playerGameType: 1, Pos: [12.5d, 64.0d, -3.25d], Rotation: [90.0f, 0.0f], Dimension: "minecraft:the_nether", UUID: [I; 1890347958, 1883389375, -2127149065, -1740645789], SelectedItemSlot: 2, Inventory: [{Slot: 0b, id: "minecraft:diamond_sword", count: 1, components: {"minecraft:damage": 12, "minecraft:enchantments": {"minecraft:sharpness": 5}, "minecraft:custom_name": 'Blade'}}, {Slot: 1b, id: "minecraft:cobblestone", count: 64}], equipment: {head: {id: "minecraft:iron_helmet", count: 1}}, active_effects: [{id: "minecraft:speed", amplifier: 1b, duration: 200, show_particles: true}], attributes: [{id: "minecraft:max_health", base: 20.0d}]}"#;
    let data = parse_snbt(snbt).unwrap();
    let p = player_from_nbt("Steve", &data, true);
    assert_eq!(p.health, 18.5);
    assert_eq!(p.food, 17);
    assert_eq!(p.gamemode, "creative");
    assert_eq!(p.dimension, "minecraft:the_nether");
    assert_eq!(p.z, -3.25);
    assert_eq!(
        p.uuid.as_deref(),
        Some("70ac6bb6-7042-3dbf-8136-47f7983fda63")
    );
    assert_eq!(p.inventory.len(), 2);
    assert!(p.inventory[0].enchanted);
    assert_eq!(p.inventory[0].enchantments[0].id, "minecraft:sharpness");
    assert_eq!(p.inventory[0].enchantments[0].level, 5);
    assert_eq!(p.inventory[0].custom_name.as_deref(), Some("Blade"));
    assert_eq!(p.inventory[0].damage, 12);
    assert_eq!(
        p.head.as_ref().map(|h| h.id.as_str()),
        Some("minecraft:iron_helmet")
    );
    assert_eq!(p.effects[0].amplifier, 1);
}

#[test]
fn parses_stored_and_legacy_item_details() {
    let modern = parse_snbt(
        r#"{id:"minecraft:enchanted_book",count:1,components:{"minecraft:stored_enchantments":{levels:{"minecraft:mending":1,"minecraft:unbreaking":3}},"minecraft:lore":['{"text":"A lucky find"}']}}"#,
    )
    .unwrap();
    let book = parse_item(&modern, 0).unwrap();
    assert!(book.enchanted);
    assert_eq!(book.enchantments.len(), 2);
    assert_eq!(book.enchantments[0].id, "minecraft:mending");
    assert_eq!(book.enchantments[1].level, 3);
    assert_eq!(book.lore, ["A lucky find"]);

    let legacy = parse_snbt(
        r#"{id:"minecraft:iron_sword",Count:1b,tag:{display:{Name:'{"text":"Old blade"}',Lore:['{"text":"One","extra":[{"text":" more"}]}']},Enchantments:[{id:"minecraft:sharpness",lvl:4s}]}}"#,
    )
    .unwrap();
    let sword = parse_item(&legacy, 0).unwrap();
    assert_eq!(sword.custom_name.as_deref(), Some("Old blade"));
    assert_eq!(sword.enchantments[0].level, 4);
    assert_eq!(sword.lore, ["One more"]);
}

#[test]
fn parses_list_reply() {
    let line = "[12:00:00 INFO]: There are 2 of a max of 20 players online: Steve (70ac6bb6-7042-3dbf-8136-47f7983fda63), Alex (0000)";
    let players = parse_list_reply(line);
    assert_eq!(players.len(), 2);
    assert_eq!(players[1].0, "Alex");
    assert!(parse_list_reply("There are 0 of a max of 20 players online:").is_empty());
}

#[test]
fn test_player_name_validation() {
    assert!(is_valid_player_name("Notch"));
    assert!(is_valid_player_name("Player_123"));
    assert!(!is_valid_player_name(""));
    assert!(!is_valid_player_name("Player With Spaces"));
    assert!(!is_valid_player_name("Player/With/Slash"));
}

#[test]
fn test_uuid_from_ints() {
    let ints = [1890347958, 1883389375, -2127149065, -1740645789];
    let uuid = uuid_from_ints(&ints);
    assert_eq!(
        uuid.as_deref(),
        Some("70ac6bb6-7042-3dbf-8136-47f7983fda63")
    );
    assert!(uuid_from_ints(&[1, 2, 3]).is_none());
}

#[test]
fn test_bedrock_player_details() {
    let details = bedrock_player_details("Steve", true, Some("123456789".to_string()));
    assert_eq!(details.name, "Steve");
    assert_eq!(details.uuid.as_deref(), Some("123456789"));
    assert!(details.online);
    assert_eq!(details.health, 20.0);
}

#[cfg(test)]
mod file_tests {
    #[test]
    #[ignore]
    fn reads_real_player_files() {
        let dir = std::path::PathBuf::from(std::env::var("INGOT_TEST_SERVER_DIR").unwrap());
        let known = crate::server::live::known_players(&dir, &dir.join("world"));
        println!("{known:?}");
        let p =
            crate::server::live::offline_player(&dir, &dir.join("world"), &known[0].name).unwrap();
        println!(
            "{} hp={} food={} pos=({}, {}, {}) dim={} seen={:?}",
            p.name, p.health, p.food, p.x, p.y, p.z, p.dimension, p.last_seen
        );
    }
}
