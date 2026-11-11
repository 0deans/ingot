//! Pure string parsing helpers for server console queries.

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
