//! Live player data without server plugins.
//!
//! Online players are read through the console (`list uuids`, `data get entity <name>`),
//! which works on vanilla, Paper/Purpur and Fabric alike. Offline players are read from
//! their save files (`<world>/players/data/<uuid>.dat`, or `playerdata/` before 26.x).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::process::ServerProcessManager;

// ─── NBT value (shared by SNBT text and binary save files) ─────────────────────

#[derive(Debug, Clone)]
pub enum Nbt {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Nbt>),
    Compound(HashMap<String, Nbt>),
    ByteArray(Vec<i8>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Nbt {
    pub fn get(&self, key: &str) -> Option<&Nbt> {
        match self {
            Nbt::Compound(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        Some(match self {
            Nbt::Byte(v) => *v as f64,
            Nbt::Short(v) => *v as f64,
            Nbt::Int(v) => *v as f64,
            Nbt::Long(v) => *v as f64,
            Nbt::Float(v) => *v as f64,
            Nbt::Double(v) => *v,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Nbt::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> &[Nbt] {
        match self {
            Nbt::List(items) => items,
            _ => &[],
        }
    }

    fn num(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(Nbt::as_f64)
    }
}

impl From<fastnbt::Value> for Nbt {
    fn from(value: fastnbt::Value) -> Self {
        use fastnbt::Value as V;
        match value {
            V::Byte(v) => Nbt::Byte(v),
            V::Short(v) => Nbt::Short(v),
            V::Int(v) => Nbt::Int(v),
            V::Long(v) => Nbt::Long(v),
            V::Float(v) => Nbt::Float(v),
            V::Double(v) => Nbt::Double(v),
            V::String(v) => Nbt::String(v),
            V::ByteArray(v) => Nbt::ByteArray(v.into_inner()),
            V::IntArray(v) => Nbt::IntArray(v.into_inner()),
            V::LongArray(v) => Nbt::LongArray(v.into_inner()),
            V::List(items) => Nbt::List(items.into_iter().map(Nbt::from).collect()),
            V::Compound(map) => Nbt::Compound(map.into_iter().map(|(k, v)| (k, Nbt::from(v))).collect()),
        }
    }
}

// ─── SNBT parser (the text format printed by `data get`) ──────────────────────

struct SnbtParser<'a> {
    src: &'a [u8],
    pos: usize,
}

pub fn parse_snbt(text: &str) -> Result<Nbt, String> {
    let mut parser = SnbtParser { src: text.as_bytes(), pos: 0 };
    let value = parser.value()?;
    parser.skip_ws();
    if parser.pos != parser.src.len() {
        return Err(format!("Unexpected trailing data at {}", parser.pos));
    }
    Ok(value)
}

impl SnbtParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("Expected '{}' at {}", c as char, self.pos))
        }
    }

    fn value(&mut self) -> Result<Nbt, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.compound(),
            Some(b'[') => self.list(),
            Some(b'"') | Some(b'\'') => Ok(Nbt::String(self.quoted()?)),
            Some(_) => Ok(Self::scalar(&self.token()?)),
            None => Err("Unexpected end of SNBT".to_string()),
        }
    }

    fn compound(&mut self) -> Result<Nbt, String> {
        self.expect(b'{')?;
        let mut map = HashMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Nbt::Compound(map));
        }
        loop {
            self.skip_ws();
            let key = match self.peek() {
                Some(b'"') | Some(b'\'') => self.quoted()?,
                _ => self.token()?,
            };
            self.expect(b':')?;
            let value = self.value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Nbt::Compound(map));
                }
                _ => return Err(format!("Expected ',' or '}}' at {}", self.pos)),
            }
        }
    }

    fn list(&mut self) -> Result<Nbt, String> {
        self.expect(b'[')?;
        // Typed arrays: [B; ...], [I; ...], [L; ...]
        let array_kind = match (self.src.get(self.pos), self.src.get(self.pos + 1)) {
            (Some(k @ (b'B' | b'I' | b'L')), Some(b';')) => Some(*k),
            _ => None,
        };
        if array_kind.is_some() {
            self.pos += 2;
        }
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
        } else {
            loop {
                items.push(self.value()?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => self.pos += 1,
                    Some(b']') => {
                        self.pos += 1;
                        break;
                    }
                    _ => return Err(format!("Expected ',' or ']' at {}", self.pos)),
                }
            }
        }
        let nums = || items.iter().filter_map(Nbt::as_f64);
        Ok(match array_kind {
            Some(b'B') => Nbt::ByteArray(nums().map(|v| v as i8).collect()),
            Some(b'I') => Nbt::IntArray(nums().map(|v| v as i32).collect()),
            Some(b'L') => Nbt::LongArray(nums().map(|v| v as i64).collect()),
            _ => Nbt::List(items),
        })
    }

    fn quoted(&mut self) -> Result<String, String> {
        let quote = self.peek().ok_or("Unexpected end of SNBT")?;
        self.pos += 1;
        let mut bytes = Vec::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == quote {
                return String::from_utf8(bytes).map_err(|e| e.to_string());
            }
            if c == b'\\' {
                let escaped = self.peek().ok_or("Unexpected end of SNBT")?;
                self.pos += 1;
                bytes.push(match escaped {
                    b'n' => b'\n',
                    b't' => b'\t',
                    other => other,
                });
            } else {
                bytes.push(c);
            }
        }
        Err("Unterminated string".to_string())
    }

    fn token(&mut self) -> Result<String, String> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b'+'))
        {
            self.pos += 1;
        }
        if start == self.pos {
            return Err(format!("Unexpected character at {}", self.pos));
        }
        Ok(String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
    }

    fn scalar(token: &str) -> Nbt {
        match token {
            "true" => return Nbt::Byte(1),
            "false" => return Nbt::Byte(0),
            _ => {}
        }
        let (body, suffix) = match token.chars().last() {
            Some(c) if "bBsSlLfFdD".contains(c) && token.len() > 1 => (&token[..token.len() - 1], Some(c)),
            _ => (token, None),
        };
        let parsed = match suffix.map(|c| c.to_ascii_lowercase()) {
            Some('b') => body.parse().ok().map(Nbt::Byte),
            Some('s') => body.parse().ok().map(Nbt::Short),
            Some('l') => body.parse().ok().map(Nbt::Long),
            Some('f') => body.parse().ok().map(Nbt::Float),
            Some('d') => body.parse().ok().map(Nbt::Double),
            _ => body
                .parse()
                .ok()
                .map(Nbt::Int)
                .or_else(|| body.parse().ok().map(Nbt::Double)),
        };
        parsed.unwrap_or_else(|| Nbt::String(token.to_string()))
    }
}

