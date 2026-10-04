use super::*;
use std::error::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

const AUTH: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/ely-session-authenticate.json"
));
const REFRESH: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/ely-session-refresh.json"
));

struct Request {
    headers: String,
    body: serde_json::Value,
}

async fn server(status: u16, body: &str) -> (ElySession, JoinHandle<Request>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let body = body.to_owned();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 2048];
        let (headers, offset, length) = loop {
            let count = stream.read(&mut buffer).await.unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(offset) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8(bytes[..offset].to_vec()).unwrap();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                break (headers, offset + 4, length);
            }
        };
        while bytes.len() < offset + length {
            let count = stream.read(&mut buffer).await.unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
        }
        let request = Request {
            headers,
            body: serde_json::from_slice(&bytes[offset..offset + length]).unwrap(),
        };
        let response = format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.shutdown().await.unwrap();
        request
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    (ElySession::new(client, format!("http://{address}")), task)
}

#[tokio::test]
async fn authentication_and_refresh_fixtures_preserve_request_and_response_fields() {
    let (provider, request) = server(200, AUTH).await;
    let response = provider
        .authenticate("FixturePlayer", "fixture-password", "fixture-client")
        .await
        .unwrap();
    assert_eq!(response.access_token, "fixture-access");
    assert_eq!(response.selected_profile.unwrap().name, "FixturePlayer");
    let request = request.await.unwrap();
    assert!(request
        .headers
        .starts_with("POST /auth/authenticate HTTP/1.1"));
    assert_eq!(
        request.body,
        serde_json::json!({ "username": "FixturePlayer", "password": "fixture-password", "clientToken": "fixture-client", "requestUser": true })
    );

    let (provider, request) = server(200, REFRESH).await;
    let response = provider
        .refresh("fixture-access", "fixture-client")
        .await
        .unwrap();
    assert_eq!(response.access_token, "rotated-access");
    assert_eq!(response.client_token, "fixture-client");
    let request = request.await.unwrap();
    assert!(request.headers.starts_with("POST /auth/refresh HTTP/1.1"));
    assert_eq!(
        request.body,
        serde_json::json!({ "accessToken": "fixture-access", "clientToken": "fixture-client", "requestUser": true })
    );
}

#[tokio::test]
async fn validation_distinguishes_invalid_tokens_from_provider_outages() {
    for status in [200, 204, 400, 401, 403, 429, 503] {
        let (provider, request) = server(status, "").await;
        let result = provider.validate("fixture-access", "fixture-client").await;
        match status {
            200 | 204 => assert_eq!(result.unwrap(), TokenValidity::Valid),
            400 | 401 | 403 => assert_eq!(result.unwrap(), TokenValidity::Invalid),
            _ => assert!(
                matches!(result, Err(ProviderError::Rejected { status: code, operation: ProviderOperation::ElyValidate }) if code.as_u16() == status)
            ),
        }
        let request = request.await.unwrap();
        assert!(request.headers.starts_with("POST /auth/validate HTTP/1.1"));
        assert_eq!(
            request.body,
            serde_json::json!({ "accessToken": "fixture-access", "clientToken": "fixture-client" })
        );
    }
}

#[tokio::test]
async fn malformed_success_and_rejected_responses_are_typed_and_do_not_echo_provider_bodies() {
    let (provider, request) =
        server(200, r#"{"accessToken":"private-token","clientToken":[]}"#).await;
    let error = provider.refresh("old", "client").await.err().unwrap();
    assert!(error.source().is_some());
    assert!(!error.to_string().contains("private-token"));
    assert!(matches!(
        error,
        ProviderError::Response {
            operation: ProviderOperation::ElyRefresh,
            ..
        }
    ));
    request.await.unwrap();
    for status in [401, 403, 429, 503] {
        let (provider, request) = server(
            status,
            r#"{"error":"private-password","errorMessage":"private-token"}"#,
        )
        .await;
        let error = provider
            .authenticate("name", "password", "client")
            .await
            .err()
            .unwrap();
        assert!(!error.to_string().contains("private-"));
        assert!(
            matches!(error, ProviderError::Rejected { status: code, .. } if code.as_u16() == status)
        );
        request.await.unwrap();
    }
}

#[tokio::test]
async fn two_factor_authentication_remains_actionable_without_arbitrary_server_text() {
    let (provider, request) = server(401, r#"{"error":"ForbiddenOperationException","errorMessage":"Account protected with two factor auth."}"#).await;
    assert!(matches!(
        provider.authenticate("name", "password", "client").await,
        Err(ProviderError::TwoFactorRequired)
    ));
    request.await.unwrap();
    let (provider, request) = server(401, "not-json private-token").await;
    assert!(matches!(
        provider.authenticate("name", "password", "client").await,
        Err(ProviderError::Rejected { .. })
    ));
    request.await.unwrap();
}

#[tokio::test]
async fn disconnected_provider_is_a_transport_failure_not_an_invalid_token() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let disconnect = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        drop(stream);
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let provider = ElySession::new(client, format!("http://{address}"));
    let error = provider
        .validate("private-token", "client")
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ProviderError::Transport {
            operation: ProviderOperation::ElyValidate,
            ..
        }
    ));
    assert!(error.source().is_some());
    assert!(!error.to_string().contains("private-token"));
    disconnect.await.unwrap();
}
