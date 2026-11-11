//! Wire models, request validation, Steam environment filtering, and bounded line framing
//! for the local CLI waiter protocol.

use serde::{Deserialize, Serialize};
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

pub(super) const MAX_MESSAGE_BYTES: usize = 16 * 1024;
const MAX_STEAM_ENV_VALUE_BYTES: usize = 1024;
const MAX_MESSAGE_READ_LIMIT: u64 = (MAX_MESSAGE_BYTES + 1) as u64;

const ALLOWED_STEAM_ENV_KEYS: &[&str] = &[
    "SteamAppId",
    "SteamGameId",
    "SteamOverlayGameId",
    "SteamClientLaunch",
    "SteamEnv",
    "SteamVirtualGamepadInfo",
    "STEAM_GAME_ID",
    "STEAM_APP_ID",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct WaiterEndpointInfo {
    pub(super) port: u16,
    pub(super) token: String,
    pub(super) pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct WaiterLaunchRequest {
    pub(super) token: String,
    pub(super) instance_id: String,
    pub(super) steam_env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum WaiterLaunchResponse {
    Accepted { reservation_id: u64 },
    Finished { exit_code: i32 },
    Failed { message: String },
}

pub(super) struct ValidatedLaunchRequest {
    pub(super) instance_id: String,
    pub(super) steam_env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestValidationError {
    Malformed,
    InvalidToken,
}

impl RequestValidationError {
    pub(super) fn into_response(self) -> WaiterLaunchResponse {
        let message = match self {
            Self::Malformed => "Malformed waiter launch request",
            Self::InvalidToken => "Invalid waiter authentication token",
        };
        WaiterLaunchResponse::Failed {
            message: message.to_string(),
        }
    }
}

pub(super) fn validate_launch_request(
    raw_line: &str,
    expected_token: &str,
) -> Result<ValidatedLaunchRequest, RequestValidationError> {
    let request: WaiterLaunchRequest =
        serde_json::from_str(raw_line).map_err(|_| RequestValidationError::Malformed)?;
    if expected_token.is_empty() || request.token != expected_token {
        return Err(RequestValidationError::InvalidToken);
    }
    Ok(ValidatedLaunchRequest {
        instance_id: request.instance_id,
        steam_env: collect_steam_env(request.steam_env),
    })
}

/// Filters environment variables down to safe, non-injectable Steam game/overlay identifiers.
pub(super) fn collect_steam_env<I>(vars: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (String, String)>,
{
    vars.into_iter()
        .filter(|(key, value)| {
            ALLOWED_STEAM_ENV_KEYS.contains(&key.as_str())
                && !value.is_empty()
                && value.len() <= MAX_STEAM_ENV_VALUE_BYTES
                && !value.contains('\0')
        })
        .collect()
}

pub(super) async fn read_bounded_line<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    let bytes_read = reader
        .take(MAX_MESSAGE_READ_LIMIT)
        .read_until(b'\n', &mut buf)
        .await?;
    if bytes_read == 0 {
        return Ok(None);
    }
    if buf.len() > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "waiter message exceeded maximum length",
        ));
    }
    let line =
        String::from_utf8(buf).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(Some(line))
}

pub(super) async fn send_message<W, T>(writer: &mut W, message: &T) -> io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
    T: Serialize,
{
    let mut encoded = serde_json::to_vec(message).map_err(io::Error::other)?;
    encoded.push(b'\n');
    writer.write_all(&encoded).await?;
    writer.flush().await
}
