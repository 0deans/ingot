//! Hermetic unit and loopback tests for Microsoft and Minecraft Services skin management.
use crate::auth::microsoft::{
    skins::{skin_variant, MicrosoftSkinClient, MicrosoftSkinError},
    DEFAULT_MINECRAFT_SERVICES_BASE,
};
use reqwest::StatusCode;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[test]
fn test_skin_variant_mapping() {
    assert_eq!(skin_variant(true), "slim");
    assert_eq!(skin_variant(false), "classic");
    assert_eq!(
        DEFAULT_MINECRAFT_SERVICES_BASE,
        "https://api.minecraftservices.com"
    );
}

struct MockServer {
    base_url: String,
    _handle: tokio::task::JoinHandle<()>,
}

impl MockServer {
    async fn start<F>(handler: F) -> Self
    where
        F: Fn(&str, &str, &str, &[u8]) -> (u16, Vec<(&'static str, &'static str)>, String)
            + Send
            + Sync
            + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let handler = Arc::new(handler);

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let handler = Arc::clone(&handler);

                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    let mut bytes = Vec::new();
                    let (method, path, headers_str, body) = loop {
                        let count = match stream.read(&mut buffer).await {
                            Ok(c) if c > 0 => c,
                            _ => return,
                        };
                        bytes.extend_from_slice(&buffer[..count]);

                        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let header_str = String::from_utf8_lossy(&bytes[..pos]).to_string();
                            let mut lines = header_str.lines();
                            let req_line = lines.next().unwrap_or("");
                            let mut parts = req_line.split_whitespace();
                            let method = parts.next().unwrap_or("GET").to_string();
                            let path = parts.next().unwrap_or("/").to_string();

                            let mut content_length = 0;
                            for line in lines {
                                if let Some((k, v)) = line.split_once(':') {
                                    if k.trim().eq_ignore_ascii_case("content-length") {
                                        content_length = v.trim().parse().unwrap_or(0);
                                    }
                                }
                            }

                            let body_start = pos + 4;
                            let remaining = bytes.len() - body_start;
                            if remaining >= content_length {
                                let body = bytes[body_start..body_start + content_length].to_vec();
                                break (method, path, header_str, body);
                            }
                        }
                    };

                    let (status, extra_headers, resp_body) =
                        handler(&method, &path, &headers_str, &body);
                    let mut response = format!(
                        "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n",
                        resp_body.len()
                    );
                    for (k, v) in extra_headers {
                        response.push_str(&format!("{k}: {v}\r\n"));
                    }
                    response.push_str("\r\n");
                    response.push_str(&resp_body);

                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.shutdown().await;
                });
            }
        });

        Self {
            base_url,
            _handle: handle,
        }
    }
}

#[tokio::test]
async fn test_set_skin_from_url_success() {
    let mock = MockServer::start(|method, path, headers, body| {
        assert_eq!(method, "POST");
        assert_eq!(path, "/minecraft/profile/skins");
        assert!(
            headers.contains("authorization: Bearer test-token-123")
                || headers.contains("Authorization: Bearer test-token-123")
        );

        let body_json: serde_json::Value = serde_json::from_slice(body).unwrap();
        assert_eq!(body_json["variant"], "slim");
        assert_eq!(body_json["url"], "https://example.com/steve.png");

        let profile_json = serde_json::json!({
            "id": "player-uuid",
            "name": "Steve",
            "skins": [
                { "state": "INACTIVE", "url": "https://textures.minecraft.net/old.png" },
                { "state": "ACTIVE", "url": "https://textures.minecraft.net/new.png" }
            ]
        });

        (
            200,
            vec![("Content-Type", "application/json")],
            profile_json.to_string(),
        )
    })
    .await;

    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), &mock.base_url);
    let result = client
        .set_skin_from_url("test-token-123", "https://example.com/steve.png", true)
        .await
        .expect("set_skin_from_url should succeed");

    assert_eq!(
        result,
        Some("https://textures.minecraft.net/new.png".to_string())
    );
}

#[tokio::test]
async fn test_upload_skin_multipart_success() {
    let mock = MockServer::start(|method, path, headers, _| {
        assert_eq!(method, "POST");
        assert_eq!(path, "/minecraft/profile/skins");
        assert!(
            headers.contains("authorization: Bearer upload-token")
                || headers.contains("Authorization: Bearer upload-token")
        );
        assert!(headers
            .to_lowercase()
            .contains("content-type: multipart/form-data"));

        let profile_json = serde_json::json!({
            "id": "player-uuid",
            "name": "Alex",
            "skins": [
                { "state": "ACTIVE", "url": "https://textures.minecraft.net/uploaded.png" }
            ]
        });

        (
            200,
            vec![("Content-Type", "application/json")],
            profile_json.to_string(),
        )
    })
    .await;

    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), &mock.base_url);
    let dummy_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let result = client
        .upload_skin("upload-token", dummy_png, false)
        .await
        .expect("upload_skin should succeed");

    assert_eq!(
        result,
        Some("https://textures.minecraft.net/uploaded.png".to_string())
    );
}

#[tokio::test]
async fn test_skin_refused_with_error_message() {
    let mock = MockServer::start(|_, _, _, _| {
        let err_json = serde_json::json!({
            "errorMessage": "Invalid image dimensions: expected 64x64 or 64x32"
        });
        (
            400,
            vec![("Content-Type", "application/json")],
            err_json.to_string(),
        )
    })
    .await;

    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), &mock.base_url);
    let err = client
        .set_skin_from_url("token", "https://example.com/bad.png", false)
        .await
        .unwrap_err();

    match err {
        MicrosoftSkinError::Refused(reason) => {
            assert_eq!(reason, "Invalid image dimensions: expected 64x64 or 64x32");
        }
        other => panic!("Expected Refused error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_skin_refused_http_status_when_no_message() {
    let mock = MockServer::start(|_, _, _, _| (401, vec![], "Unauthorized".to_string())).await;

    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), &mock.base_url);
    let err = client
        .set_skin_from_url("expired-token", "https://example.com/skin.png", true)
        .await
        .unwrap_err();

    match err {
        MicrosoftSkinError::HttpStatus(status) => {
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        }
        other => panic!("Expected HttpStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_skin_response_decode_error() {
    let mock = MockServer::start(|_, _, _, _| {
        (
            200,
            vec![("Content-Type", "application/json")],
            "{\"broken\": true}".to_string(),
        )
    })
    .await;

    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), &mock.base_url);
    let err = client
        .set_skin_from_url("token", "https://example.com/skin.png", true)
        .await
        .unwrap_err();

    match err {
        MicrosoftSkinError::Response(_) => {}
        other => panic!("Expected Response decode error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_skin_transport_error() {
    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), "http://127.0.0.1:1");

    let err = client
        .set_skin_from_url("token", "https://example.com/skin.png", true)
        .await
        .unwrap_err();

    match err {
        MicrosoftSkinError::Transport(_) => {}
        other => panic!("Expected Transport error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_active_skin_missing_in_profile_returns_none() {
    let mock = MockServer::start(|_, _, _, _| {
        let profile_json = serde_json::json!({
            "id": "player-uuid",
            "name": "Steve",
            "skins": []
        });

        (
            200,
            vec![("Content-Type", "application/json")],
            profile_json.to_string(),
        )
    })
    .await;

    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), &mock.base_url);
    let result = client
        .set_skin_from_url("token", "https://example.com/skin.png", true)
        .await
        .expect("call should succeed");

    assert_eq!(result, None);
}
