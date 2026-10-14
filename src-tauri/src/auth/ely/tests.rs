//! Unit and hermetic loopback tests for Ely.by auth, catalog, web session, and skin management.
use super::*;
use reqwest::StatusCode;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[test]
fn test_build_oauth_complete_url() {
    let url = "https://account.ely.by/oauth2/v1/ely?scope=account_info%2Caccount_email&state=826fcd58c1a95a8f7418ac1211990821&response_type=code&redirect_uri=https%3A%2F%2Fely.by%2Fauthorization%2Foauth";
    let complete_url = web_session::build_oauth_complete_url("https://account.ely.by", url);
    assert!(complete_url.starts_with("https://account.ely.by/api/oauth2/v1/complete?"));
    assert!(complete_url.contains("client_id=ely"));
    assert!(complete_url.contains("scope=account_info%20account_email"));
    assert!(complete_url.contains("state=826fcd58c1a95a8f7418ac1211990821"));
    assert!(complete_url.contains("redirect_uri=https%3A%2F%2Fely.by%2Fauthorization%2Foauth"));
}

#[test]
fn test_build_catalog_query() {
    assert_eq!(catalog::build_catalog_query(1, None, None, None), "page=1");

    assert_eq!(
        catalog::build_catalog_query(2, Some("steve"), Some("latest"), Some("Erick")),
        "tags[0]=steve&sort=time&uploader=Erick&page=2"
    );

    assert_eq!(
        catalog::build_catalog_query(1, Some("steve alex warrior"), Some("popular"), None),
        "tags[0]=steve&tags[1]=alex&tags[2]=warrior&sort=best&page=1"
    );

    assert_eq!(
        catalog::build_catalog_query(1, None, Some("default"), None),
        "page=1"
    );
}

#[test]
fn test_parse_slim_filter() {
    assert_eq!(catalog::parse_slim_filter(Some("slim")), Some(true));
    assert_eq!(catalog::parse_slim_filter(Some("ALEX")), Some(true));
    assert_eq!(catalog::parse_slim_filter(Some("steve")), Some(false));
    assert_eq!(catalog::parse_slim_filter(Some("classic")), Some(false));
    assert_eq!(catalog::parse_slim_filter(Some("new")), Some(false));
    assert_eq!(catalog::parse_slim_filter(Some("unknown")), None);
    assert_eq!(catalog::parse_slim_filter(None), None);
}

#[test]
fn test_extract_skin_id() {
    let json1 = serde_json::json!({
        "error": "success",
        "url": "/skins/s98765/edit",
        "text": "Success"
    });
    assert_eq!(skins::extract_skin_id(&json1), Some(98765));

    let json2 = serde_json::json!({
        "id": 123456,
        "success": true
    });
    assert_eq!(skins::extract_skin_id(&json2), Some(123456));

    let json3 = serde_json::json!({
        "url": "https://ely.by/skins/s45678",
    });
    assert_eq!(skins::extract_skin_id(&json3), Some(45678));

    let json4 = serde_json::json!({
        "id": "78910"
    });
    assert_eq!(skins::extract_skin_id(&json4), Some(78910));

    let json5 = serde_json::json!({
        "extra": { "skinId": 555 }
    });
    assert_eq!(skins::extract_skin_id(&json5), Some(555));

    let json_empty = serde_json::json!({});
    assert_eq!(skins::extract_skin_id(&json_empty), None);
}

#[test]
fn test_strip_html_tags() {
    assert_eq!(skins::strip_html_tags("Hello World"), "Hello World");
    assert_eq!(
        skins::strip_html_tags("<p>Error: <b>Forbidden</b></p>"),
        "Error: Forbidden"
    );
    assert_eq!(skins::strip_html_tags(""), "");
}

#[test]
fn test_check_ely_web_response() {
    let ok1 = serde_json::json!({ "success": true });
    assert!(skins::check_ely_web_response(&ok1).is_ok());

    let ok2 = serde_json::json!({ "error": "success" });
    assert!(skins::check_ely_web_response(&ok2).is_ok());

    let err1 = serde_json::json!({
        "error": "failed",
        "text": "<span>Access Denied</span>"
    });
    match skins::check_ely_web_response(&err1) {
        Err(ElyError::Api(msg)) => assert_eq!(msg, "Access Denied"),
        other => panic!("Expected Api error, got {other:?}"),
    }

    let err2 = serde_json::json!({
        "success": false,
        "message": "Invalid password"
    });
    match skins::check_ely_web_response(&err2) {
        Err(ElyError::Api(msg)) => assert_eq!(msg, "Invalid password"),
        other => panic!("Expected Api error, got {other:?}"),
    }
}

