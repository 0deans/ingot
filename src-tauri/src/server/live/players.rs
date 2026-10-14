use super::items::parse_item;
use super::models::{gamemode_name, uuid_from_ints, PlayerDetails, StatusEffect};
use super::nbt::Nbt;

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
            a.get("id").or_else(|| a.get("Name")).and_then(Nbt::as_str)
                == Some("minecraft:max_health")
        })
        .and_then(|a| a.num("base").or_else(|| a.num("Base")))
        .unwrap_or(20.0);

    let mut inventory = Vec::new();
    let (mut head, mut chest, mut legs, mut feet, mut offhand) = (None, None, None, None, None);
    for item in data.get("Inventory").map(Nbt::as_list).unwrap_or_default() {
        let slot = item.num("Slot").unwrap_or(-1.0) as i32;
        let Some(stack) = parse_item(item, slot) else {
            continue;
        };
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
