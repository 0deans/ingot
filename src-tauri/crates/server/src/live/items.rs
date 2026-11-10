use super::models::{ItemEnchantment, ItemStack};
use super::nbt::Nbt;
use std::collections::HashMap;

/// Text components are either plain strings, JSON strings or `{text: ...}` compounds
pub fn json_component_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(parts) => {
            let text: String = parts.iter().filter_map(json_component_text).collect();
            (!text.is_empty()).then_some(text)
        }
        serde_json::Value::Object(fields) => {
            let mut text = fields
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(extra) = fields.get("extra") {
                if let Some(more) = json_component_text(extra) {
                    text.push_str(&more);
                }
            }
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

pub fn component_text(value: &Nbt) -> Option<String> {
    match value {
        Nbt::String(s) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(s) {
                if let Some(text) = json_component_text(&json) {
                    return Some(text);
                }
            }
            Some(s.clone())
        }
        Nbt::Compound(_) => {
            let mut text = value
                .get("text")
                .and_then(Nbt::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(extra) = value.get("extra") {
                for part in extra.as_list() {
                    if let Some(more) = component_text(part) {
                        text.push_str(&more);
                    }
                }
            }
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

pub fn collect_enchantments(value: &Nbt, levels: &mut HashMap<String, i32>) {
    match value.get("levels").unwrap_or(value) {
        Nbt::Compound(entries) => {
            for (id, level) in entries {
                if let Some(level) = level.as_f64().filter(|_| id.contains(':')) {
                    levels.insert(id.clone(), level as i32);
                }
            }
        }
        Nbt::List(entries) => {
            for entry in entries {
                if let (Some(id), Some(level)) = (
                    entry.get("id").and_then(Nbt::as_str),
                    entry.num("lvl").or_else(|| entry.num("level")),
                ) {
                    levels.insert(id.to_string(), level as i32);
                }
            }
        }
        _ => {}
    }
}

pub fn parse_item(item: &Nbt, slot: i32) -> Option<ItemStack> {
    let id = item.get("id")?.as_str()?.to_string();
    let count = item
        .num("count")
        .or_else(|| item.num("Count"))
        .unwrap_or(1.0) as i32;
    let components = item.get("components");
    let component = |key: &str| components.and_then(|c| c.get(key));
    let mut levels = HashMap::new();
    for source in [
        component("minecraft:enchantments"),
        component("minecraft:stored_enchantments"),
        item.get("Enchantments"),
        item.get("StoredEnchantments"),
        item.get("tag").and_then(|tag| tag.get("Enchantments")),
        item.get("tag")
            .and_then(|tag| tag.get("StoredEnchantments")),
    ]
    .into_iter()
    .flatten()
    {
        collect_enchantments(source, &mut levels);
    }
    let mut enchantments: Vec<ItemEnchantment> = levels
        .into_iter()
        .map(|(id, level)| ItemEnchantment { id, level })
        .collect();
    enchantments.sort_by(|a, b| a.id.cmp(&b.id));
    let enchanted = !enchantments.is_empty()
        || component("minecraft:enchantment_glint_override").and_then(Nbt::as_f64) == Some(1.0);
    let lore = component("minecraft:lore")
        .or_else(|| {
            item.get("tag")
                .and_then(|tag| tag.get("display"))
                .and_then(|display| display.get("Lore"))
        })
        .map(|value| value.as_list().iter().filter_map(component_text).collect())
        .unwrap_or_default();
    Some(ItemStack {
        slot,
        id,
        count,
        custom_name: component("minecraft:custom_name")
            .or_else(|| {
                item.get("tag")
                    .and_then(|tag| tag.get("display"))
                    .and_then(|display| display.get("Name"))
            })
            .and_then(component_text),
        enchanted,
        enchantments,
        lore,
        damage: component("minecraft:damage")
            .and_then(Nbt::as_f64)
            .unwrap_or(0.0) as i32,
        max_damage: component("minecraft:max_damage")
            .and_then(Nbt::as_f64)
            .map(|v| v as i32),
    })
}
