use super::error::WorldError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum WorldDimension {
    Nether,
    TheEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldSummary {
    pub folder_name: String,
    pub display_name: String,
    pub instance_id: String,
    pub instance_name: String,
    pub game_mode: String,
    pub is_hardcore: bool,
    pub seed: Option<String>,
    pub last_played: Option<u64>,
    pub size_bytes: u64,
    pub version_name: Option<String>,
    pub icon: Option<String>,
    pub has_nether: bool,
    pub has_the_end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldBackupInfo {
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldResetResult {
    pub dimension: WorldDimension,
    pub reset_paths: Vec<String>,
    pub backup_path: Option<String>,
}

pub fn validate_world_folder_name(name: &str) -> Result<&str, WorldError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(WorldError::InvalidFolderName(
            "Folder name cannot be empty".to_string(),
        ));
    }
    if trimmed.starts_with('.') || trimmed == "qp_world" || trimmed.starts_with(".qp_") {
        return Err(WorldError::InvalidFolderName(format!(
            "Reserved or hidden folder name: {trimmed}"
        )));
    }
    if trimmed.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(WorldError::InvalidFolderName(format!(
            "Folder name contains invalid characters: {trimmed}"
        )));
    }
    // Check Windows reserved names (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or(trimmed)
        .to_ascii_uppercase();
    let is_reserved = matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if is_reserved {
        return Err(WorldError::InvalidFolderName(format!(
            "Folder name is a reserved system name: {trimmed}"
        )));
    }
    Ok(trimmed)
}
