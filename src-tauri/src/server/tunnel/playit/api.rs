use super::{ClaimRejection, PlayitError, PlayitInner, PlayitTunnelStatus, PLAYIT_API};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub(crate) struct AgentContext {
    pub(crate) inner: Arc<Mutex<PlayitInner>>,
    pub(crate) client: reqwest::Client,
    pub(crate) binary_path: PathBuf,
    pub(crate) playit_dir: PathBuf,
    pub(crate) sandbox_dir: Option<PathBuf>,
}

impl AgentContext {
    pub(crate) async fn update(&self, f: impl FnOnce(&mut PlayitTunnelStatus)) {
        let mut guard = self.inner.lock().await;
        f(&mut guard.status);
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

    ctx.update(|s| {
        s.status = "claiming".to_string();
        s.claim_url = Some(claim_url);
    })
    .await;

    let version = format!("ingot {}", env!("CARGO_PKG_VERSION"));
    loop {
        let setup = api_call(
            &ctx.client,
            "/claim/setup",
            json!({ "code": code, "agent_type": "self-managed", "version": version }),
            None,
        )
        .await;

        match setup {
            Ok(data) if data.as_str() == Some("UserAccepted") => break,
            Ok(data) if data.as_str() == Some("UserRejected") => {
                return Err(PlayitError::ClaimRejected)
            }
            Ok(_) => {}
            Err(PlayitError::Rejected(ClaimRejection::Expired | ClaimRejection::InvalidCode)) => {
                return Err(PlayitError::ClaimExpired)
            }
            Err(error) => eprintln!("[Playit] Claim status check failed: {error}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    for _ in 0..10 {
        match api_call(
            &ctx.client,
            "/claim/exchange",
            json!({ "code": code }),
            None,
        )
        .await
        {
            Ok(data) => {
                if let Some(secret) = data["secret_key"].as_str() {
                    return Ok(secret.to_string());
                }
                return Err(PlayitError::InvalidResponse);
            }
            Err(PlayitError::Rejected(ClaimRejection::NotAccepted) | PlayitError::Network(_)) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
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

    ctx.update(|s| {
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
