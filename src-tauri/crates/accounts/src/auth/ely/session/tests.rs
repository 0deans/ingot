use super::*;
use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
use std::error::Error;

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

async fn server(status: u16, body: &str) -> (ElySession, LoopbackServer) {
    let fixture = LoopbackServer::exact_script(vec![ScriptedExchange::any_path(
        ResponseSpec::json(status, body.as_bytes().to_vec()),
    )])
    .await;
    let client = Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    (
        ElySession::new(client, fixture.base_url().to_string()),
        fixture,
    )
}

async fn single_request(server: LoopbackServer) -> Request {
    let mut captured = server.finish().await;
    assert_eq!(captured.len(), 1);
    let request = captured.remove(0);
    Request {
        headers: request.raw_headers,
        body: serde_json::from_slice(&request.body).unwrap(),
    }
}

#[tokio::test]
async fn authentication_and_refresh_fixtures_preserve_request_and_response_fields() {
    let (provider, server_handle) = server(200, AUTH).await;
    let response = provider
        .authenticate("FixturePlayer", "fixture-password", "fixture-client")
        .await
        .unwrap();
    assert_eq!(response.access_token, "fixture-access");
    assert_eq!(response.selected_profile.unwrap().name, "FixturePlayer");
    let request = single_request(server_handle).await;
    assert!(request
        .headers
        .starts_with("POST /auth/authenticate HTTP/1.1"));
    assert_eq!(
        request.body,
        serde_json::json!({ "username": "FixturePlayer", "password": "fixture-password", "clientToken": "fixture-client", "requestUser": true })
    );

    let (provider, server_handle) = server(200, REFRESH).await;
    let response = provider
        .refresh("fixture-access", "fixture-client")
        .await
        .unwrap();
    assert_eq!(response.access_token, "rotated-access");
    assert_eq!(response.client_token, "fixture-client");
    let request = single_request(server_handle).await;
    assert!(request.headers.starts_with("POST /auth/refresh HTTP/1.1"));
    assert_eq!(
        request.body,
        serde_json::json!({ "accessToken": "fixture-access", "clientToken": "fixture-client", "requestUser": true })
    );
}

#[tokio::test]
async fn validation_distinguishes_invalid_tokens_from_provider_outages() {
    for status in [200, 204, 400, 401, 403, 429, 503] {
        let (provider, server_handle) = server(status, "").await;
        let result = provider.validate("fixture-access", "fixture-client").await;
        match status {
            200 | 204 => assert_eq!(result.unwrap(), TokenValidity::Valid),
            400 | 401 | 403 => assert_eq!(result.unwrap(), TokenValidity::Invalid),
            _ => assert!(
                matches!(result, Err(ProviderError::Rejected { status: code, operation: ProviderOperation::ElyValidate }) if code.as_u16() == status)
            ),
        }
        let request = single_request(server_handle).await;
        assert!(request.headers.starts_with("POST /auth/validate HTTP/1.1"));
        assert_eq!(
            request.body,
            serde_json::json!({ "accessToken": "fixture-access", "clientToken": "fixture-client" })
        );
    }
}

#[tokio::test]
async fn malformed_success_and_rejected_responses_are_typed_and_do_not_echo_provider_bodies() {
    let (provider, server_handle) =
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
    let _ = single_request(server_handle).await;
    for status in [401, 403, 429, 503] {
        let (provider, server_handle) = server(
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
        let _ = single_request(server_handle).await;
    }
}

#[tokio::test]
async fn two_factor_authentication_remains_actionable_without_arbitrary_server_text() {
    let (provider, server_handle) = server(401, r#"{"error":"ForbiddenOperationException","errorMessage":"Account protected with two factor auth."}"#).await;
    assert!(matches!(
        provider.authenticate("name", "password", "client").await,
        Err(ProviderError::TwoFactorRequired)
    ));
    let _ = single_request(server_handle).await;
    let (provider, server_handle) = server(401, "not-json private-token").await;
    assert!(matches!(
        provider.authenticate("name", "password", "client").await,
        Err(ProviderError::Rejected { .. })
    ));
    let _ = single_request(server_handle).await;
}

#[tokio::test]
async fn disconnected_provider_is_a_transport_failure_not_an_invalid_token() {
    let fixture = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
        "/auth/validate",
        ResponseSpec::close_connection(),
    )])
    .await;
    let client = Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let provider = ElySession::new(client, fixture.base_url().to_string());
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
    let _ = fixture.finish().await;
}