struct MockServer {
    base_url: String,
    _handle: tokio::task::JoinHandle<()>,
}

impl MockServer {
    async fn start<F>(handler: F) -> Self
    where
        F: Fn(&str, &str, &[u8]) -> (u16, Vec<(&'static str, &'static str)>, String)
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
                    let (method, path, body) = loop {
                        let count = match stream.read(&mut buffer).await {
                            Ok(c) if c > 0 => c,
                            _ => return,
                        };
                        bytes.extend_from_slice(&buffer[..count]);

                        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let header_str = String::from_utf8_lossy(&bytes[..pos]);
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
                                break (method, path, body);
                            }
                        }
                    };

                    let (status, extra_headers, resp_body) = handler(&method, &path, &body);
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
async fn test_fetch_catalog_hermetic() {
    let mock = MockServer::start(|method, path, _| {
        assert_eq!(method, "GET");
        assert!(path.starts_with("/api/legacy/skins?"));

        let catalog_json = serde_json::json!({
            "items": [
                {
                    "id": 101,
                    "skin_url": "https://ely.by/skin1.png",
                    "is_slim": true,
                    "count_wearers": 5,
                    "count_cubes": 10,
                    "count_views_total": 20,
                    "tags": ["warrior"]
                },
                {
                    "id": 102,
                    "skin_url": "https://ely.by/skin2.png",
                    "is_slim": false,
                    "count_wearers": 2,
                    "count_cubes": 1,
                    "count_views_total": 5,
                    "tags": ["steve"]
                }
            ],
            "total_items": 100,
            "current": 1,
            "last": 5
        });

        (
            200,
            vec![("Content-Type", "application/json")],
            catalog_json.to_string(),
        )
    })
    .await;

    let service =
        ElyAuthService::with_endpoints(reqwest::Client::new(), &mock.base_url, &mock.base_url);

    // Slim filter true should keep only item 101
    let resp = service
        .fetch_catalog(1, None, None, Some("slim".to_string()), None)
        .await
        .expect("fetch_catalog should succeed");

    assert_eq!(resp.total_items, 100);
    assert_eq!(resp.current_page, 1);
    assert_eq!(resp.last_page, 5);
    assert_eq!(resp.items.len(), 1);
    assert_eq!(resp.items[0].id, 101);
    assert!(resp.items[0].is_slim);
    assert_eq!(resp.items[0].tags, vec!["warrior".to_string()]);
}

#[tokio::test]
async fn test_fetch_catalog_http_error() {
    let mock = MockServer::start(|_, _, _| (503, vec![], "Service Unavailable".to_string())).await;

    let service =
        ElyAuthService::with_endpoints(reqwest::Client::new(), &mock.base_url, &mock.base_url);

    let err = service
        .fetch_catalog(1, None, None, None, None)
        .await
        .unwrap_err();

    match err {
        ElyError::HttpStatus(status) => assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE),
        other => panic!("Expected HttpStatus, got {other:?}"),
    }
}

#[tokio::test]
async fn test_wear_skin_hermetic_flow() {
    let mock = MockServer::start(|method, path, body| {
        if path == "/authorization/login" {
            return (
                302,
                vec![(
                    "Location",
                    "/oauth2/v1/ely?redirect_uri=/authorization/oauth&state=test",
                )],
                String::new(),
            );
        }
        if path == "/api/authentication/login" {
            let json = serde_json::json!({
                "success": true,
                "access_token": "mock_token_abc"
            });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }
        if path.starts_with("/api/oauth2/v1/complete") {
            let json = serde_json::json!({
                "success": true,
                "redirectUri": "/authorization/oauth?code=xyz"
            });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }
        if path.starts_with("/authorization/oauth") {
            return (200, vec![], "OK".to_string());
        }
        if path == "/api/legacy/users/skin" {
            assert_eq!(method, "PUT");
            let body_str = String::from_utf8_lossy(body);
            assert_eq!(body_str, "skinId=42");
            let json = serde_json::json!({ "success": true });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }

        (404, vec![], "Not Found".to_string())
    })
    .await;

    let service =
        ElyAuthService::with_endpoints(reqwest::Client::new(), &mock.base_url, &mock.base_url);

    let res = service.wear_skin("testuser", "correctpass", 42).await;
    assert!(res.is_ok(), "wear_skin should succeed, got {res:?}");
}