// ─── Player data model ─────────────────────────────────────────────────────────
//
// Float fields are exported to TypeScript as plain `number` via `#[specta(type = i32)]`;
// specta would otherwise type them `number | null` (for NaN), which these never are.

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ItemStack {
    /// Inventory slot (0-8 hotbar, 9-35 main); -1 for equipment
    pub slot: i32,
    pub id: String,
    pub count: i32,
    pub custom_name: Option<String>,
    pub enchanted: bool,
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

fn gamemode_name(id: i64) -> &'static str {
    match id {
        1 => "creative",
        2 => "adventure",
        3 => "spectator",
        _ => "survival",
    }
}

/// Text components are either plain strings, JSON strings or `{text: ...}` compounds
fn component_text(value: &Nbt) -> Option<String> {
    match value {
        Nbt::String(s) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(s) {
                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                    return Some(text.to_string());
                }
                if let Some(text) = json.as_str() {
                    return Some(text.to_string());
                }
            }
            Some(s.clone())
        }
        Nbt::Compound(_) => value.get("text").and_then(Nbt::as_str).map(str::to_string),
        _ => None,
    }
}

fn parse_item(item: &Nbt, slot: i32) -> Option<ItemStack> {
    let id = item.get("id")?.as_str()?.to_string();
    let count = item.num("count").or_else(|| item.num("Count")).unwrap_or(1.0) as i32;
    let components = item.get("components");
    let component = |key: &str| components.and_then(|c| c.get(key));
    let enchanted = component("minecraft:enchantments").is_some_and(|e| match e {
        Nbt::Compound(map) => !map.is_empty(),
        _ => true,
    }) || component("minecraft:stored_enchantments").is_some()
        || component("minecraft:enchantment_glint_override").and_then(Nbt::as_f64) == Some(1.0);
    Some(ItemStack {
        slot,
        id,
        count,
        custom_name: component("minecraft:custom_name").and_then(component_text),
        enchanted,
        damage: component("minecraft:damage").and_then(Nbt::as_f64).unwrap_or(0.0) as i32,
        max_damage: component("minecraft:max_damage").and_then(Nbt::as_f64).map(|v| v as i32),
    })
}

