use super::models::{bedrock_player_details, is_valid_player_name, LiveError, PlayerDetails};
use super::players::player_from_nbt;
use super::snbt::parse_snbt;
use crate::server::process::ServerProcessManager;
use std::collections::HashMap;
use std::time::Duration;

pub const ENTITY_DATA_MARKER: &str = " has the following entity data: ";

/// Lines that answer our internal queries; hidden from the user's console while pending
pub fn is_query_reply(line: &str) -> bool {
    line.contains(ENTITY_DATA_MARKER)
        || line.contains("No entity was found")
        || is_list_reply(line)
        || line.contains("TPS from last")
        || line.contains("Saving the game")
        || line.contains("Saved the game")
}

pub fn is_list_reply(line: &str) -> bool {
    line.contains("There are ") && line.contains("players online")
}

/// Parses `list uuids`: "There are 1 of a max of 20 players online: Steve (uuid), Alex (uuid)"
pub fn parse_list_reply(line: &str) -> Vec<(String, Option<String>)> {
    let Some(idx) = line.find("players online") else {
        return Vec::new();
    };
    let rest = &line[idx..];
    let Some(colon) = rest.find(':') else {
        return Vec::new();
    };
    rest[colon + 1..]
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.split_once(" (") {
            Some((name, uuid)) => (
                name.trim().to_string(),
                Some(uuid.trim_end_matches(')').to_string()),
            ),
            None => (entry.to_string(), None),
        })
        .collect()
}

pub async fn list_online(
    pm: &ServerProcessManager,
    server_id: &str,
) -> Result<Vec<(String, Option<String>)>, LiveError> {
    if pm.is_bedrock(server_id).await {
        return Ok(pm.get_bedrock_online_players(server_id).await);
    }
    let line = pm
        .query_typed(
            server_id,
            "list uuids",
            is_list_reply,
            Duration::from_secs(3),
        )
        .await
        .map_err(|error| LiveError::Query(Box::new(error)))?;
    Ok(parse_list_reply(&line))
}

pub async fn query_player(
    pm: &ServerProcessManager,
    server_id: &str,
    name: &str,
) -> Result<PlayerDetails, LiveError> {
    if !is_valid_player_name(name) {
        return Err(LiveError::InvalidPlayerName(name.to_string()));
    }
    if pm.is_bedrock(server_id).await {
        let xuid = pm.get_player_xuid(server_id, name).await;
        return Ok(bedrock_player_details(name, true, xuid));
    }
    let line = pm
        .query_typed(
            server_id,
            &format!("data get entity {name}"),
            |l| l.contains(ENTITY_DATA_MARKER) || l.contains("No entity was found"),
            Duration::from_secs(3),
        )
        .await
        .map_err(|error| LiveError::Query(Box::new(error)))?;
    let snbt = line
        .split_once(ENTITY_DATA_MARKER)
        .map(|(_, data)| data)
        .ok_or_else(|| LiveError::PlayerNotOnline(name.to_string()))?;
    let data = parse_snbt(snbt)?;
    Ok(player_from_nbt(name, &data, true))
}

/// Full details of every online player. Cached briefly because several UI panels
/// (player list, map) poll at the same time and each query costs console commands.
pub async fn online_players(
    pm: &ServerProcessManager,
    server_id: &str,
) -> Result<Vec<PlayerDetails>, LiveError> {
    use std::sync::Mutex;
    use std::time::Instant;
    type Cache = HashMap<String, (Instant, Vec<PlayerDetails>)>;
    static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

    if let Some((at, players)) = CACHE
        .lock()
        .ok()
        .and_then(|c| c.as_ref()?.get(server_id).cloned())
    {
        if at.elapsed() < Duration::from_millis(1500) {
            return Ok(players);
        }
    }

    if pm.is_bedrock(server_id).await {
        let players: Vec<PlayerDetails> = pm
            .get_bedrock_online_players(server_id)
            .await
            .into_iter()
            .map(|(name, xuid)| bedrock_player_details(&name, true, xuid))
            .collect();
        if let Ok(mut cache) = CACHE.lock() {
            cache
                .get_or_insert_with(HashMap::new)
                .insert(server_id.to_string(), (Instant::now(), players.clone()));
        }
        return Ok(players);
    }

    let mut players = Vec::new();
    for (name, uuid) in list_online(pm, server_id).await? {
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