#[tokio::test]
async fn test_auth_wrong_password_returns_typed_error() {
    let mock = MockServer::start(|_, path, _| {
        if path == "/authorization/login" {
            return (
                302,
                vec![("Location", "/oauth2/v1/ely?redirect_uri=/cb")],
                String::new(),
            );
        }
        if path == "/api/authentication/login" {
            let json = serde_json::json!({
                "success": false,
                "errors": {
                    "password": "password_incorrect"
                }
            });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }
        (404, vec![], "Not Found".to_string())
    })
    .await;

    let service =
        ElyAuthService::with_endpoints(reqwest::Client::new(), &mock.base_url, &mock.base_url);

    let res = service.wear_skin("testuser", "wrongpass", 1).await;
    match res {
        Err(ElyError::IncorrectPassword) => {}
        other => panic!("Expected IncorrectPassword, got {other:?}"),
    }
}

#[tokio::test]
async fn test_auth_totp_required_returns_typed_error() {
    let mock = MockServer::start(|_, path, _| {
        if path == "/authorization/login" {
            return (
                302,
                vec![("Location", "/oauth2/v1/ely?redirect_uri=/cb")],
                String::new(),
            );
        }
        if path == "/api/authentication/login" {
            let json = serde_json::json!({
                "success": false,
                "errors": {
                    "totp": "totp_required"
                }
            });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }
        (404, vec![], "Not Found".to_string())
    })
    .await;

    let service =
        ElyAuthService::with_endpoints(reqwest::Client::new(), &mock.base_url, &mock.base_url);

    let res = service.wear_skin("testuser", "pass", 1).await;
    match res {
        Err(ElyError::TwoFactorRequired) => {}
        other => panic!("Expected TwoFactorRequired, got {other:?}"),
    }
}

#[tokio::test]
async fn test_upload_skin_hermetic_flow() {
    let mock = MockServer::start(|method, path, _| {
        if path == "/authorization/login" {
            return (
                302,
                vec![("Location", "/oauth2/v1/ely?redirect_uri=/cb")],
                String::new(),
            );
        }
        if path == "/api/authentication/login" {
            let json = serde_json::json!({
                "success": true,
                "access_token": "token_123"
            });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }
        if path.starts_with("/api/oauth2/v1/complete") {
            let json = serde_json::json!({
                "success": true,
                "redirectUri": "/cb"
            });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }
        if path == "/cb" {
            return (200, vec![], "OK".to_string());
        }
        if path == "/api/legacy/skins" {
            assert_eq!(method, "POST");
            let json = serde_json::json!({
                "success": true,
                "id": 8888
            });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }
        if path == "/api/legacy/users/skin" {
            assert_eq!(method, "PUT");
            let json = serde_json::json!({ "success": true });
            return (
                200,
                vec![("Content-Type", "application/json")],
                json.to_string(),
            );
        }

        (404, vec![], "Not Found".to_string())
    })
    .await;

    let service =
        ElyAuthService::with_endpoints(reqwest::Client::new(), &mock.base_url, &mock.base_url);

    let dummy_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let res = service.upload_skin("user", "pass", dummy_png).await;
    assert!(res.is_ok(), "upload_skin should succeed, got {res:?}");
}

#[tokio::test]
#[ignore = "requires the public Ely.by API; run explicitly as a live smoke test"]
async fn test_fetch_catalog_with_uploader() {
    let service = ElyAuthService::new();
    let res = service
        .fetch_catalog(
            1,
            None,
            Some("latest".into()),
            None,
            Some("ErickSkrauch".into()),
        )
        .await;
    assert!(
        res.is_ok(),
        "Expected fetch_catalog with uploader to succeed, got: {:?}",
        res.err()
    );
}