fn uuid_from_ints(ints: &[i32]) -> Option<String> {
    if ints.len() != 4 {
        return None;
    }
    let hex: String = ints.iter().map(|v| format!("{:08x}", *v as u32)).collect();
    Some(format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32]))
}

pub fn player_from_nbt(name: &str, data: &Nbt, online: bool) -> PlayerDetails {
    let pos: Vec<f64> = data
        .get("Pos")
        .map(|p| p.as_list().iter().filter_map(Nbt::as_f64).collect())
        .unwrap_or_default();
    let rotation: Vec<f64> = data
        .get("Rotation")
        .map(|p| p.as_list().iter().filter_map(Nbt::as_f64).collect())
        .unwrap_or_default();

    let max_health = data
        .get("attributes")
        .or_else(|| data.get("Attributes"))
        .map(Nbt::as_list)
        .unwrap_or_default()
        .iter()
        .find(|a| {
            a.get("id").or_else(|| a.get("Name")).and_then(Nbt::as_str) == Some("minecraft:max_health")
        })
        .and_then(|a| a.num("base").or_else(|| a.num("Base")))
        .unwrap_or(20.0);

    let mut inventory = Vec::new();
    let (mut head, mut chest, mut legs, mut feet, mut offhand) = (None, None, None, None, None);
    for item in data.get("Inventory").map(Nbt::as_list).unwrap_or_default() {
        let slot = item.num("Slot").unwrap_or(-1.0) as i32;
        let Some(stack) = parse_item(item, slot) else { continue };
        // Before 1.21.5, armor and offhand were stored as special inventory slots
        match slot {
            103 => head = Some(stack),
            102 => chest = Some(stack),
            101 => legs = Some(stack),
            100 => feet = Some(stack),
            -106 => offhand = Some(stack),
            0..=35 => inventory.push(stack),
            _ => {}
        }
    }
    if let Some(equipment) = data.get("equipment") {
        let equip = |key: &str| equipment.get(key).and_then(|i| parse_item(i, -1));
        head = equip("head").or(head);
        chest = equip("chest").or(chest);
        legs = equip("legs").or(legs);
        feet = equip("feet").or(feet);
        offhand = equip("offhand").or(offhand);
    }

    let ender_items = data
        .get("EnderItems")
        .map(Nbt::as_list)
        .unwrap_or_default()
        .iter()
        .filter_map(|i| parse_item(i, i.num("Slot").unwrap_or(0.0) as i32))
        .collect();

    let effects = data
        .get("active_effects")
        .or_else(|| data.get("ActiveEffects"))
        .map(Nbt::as_list)
        .unwrap_or_default()
        .iter()
        .filter_map(|e| {
            let id = match e.get("id") {
                Some(Nbt::String(s)) => s.clone(),
                _ => return None,
            };
            Some(StatusEffect {
                id,
                amplifier: e.num("amplifier").unwrap_or(0.0) as i32,
                duration: e.num("duration").unwrap_or(0.0) as i32,
            })
        })
        .collect();

    let last_seen = data
        .get("Paper")
        .and_then(|p| p.num("LastSeen"))
        .or_else(|| data.get("bukkit").and_then(|b| b.num("lastPlayed")));

    PlayerDetails {
        name: name.to_string(),
        uuid: match data.get("UUID") {
            Some(Nbt::IntArray(ints)) => uuid_from_ints(ints),
            _ => None,
        },
        online,
        health: data.num("Health").unwrap_or(20.0) as f32,
        max_health: max_health as f32,
        absorption: data.num("AbsorptionAmount").unwrap_or(0.0) as f32,
        food: data.num("foodLevel").unwrap_or(20.0) as i32,
        saturation: data.num("foodSaturationLevel").unwrap_or(0.0) as f32,
        xp_level: data.num("XpLevel").unwrap_or(0.0) as i32,
        xp_progress: data.num("XpP").unwrap_or(0.0) as f32,
        gamemode: gamemode_name(data.num("playerGameType").unwrap_or(0.0) as i64).to_string(),
        x: pos.first().copied().unwrap_or(0.0),
        y: pos.get(1).copied().unwrap_or(0.0),
        z: pos.get(2).copied().unwrap_or(0.0),
        yaw: rotation.first().copied().unwrap_or(0.0) as f32,
        dimension: data
            .get("Dimension")
            .and_then(Nbt::as_str)
            .unwrap_or("minecraft:overworld")
            .to_string(),
        air: data.num("Air").unwrap_or(300.0) as i32,
        on_fire: data.num("Fire").unwrap_or(-20.0) > 0.0,
        selected_slot: data.num("SelectedItemSlot").unwrap_or(0.0) as i32,
        inventory,
        head,
        chest,
        legs,
        feet,
        offhand,
        ender_items,
        effects,
        last_seen,
    }
}

