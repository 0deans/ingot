use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ItemEnchantment {
    pub id: String,
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ItemStack {
    /// Inventory slot (0-8 hotbar, 9-35 main); -1 for equipment
    pub slot: i32,
    pub id: String,
    pub count: i32,
    pub custom_name: Option<String>,
    pub enchanted: bool,
    pub enchantments: Vec<ItemEnchantment>,
    pub lore: Vec<String>,
    pub damage: i32,
    pub max_damage: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusEffect {
    pub id: String,
    pub amplifier: i32,
    /// Remaining ticks; -1 for infinite
    pub duration: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetails {
    pub name: String,
    pub uuid: Option<String>,
    pub online: bool,
    #[specta(type = i32)]
    pub health: f32,
    #[specta(type = i32)]
    pub max_health: f32,
    #[specta(type = i32)]
    pub absorption: f32,
    pub food: i32,
    #[specta(type = i32)]
    pub saturation: f32,
    pub xp_level: i32,
    #[specta(type = i32)]
    pub xp_progress: f32,
    pub gamemode: String,
    #[specta(type = i32)]
    pub x: f64,
    #[specta(type = i32)]
    pub y: f64,
    #[specta(type = i32)]
    pub z: f64,
    #[specta(type = i32)]
    pub yaw: f32,
    pub dimension: String,
    pub air: i32,
    pub on_fire: bool,
    pub selected_slot: i32,
    pub inventory: Vec<ItemStack>,
    pub head: Option<ItemStack>,
    pub chest: Option<ItemStack>,
    pub legs: Option<ItemStack>,
    pub feet: Option<ItemStack>,
    pub offhand: Option<ItemStack>,
    pub ender_items: Vec<ItemStack>,
    pub effects: Vec<StatusEffect>,
    /// Unix milliseconds, when known (offline players)
    #[specta(type = Option<i32>)]
    pub last_seen: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KnownPlayer {
    pub name: String,
    pub uuid: String,
    pub online: bool,
    #[specta(type = Option<i32>)]
    pub last_seen: Option<f64>,
}

pub fn gamemode_name(id: i64) -> &'static str {
    match id {
        1 => "creative",
        2 => "adventure",
        3 => "spectator",
        _ => "survival",
    }
}

pub fn uuid_from_ints(ints: &[i32]) -> Option<String> {
    if ints.len() != 4 {
        return None;
    }
    let hex: String = ints.iter().map(|v| format!("{:08x}", *v as u32)).collect();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

/// Minecraft usernames: 3-16 chars of [A-Za-z0-9_] (offline servers may be looser, but
/// this also keeps names safe to put into commands)
pub fn is_valid_player_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn bedrock_player_details(name: &str, online: bool, xuid: Option<String>) -> PlayerDetails {
    PlayerDetails {
        name: name.to_string(),
        uuid: xuid,
        online,
        health: 20.0,
        max_health: 20.0,
        absorption: 0.0,
        food: 20,
        saturation: 5.0,
        xp_level: 0,
        xp_progress: 0.0,
        gamemode: "survival".to_string(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        yaw: 0.0,
        dimension: "minecraft:overworld".to_string(),
        air: 300,
        on_fire: false,
        selected_slot: 0,
        inventory: Vec::new(),
        head: None,
        chest: None,
        legs: None,
        feet: None,
        offhand: None,
        ender_items: Vec::new(),
        effects: Vec::new(),
        last_seen: None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LiveError {
    #[error("Invalid player name: {0}")]
    InvalidPlayerName(String),
    #[error("{0} is not online")]
    PlayerNotOnline(String),
    #[error("No saved data for {0}")]
    NoSavedData(String),
    #[error("SNBT parse error: {0}")]
    Snbt(#[from] super::snbt::SnbtError),
    #[error("Server query error: {0}")]
    Query(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}
