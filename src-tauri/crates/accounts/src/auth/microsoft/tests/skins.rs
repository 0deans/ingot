//! Hermetic unit and loopback tests for Microsoft and Minecraft Services skin management.
use crate::auth::microsoft::{
    skins::{skin_variant, MicrosoftSkinClient, MicrosoftSkinError},
    DEFAULT_MINECRAFT_SERVICES_BASE,
};
use ingot_test_support::{LoopbackServer, ResponseSpec};
use reqwest::StatusCode;

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
    _server: LoopbackServer,
}

impl MockServer {
    async fn start<F>(handler: F) -> Self
    where
        F: Fn(&str, &str, &str, &[u8]) -> (u16, Vec<(&'static str, &'static str)>, String)
            + Send
            + Sync
            + 'static,
    {
        let server = LoopbackServer::with_handler(move |request| {
            let (status, extra_headers, resp_body) = handler(
                &request.method,
                &request.path,
                &request.raw_headers,
                &request.body,
            );
            let mut spec = ResponseSpec::bytes(status, resp_body.into_bytes());
            for (k, v) in extra_headers {
                spec = spec.with_header(k, v);
            }
            spec
        })
        .await;

        Self {
            base_url: server.base_url().to_string(),
            _server: server,
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
            "errorMessage": "private-token private-password"
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

    assert!(matches!(
        err,
        MicrosoftSkinError::HttpStatus(StatusCode::BAD_REQUEST)
    ));
    assert!(!format!("{err} {err:?}").contains("private-"));
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
            r#"{"id":"id","name":"Steve","skins":"private-token"}"#.to_string(),
        )
    })
    .await;

    let client = MicrosoftSkinClient::with_endpoint(reqwest::Client::new(), &mock.base_url);
    let err = client
        .set_skin_from_url("token", "https://example.com/skin.png", true)
        .await
        .unwrap_err();

    assert!(matches!(err, MicrosoftSkinError::Response(_)));
    assert!(!format!("{err} {err:?}").contains("private-token"));
    assert!(std::error::Error::source(&err).is_some());
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