// ─── Console queries ──────────────────────────────────────────────────────────

const ENTITY_DATA_MARKER: &str = " has the following entity data: ";

/// Lines that answer our internal queries; hidden from the user's console while pending
pub fn is_query_reply(line: &str) -> bool {
    line.contains(ENTITY_DATA_MARKER)
        || line.contains("No entity was found")
        || is_list_reply(line)
        || line.contains("TPS from last")
        // Periodic saves for the live map
        || line.contains("Saving the game")
        || line.contains("Saved the game")
}

fn is_list_reply(line: &str) -> bool {
    line.contains("There are ") && line.contains("players online")
}

/// Parses `list uuids`: "There are 1 of a max of 20 players online: Steve (uuid), Alex (uuid)"
fn parse_list_reply(line: &str) -> Vec<(String, Option<String>)> {
    let Some(idx) = line.find("players online") else { return Vec::new() };
    let rest = &line[idx..];
    let Some(colon) = rest.find(':') else { return Vec::new() };
    rest[colon + 1..]
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.split_once(" (") {
            Some((name, uuid)) => (name.trim().to_string(), Some(uuid.trim_end_matches(')').to_string())),
            None => (entry.to_string(), None),
        })
        .collect()
}

/// Minecraft usernames: 3-16 chars of [A-Za-z0-9_] (offline servers may be looser, but
/// this also keeps names safe to put into commands)
pub fn is_valid_player_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 32 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub async fn list_online(pm: &ServerProcessManager, server_id: &str) -> Result<Vec<(String, Option<String>)>, String> {
    let line = pm
        .query(server_id, "list uuids", is_list_reply, Duration::from_secs(3))
        .await?;
    Ok(parse_list_reply(&line))
}

pub async fn query_player(pm: &ServerProcessManager, server_id: &str, name: &str) -> Result<PlayerDetails, String> {
    if !is_valid_player_name(name) {
        return Err(format!("Invalid player name: {name}"));
    }
    let line = pm
        .query(
            server_id,
            &format!("data get entity {name}"),
            |l| l.contains(ENTITY_DATA_MARKER) || l.contains("No entity was found"),
            Duration::from_secs(3),
        )
        .await?;
    let snbt = line
        .split_once(ENTITY_DATA_MARKER)
        .map(|(_, data)| data)
        .ok_or_else(|| format!("{name} is not online"))?;
    let data = parse_snbt(snbt)?;
    Ok(player_from_nbt(name, &data, true))
}

/// Full details of every online player. Cached briefly because several UI panels
/// (player list, map) poll at the same time and each query costs console commands.
pub async fn online_players(pm: &ServerProcessManager, server_id: &str) -> Result<Vec<PlayerDetails>, String> {
    use std::sync::Mutex;
    use std::time::Instant;
    type Cache = HashMap<String, (Instant, Vec<PlayerDetails>)>;
    static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

    if let Some((at, players)) = CACHE.lock().ok().and_then(|c| c.as_ref()?.get(server_id).cloned()) {
        if at.elapsed() < Duration::from_millis(1500) {
            return Ok(players);
        }
    }

    let mut players = Vec::new();
    for (name, uuid) in list_online(pm, server_id).await? {
        // A player may leave between `list` and `data get`; just skip them
        if let Ok(mut details) = query_player(pm, server_id, &name).await {
            details.uuid = details.uuid.or(uuid);
            players.push(details);
        }
    }
    if let Ok(mut cache) = CACHE.lock() {
        cache
            .get_or_insert_with(HashMap::new)
            .insert(server_id.to_string(), (Instant::now(), players.clone()));
    }
    Ok(players)
}

// ─── Save files ───────────────────────────────────────────────────────────────

/// Folders holding `<uuid>.dat` player files (26.x layout first)
fn player_data_dirs(world_dir: &Path) -> [PathBuf; 2] {
    [world_dir.join("players").join("data"), world_dir.join("playerdata")]
}

