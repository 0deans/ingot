use super::models::ServerProperties;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parses server.properties from a key=value file format
pub fn read_server_properties_from_dir(
    server_dir: &Path,
) -> Result<ServerProperties, crate::server::files::ServerFileError> {
    let file_path = server_dir.join("server.properties");
    let contents = match fs::read_to_string(&file_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ServerProperties::default())
        }
        Err(error) => return Err(error.into()),
    };

    let mut map = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(
                k.trim().to_string(),
                crate::server::files::unescape_property(v.trim()),
            );
        }
    }

    let default = ServerProperties::default();
    Ok(ServerProperties {
        server_port: parse_setting(&map, "server-port", default.server_port)?,
        motd: map
            .get("motd")
            .or_else(|| map.get("server-name"))
            .cloned()
            .unwrap_or(default.motd),
        online_mode: parse_setting(&map, "online-mode", default.online_mode)?,
        difficulty: map.get("difficulty").cloned().unwrap_or(default.difficulty),
        gamemode: map.get("gamemode").cloned().unwrap_or(default.gamemode),
        max_players: parse_setting(&map, "max-players", default.max_players)?,
        pvp: parse_setting(&map, "pvp", default.pvp)?,
        view_distance: parse_setting(&map, "view-distance", default.view_distance)?,
        simulation_distance: parse_setting(
            &map,
            "simulation-distance",
            default.simulation_distance,
        )?,
        white_list: parse_setting(&map, "white-list", default.white_list)?,
        allow_flight: parse_setting(&map, "allow-flight", default.allow_flight)?,
        spawn_protection: parse_setting(&map, "spawn-protection", default.spawn_protection)?,
    })
}

/// Updates supported keys while retaining custom settings and Bedrock aliases.
pub fn write_server_properties_to_dir(
    server_dir: &Path,
    props: &ServerProperties,
) -> Result<(), crate::server::files::ServerFileError> {
    let entries = property_updates(server_dir, props)?;
    crate::server::files::write_properties(server_dir, &entries)
}

pub(crate) fn property_updates(
    server_dir: &Path,
    props: &ServerProperties,
) -> Result<Vec<crate::server::files::PropertyEntry>, crate::server::files::ServerFileError> {
    use crate::server::files::{read_properties, PropertyEntry};
    let existing = read_properties(server_dir)?;
    let values = [
        ("server-port", props.server_port.to_string()),
        ("query.port", props.server_port.to_string()),
        ("motd", props.motd.clone()),
        ("online-mode", props.online_mode.to_string()),
        ("difficulty", props.difficulty.clone()),
        ("gamemode", props.gamemode.clone()),
        ("max-players", props.max_players.to_string()),
        ("pvp", props.pvp.to_string()),
        ("view-distance", props.view_distance.to_string()),
        ("simulation-distance", props.simulation_distance.to_string()),
        ("white-list", props.white_list.to_string()),
        ("allow-flight", props.allow_flight.to_string()),
        ("spawn-protection", props.spawn_protection.to_string()),
    ];
    let mut updates: Vec<PropertyEntry> = values
        .into_iter()
        .map(|(key, value)| PropertyEntry {
            key: key.into(),
            value,
        })
        .collect();
    for (key, value) in [
        ("server-name", props.motd.clone()),
        ("allow-list", props.white_list.to_string()),
        (
            "server-portv6",
            props.server_port.saturating_add(1).to_string(),
        ),
    ] {
        if existing.iter().any(|entry| entry.key == key) {
            updates.push(PropertyEntry {
                key: key.into(),
                value,
            });
        }
    }
    Ok(updates)
}

fn parse_setting<T: std::str::FromStr>(
    values: &HashMap<String, String>,
    key: &str,
    default: T,
) -> Result<T, crate::server::files::ServerFileError> {
    match values.get(key) {
        Some(value) => value
            .parse()
            .map_err(|_| crate::server::files::ServerFileError::InvalidProperty(key.into())),
        None => Ok(default),
    }
}
