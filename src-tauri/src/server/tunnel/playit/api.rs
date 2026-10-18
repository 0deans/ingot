use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex};

use super::adapter::PlayitAdapter;
use super::generation::Generation;
use super::{ClaimRejection, PlayitError, PlayitInner, PlayitTunnelStatus, PLAYIT_API};

pub(crate) struct AgentContext {
    pub(crate) inner: Arc<Mutex<PlayitInner>>,
    pub(crate) status_tx: Arc<watch::Sender<PlayitTunnelStatus>>,
    pub(crate) adapter: Arc<dyn PlayitAdapter>,
    pub(crate) client: reqwest::Client,
    pub(crate) generation: Generation,
    pub(crate) binary_path: PathBuf,
    pub(crate) playit_dir: PathBuf,
    pub(crate) sandbox_dir: Option<PathBuf>,
    pub(crate) cancel_rx: watch::Receiver<bool>,
}

impl AgentContext {
    pub(crate) async fn update_status(&self, f: impl FnOnce(&mut PlayitTunnelStatus)) -> bool {
        let mut guard = self.inner.lock().await;
        if guard.generation != self.generation.0 {
            return false;
        }
        f(&mut guard.status);
        let _ = self.status_tx.send(guard.status.clone());
        true
    }
}

/// Calls the playit API, which wraps responses as `{"status": "success", "data": ...}`
pub(crate) async fn api_call(
    client: &reqwest::Client,
    path: &str,
    body: Value,
    secret: Option<&str>,
) -> Result<Value, PlayitError> {
    let mut req = client.post(format!("{PLAYIT_API}{path}")).json(&body);
    if let Some(secret) = secret {
        req = req.header("Authorization", format!("Agent-Key {secret}"));
    }
    let response = req
        .send()
        .await
        .map_err(|error| PlayitError::Network(error.without_url()))?;
    if !response.status().is_success() {
        return Err(PlayitError::HttpStatus(response.status()));
    }
    let res: Value = response
        .json()
        .await
        .map_err(|error| PlayitError::Network(error.without_url()))?;
    decode_response(res)
}

pub(super) fn decode_response(response: Value) -> Result<Value, PlayitError> {
    if response["status"] == "success" {
        return response
            .get("data")
            .cloned()
            .ok_or(PlayitError::InvalidResponse);
    }
    if response["status"] != "fail" && response["status"] != "error" {
        return Err(PlayitError::InvalidResponse);
    }
    let data = &response["data"];
    let code = data
        .as_str()
        .or_else(|| data.get("type").and_then(Value::as_str));
    let rejection = match code {
        Some("CodeExpired") => ClaimRejection::Expired,
        Some("InvalidCode") => ClaimRejection::InvalidCode,
        Some("NotAccepted") => ClaimRejection::NotAccepted,
        _ => ClaimRejection::Other,
    };
    Err(PlayitError::Rejected(rejection))
}

/// Links this agent to the user's playit.gg account and returns its secret key
pub(crate) async fn claim_agent(ctx: &AgentContext) -> Result<String, PlayitError> {
    let code: String = uuid::Uuid::new_v4().simple().to_string()[..10].to_string();
    let claim_url = format!("https://playit.gg/claim/{code}");

    ctx.update_status(|s| {
        s.status = "claiming".to_string();
        s.claim_url = Some(claim_url);
    })
    .await;

    let mut cancel_rx = ctx.cancel_rx.clone();
    loop {
        if *cancel_rx.borrow() {
            return Err(PlayitError::Cancelled);
        }

        let setup = ctx.adapter.claim_setup(&ctx.client, &code).await;
        match setup {
            Ok(status) if status == "UserAccepted" => break,
            Ok(status) if status == "UserRejected" => return Err(PlayitError::ClaimRejected),
            Ok(_) => {}
            Err(PlayitError::Rejected(ClaimRejection::Expired | ClaimRejection::InvalidCode)) => {
                return Err(PlayitError::ClaimExpired)
            }
            Err(PlayitError::Cancelled) => return Err(PlayitError::Cancelled),
            Err(error) => eprintln!("[Playit] Claim status check failed: {error}"),
        }

        tokio::select! {
            _ = cancel_rx.changed() => return Err(PlayitError::Cancelled),
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
        }
    }

    for _ in 0..10 {
        if *cancel_rx.borrow() {
            return Err(PlayitError::Cancelled);
        }

        match ctx.adapter.claim_exchange(&ctx.client, &code).await {
            Ok(secret) => return Ok(secret),
            Err(PlayitError::Rejected(ClaimRejection::NotAccepted) | PlayitError::Network(_)) => {
                tokio::select! {
                    _ = cancel_rx.changed() => return Err(PlayitError::Cancelled),
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                }
            }
            Err(PlayitError::Cancelled) => return Err(PlayitError::Cancelled),
            Err(error) => return Err(error),
        }
    }

    Err(PlayitError::ClaimTimeout)
}

/// Picks the tunnel to show from `/v1/agents/rundata`, preferring Minecraft Java ones
pub(crate) async fn apply_rundata(ctx: &AgentContext, data: &Value) {
    let tunnels: Vec<&Value> = data["tunnels"]
        .as_array()
        .map(|t| {
            t.iter()
                .filter(|t| t["disabled_reason"].is_null())
                .collect()
        })
        .unwrap_or_default();

    let tunnel = tunnels
        .iter()
        .find(|t| t["tunnel_type"] == "minecraft-java")
        .or_else(|| tunnels.first());

    let address = tunnel
        .and_then(|t| t["display_address"].as_str())
        .map(str::to_string);
    let has_pending = data["pending"].as_array().is_some_and(|p| !p.is_empty());
    let notice = data["notices"]
        .as_array()
        .and_then(|n| n.first())
        .and_then(|n| n["message"].as_str())
        .map(str::to_string);

    ctx.update_status(|s| {
        s.public_address = address.clone();
        if address.is_some() {
            s.status = "connected".to_string();
            s.message = notice;
        } else if has_pending {
            s.status = "connecting".to_string();
            s.message = Some("playit.gg is allocating your tunnel...".to_string());
        } else {
            s.status = "no_tunnel".to_string();
            s.message = Some(notice.unwrap_or_else(|| {
                "Agent connected, but it has no tunnels. Add a \"Minecraft Java\" tunnel for this agent on playit.gg pointing to your server port.".to_string()
            }));
        }
    })
    .await;
}