fn read_player_file(path: &Path) -> Option<Nbt> {
    let bytes = std::fs::read(path).ok()?;
    let mut decoder = flate2::read::GzDecoder::new(bytes.as_slice());
    let mut raw = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut raw).ok()?;
    fastnbt::from_bytes::<fastnbt::Value>(&raw).ok().map(Nbt::from)
}

/// name/uuid pairs the server has seen, from usercache.json
fn usercache(server_dir: &Path) -> HashMap<String, String> {
    #[derive(Deserialize)]
    struct Entry {
        name: String,
        uuid: String,
    }
    std::fs::read_to_string(server_dir.join("usercache.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<Entry>>(&raw).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|e| (e.uuid.to_lowercase(), e.name))
        .collect()
}

/// Every player with a save file, newest first
pub fn known_players(server_dir: &Path, world_dir: &Path) -> Vec<KnownPlayer> {
    let names = usercache(server_dir);
    let mut players: Vec<KnownPlayer> = Vec::new();
    for dir in player_data_dirs(world_dir) {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("dat") {
                continue;
            }
            let Some(uuid) = path.file_stem().and_then(|s| s.to_str()).map(str::to_lowercase) else { continue };
            if players.iter().any(|p| p.uuid == uuid) {
                continue;
            }
            let data = read_player_file(&path);
            let name = names.get(&uuid).cloned().or_else(|| {
                data.as_ref()
                    .and_then(|d| d.get("bukkit"))
                    .and_then(|b| b.get("lastKnownName"))
                    .and_then(Nbt::as_str)
                    .map(str::to_string)
            });
            let Some(name) = name else { continue };
            let last_seen = data.as_ref().and_then(|d| player_from_nbt(&name, d, false).last_seen).or_else(|| {
                entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as f64)
            });
            players.push(KnownPlayer { name, uuid, online: false, last_seen });
        }
    }
    players.sort_by(|a, b| b.last_seen.unwrap_or(0.0).total_cmp(&a.last_seen.unwrap_or(0.0)));
    players
}

/// Reads an offline player's last saved state
pub fn offline_player(server_dir: &Path, world_dir: &Path, name: &str) -> Result<PlayerDetails, String> {
    let known = known_players(server_dir, world_dir);
    let player = known
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("No saved data for {name}"))?;
    for dir in player_data_dirs(world_dir) {
        if let Some(data) = read_player_file(&dir.join(format!("{}.dat", player.uuid))) {
            let mut details = player_from_nbt(&player.name, &data, false);
            details.uuid = Some(player.uuid.clone());
            return Ok(details);
        }
    }
    Err(format!("No saved data for {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(p.uuid.as_deref(), Some("70ac6bb6-7042-3dbf-8136-47f7983fda63"));
        assert_eq!(p.inventory.len(), 2);
        assert!(p.inventory[0].enchanted);
        assert_eq!(p.inventory[0].custom_name.as_deref(), Some("Blade"));
        assert_eq!(p.inventory[0].damage, 12);
        assert_eq!(p.head.as_ref().map(|h| h.id.as_str()), Some("minecraft:iron_helmet"));
        assert_eq!(p.effects[0].amplifier, 1);
    }

    #[test]
    fn parses_list_reply() {
        let line = "[12:00:00 INFO]: There are 2 of a max of 20 players online: Steve (70ac6bb6-7042-3dbf-8136-47f7983fda63), Alex (0000)";
        let players = parse_list_reply(line);
        assert_eq!(players.len(), 2);
        assert_eq!(players[1].0, "Alex");
        assert!(parse_list_reply("There are 0 of a max of 20 players online:").is_empty());
    }
}

#[cfg(test)]
mod file_tests {
    /// Run with INGOT_TEST_SERVER_DIR=<server dir> cargo test -- --ignored
    #[test]
    #[ignore]
    fn reads_real_player_files() {
        let dir = std::path::PathBuf::from(std::env::var("INGOT_TEST_SERVER_DIR").unwrap());
        let known = super::known_players(&dir, &dir.join("world"));
        println!("{known:?}");
        let p = super::offline_player(&dir, &dir.join("world"), &known[0].name).unwrap();
        println!("{} hp={} food={} pos=({}, {}, {}) dim={} seen={:?}", p.name, p.health, p.food, p.x, p.y, p.z, p.dimension, p.last_seen);
    }
}
