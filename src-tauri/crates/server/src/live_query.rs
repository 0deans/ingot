//! Live query orchestration for active server processes.

use ingot_server_map::live::{
    bedrock_player_details, is_valid_player_name, parse_snbt, player_from_nbt, PlayerDetails,
};
use std::collections::HashMap;
use std::time::Duration;

use crate::process::{ProcessError, ServerProcessManager};

pub const ENTITY_DATA_MARKER: &str = " has the following entity data: ";

#[derive(Debug, thiserror::Error)]
pub enum LiveQueryError {
    #[error("Invalid player name: {0}")]
    InvalidPlayerName(String),
    #[error("{0} is not online")]
    PlayerNotOnline(String),
    #[error("SNBT parse error: {0}")]
    Snbt(#[from] ingot_server_map::live::snbt::SnbtError),
    #[error("Server query error: {0}")]
    Query(#[source] Box<ProcessError>),
}

impl From<ProcessError> for LiveQueryError {
    fn from(error: ProcessError) -> Self {
        Self::Query(Box::new(error))
    }
}

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
) -> Result<Vec<(String, Option<String>)>, LiveQueryError> {
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
        .map_err(|error| LiveQueryError::Query(Box::new(error)))?;
    Ok(parse_list_reply(&line))
}

pub async fn query_player(
    pm: &ServerProcessManager,
    server_id: &str,
    name: &str,
) -> Result<PlayerDetails, LiveQueryError> {
    if !is_valid_player_name(name) {
        return Err(LiveQueryError::InvalidPlayerName(name.to_string()));
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
        .map_err(|error| LiveQueryError::Query(Box::new(error)))?;
    let snbt = line
        .split_once(ENTITY_DATA_MARKER)
        .map(|(_, data)| data)
        .ok_or_else(|| LiveQueryError::PlayerNotOnline(name.to_string()))?;
    let data = parse_snbt(snbt)?;
    Ok(player_from_nbt(name, &data, true))
}

/// Full details of every online player. Cached briefly because several UI panels
/// (player list, map) poll at the same time and each query costs console commands.
pub async fn online_players(
    pm: &ServerProcessManager,
    server_id: &str,
) -> Result<Vec<PlayerDetails>, LiveQueryError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[tokio::test]
    async fn listing_stopped_server_preserves_process_failure() {
        let manager = ServerProcessManager::new();
        let error = list_online(&manager, "stopped-server").await.unwrap_err();
        let source = error
            .source()
            .unwrap()
            .downcast_ref::<Box<ProcessError>>()
            .unwrap();
        assert!(matches!(source.as_ref(), ProcessError::NotRunning(id) if id == "stopped-server"));
    }

    #[tokio::test]
    async fn querying_player_on_stopped_server_preserves_process_failure() {
        let manager = ServerProcessManager::new();
        let error = query_player(&manager, "stopped-server", "Alex")
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
        let manager = ServerProcessManager::new();
        let error = query_player(&manager, "stopped-server", "invalid player")
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            LiveQueryError::InvalidPlayerName(name) if name == "invalid player"
        ));
    }
}
