use super::models::{LogError, PasteProvider, PasteUploadResult};
use super::sanitize::{sanitize_log_content, SanitizeContext};
use serde::{Deserialize, Serialize};

pub const DEFAULT_MCLOGS_ENDPOINT: &str = "https://api.mclo.gs/1/log";
pub const DEFAULT_PASTEEE_ENDPOINT: &str = "https://api.paste.ee/v1/pastes";
const MAX_UPLOAD_LINES: usize = 25_000;
const MAX_UPLOAD_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct PasteEndpoints {
    pub mclogs_url: String,
    pub pasteee_url: String,
    pub pasteee_api_key: String,
}

impl Default for PasteEndpoints {
    fn default() -> Self {
        Self {
            mclogs_url: DEFAULT_MCLOGS_ENDPOINT.to_string(),
            pasteee_url: DEFAULT_PASTEEE_ENDPOINT.to_string(),
            pasteee_api_key: std::env::var("PASTEEE_API_KEY").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MclogsResponse {
    success: bool,
    id: Option<String>,
    url: Option<String>,
    raw: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct PasteEeRequest<'a> {
    description: &'a str,
    sections: Vec<PasteEeSection<'a>>,
}

#[derive(Debug, Serialize)]
struct PasteEeSection<'a> {
    name: &'a str,
    contents: &'a str,
}

#[derive(Debug, Deserialize)]
struct PasteEeResponse {
    success: Option<bool>,
    id: Option<String>,
    link: Option<String>,
}

pub fn cap_log_for_upload(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let sliced = if lines.len() > MAX_UPLOAD_LINES {
        &lines[lines.len() - MAX_UPLOAD_LINES..]
    } else {
        &lines[..]
    };
    let joined = sliced.join("\n");
    if joined.len() <= MAX_UPLOAD_BYTES {
        return joined;
    }
    let start = joined.len() - MAX_UPLOAD_BYTES;
    let valid_start = joined
        .char_indices()
        .find(|(idx, _)| *idx >= start)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    joined[valid_start..].to_string()
}

pub async fn upload_sanitized_log(
    client: &reqwest::Client,
    provider: PasteProvider,
    raw_content: &str,
    ctx: &SanitizeContext,
    endpoints: &PasteEndpoints,
) -> Result<PasteUploadResult, LogError> {
    if raw_content.trim().is_empty() {
        return Err(LogError::EmptyContent);
    }
    let sanitized = sanitize_log_content(raw_content, ctx);
    let bounded = cap_log_for_upload(&sanitized);
    if bounded.trim().is_empty() {
        return Err(LogError::EmptyContent);
    }

    match provider {
        PasteProvider::Mclogs => upload_to_mclogs(client, &bounded, &endpoints.mclogs_url).await,
        PasteProvider::PasteEe => {
            upload_to_pasteee(
                client,
                &bounded,
                &endpoints.pasteee_url,
                &endpoints.pasteee_api_key,
            )
            .await
        }
    }
}

async fn upload_to_mclogs(
    client: &reqwest::Client,
    content: &str,
    endpoint: &str,
) -> Result<PasteUploadResult, LogError> {
    let response = client
        .post(endpoint)
        .form(&[("content", content)])
        .send()
        .await
        .map_err(LogError::Transport)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(LogError::HttpStatus {
            status: status.as_u16(),
            message: body.chars().take(200).collect(),
        });
    }

    let parsed: MclogsResponse = response
        .json()
        .await
        .map_err(|e| LogError::InvalidResponse(e.to_string()))?;

    if !parsed.success {
        return Err(LogError::ProviderRejected(
            parsed
                .error
                .unwrap_or_else(|| "Unknown mclo.gs error".to_string()),
        ));
    }

    let id = parsed
        .id
        .ok_or_else(|| LogError::InvalidResponse("Missing paste ID from mclo.gs".to_string()))?;
    let url = parsed
        .url
        .ok_or_else(|| LogError::InvalidResponse("Missing paste URL from mclo.gs".to_string()))?;

    Ok(PasteUploadResult {
        provider: PasteProvider::Mclogs,
        id,
        url,
        raw_url: parsed.raw,
    })
}

async fn upload_to_pasteee(
    client: &reqwest::Client,
    content: &str,
    endpoint: &str,
    api_key: &str,
) -> Result<PasteUploadResult, LogError> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err(LogError::ProviderRejected(
            "paste.ee requires a free Application Key. Please set the PASTEEE_API_KEY environment variable, or use mclo.gs for free 1-click uploads without an account.".to_string(),
        ));
    }

    let payload = PasteEeRequest {
        description: "Ingot Minecraft Log",
        sections: vec![PasteEeSection {
            name: "latest.log",
            contents: content,
        }],
    };

    let response = client
        .post(endpoint)
        .header("X-Auth-Token", key)
        .json(&payload)
        .send()
        .await
        .map_err(LogError::Transport)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(LogError::HttpStatus {
            status: status.as_u16(),
            message: body.chars().take(200).collect(),
        });
    }

    let parsed: PasteEeResponse = response
        .json()
        .await
        .map_err(|e| LogError::InvalidResponse(e.to_string()))?;

    if parsed.success == Some(false) {
        return Err(LogError::ProviderRejected(
            "paste.ee rejected the upload".to_string(),
        ));
    }

    let id = parsed
        .id
        .ok_or_else(|| LogError::InvalidResponse("Missing paste ID from paste.ee".to_string()))?;
    let url = parsed
        .link
        .ok_or_else(|| LogError::InvalidResponse("Missing paste URL from paste.ee".to_string()))?;
    let raw_url = Some(format!("https://paste.ee/r/{id}"));

    Ok(PasteUploadResult {
        provider: PasteProvider::PasteEe,
        id,
        url,
        raw_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};

    async fn spawn_mock_server(
        status: u16,
        response_body: &'static str,
    ) -> (String, LoopbackServer) {
        let server = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
            "/upload",
            ResponseSpec::json(status, response_body),
        )])
        .await;
        (server.url("/upload"), server)
    }

    #[tokio::test]
    async fn uploads_sanitized_log_to_mclogs_endpoint() {
        let (url, server) = spawn_mock_server(
            200,
            r#"{"success":true,"id":"AbCdEf1","url":"https://mclo.gs/AbCdEf1","raw":"https://api.mclo.gs/1/raw/AbCdEf1"}"#,
        )
        .await;

        let endpoints = PasteEndpoints {
            mclogs_url: url,
            ..Default::default()
        };
        let ctx = SanitizeContext {
            usernames: vec!["SecretPlayer".to_string()],
            uuids: vec![],
            os_username: None,
            home_dirs: vec![],
        };
        let client = reqwest::Client::new();
        let res = upload_sanitized_log(
            &client,
            PasteProvider::Mclogs,
            "[12:00:00] [main/INFO]: Setting user: SecretPlayer --accessToken secret_tok_999",
            &ctx,
            &endpoints,
        )
        .await
        .unwrap();

        assert_eq!(res.provider, PasteProvider::Mclogs);
        assert_eq!(res.id, "AbCdEf1");
        assert_eq!(res.url, "https://mclo.gs/AbCdEf1");
        assert_eq!(
            res.raw_url.as_deref(),
            Some("https://api.mclo.gs/1/raw/AbCdEf1")
        );

        let captured = server.finish().await;
        assert_eq!(captured.len(), 1);
        let request_payload = &captured[0].raw_request;
        assert!(!request_payload.contains("SecretPlayer"));
        assert!(!request_payload.contains("secret_tok_999"));
        assert!(request_payload.contains("REDACTED"));
    }

    #[tokio::test]
    async fn uploads_sanitized_log_to_pasteee_endpoint() {
        let (url, server) = spawn_mock_server(
            201,
            r#"{"success":true,"id":"XyZ987","link":"https://paste.ee/p/XyZ987"}"#,
        )
        .await;

        let endpoints = PasteEndpoints {
            pasteee_url: url,
            pasteee_api_key: "test-key".to_string(),
            ..Default::default()
        };
        let ctx = SanitizeContext::default();
        let client = reqwest::Client::new();
        let res = upload_sanitized_log(
            &client,
            PasteProvider::PasteEe,
            "[12:00:00] [main/INFO]: Connecting to 198.51.100.24:25565",
            &ctx,
            &endpoints,
        )
        .await
        .unwrap();

        assert_eq!(res.provider, PasteProvider::PasteEe);
        assert_eq!(res.id, "XyZ987");
        assert_eq!(res.url, "https://paste.ee/p/XyZ987");
        assert_eq!(res.raw_url.as_deref(), Some("https://paste.ee/r/XyZ987"));

        let captured = server.finish().await;
        assert_eq!(captured.len(), 1);
        let request_payload = &captured[0].raw_request;
        assert!(!request_payload.contains("198.51.100.24"));
        assert!(request_payload.contains("[REDACTED_IP]"));
    }

    #[tokio::test]
    async fn rejects_pasteee_when_no_api_key_configured() {
        let endpoints = PasteEndpoints {
            pasteee_api_key: String::new(),
            ..Default::default()
        };
        let ctx = SanitizeContext::default();
        let client = reqwest::Client::new();
        let res = upload_sanitized_log(
            &client,
            PasteProvider::PasteEe,
            "test log",
            &ctx,
            &endpoints,
        )
        .await;
        assert!(matches!(res, Err(LogError::ProviderRejected(_))));
    }
}
