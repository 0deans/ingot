use super::{
    super::{error::Stage, MicrosoftError},
    support::*,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[tokio::test]
async fn device_code_fixture_and_request_keep_the_existing_contract() {
    let server = Server::new(vec![reply("/device", 200, DEVICE)]).await;
    let session = server.session(
        Arc::new(Clock::default()),
        Some("fixture-client".to_owned()),
    );
    let code = session.start().await.unwrap();
    assert_eq!(code.user_code, "FIXTURE");
    assert_eq!(code.interval, 2);
    assert_eq!(code.expires_in, 900);
    let requests = server.requests().await;
    assert!(requests[0].body.contains("client_id=fixture-client"));
    assert!(requests[0]
        .body
        .contains("scope=XboxLive.signin+offline_access"));
}

#[tokio::test]
async fn pending_and_slow_down_poll_with_controlled_time_then_exchange_the_session() {
    let mut script = vec![
        reply("/token", 400, r#"{"error":"authorization_pending"}"#),
        reply("/token", 400, r#"{"error":"slow_down"}"#),
    ];
    script.extend(chain());
    let server = Server::new(script).await;
    let clock = Arc::new(Clock::default());
    let session = server.session(clock.clone(), Some("fixture-client".to_owned()));
    let result = session.finish(&code(), &|| false).await.unwrap();
    assert_eq!(result.refresh_token, "fixture-rotated-refresh");
    assert_eq!(
        clock.waits(),
        [
            Duration::from_secs(2),
            Duration::from_secs(2),
            Duration::from_secs(7)
        ]
    );
    let requests = server.requests().await;
    assert_eq!(requests.len(), 7);
    assert!(requests[0]
        .body
        .contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"));
    assert!(requests[0].body.contains("device_code=fixture-device"));
}

#[tokio::test]
async fn oauth_terminal_errors_are_typed_and_raw_descriptions_are_not_exposed() {
    for name in [
        "authorization_declined",
        "expired_token",
        "bad_verification_code",
        "unknown-private-token",
    ] {
        let body = serde_json::json!({"error":name,"error_description":"fixture-private-password"})
            .to_string();
        let server = Server::new(vec![reply("/token", 400, &body)]).await;
        let session = server.session(
            Arc::new(Clock::default()),
            Some("fixture-client".to_owned()),
        );
        let error = session.finish(&code(), &|| false).await.unwrap_err();
        match name {
            "authorization_declined" => assert!(matches!(error, MicrosoftError::Declined)),
            "expired_token" => assert!(matches!(error, MicrosoftError::CodeExpired)),
            "bad_verification_code" => assert!(matches!(error, MicrosoftError::InvalidCode)),
            _ => assert!(matches!(
                error,
                MicrosoftError::Rejected {
                    stage: Stage::DeviceToken,
                    ..
                }
            )),
        }
        assert!(!error.to_string().contains("private-"));
        server.requests().await;
    }
}

#[tokio::test]
async fn expired_cancelled_and_unconfigured_attempts_make_no_provider_requests() {
    let server = Server::new(vec![]).await;
    let clock = Arc::new(Clock::default());
    let session = server.session(clock.clone(), None);
    assert!(matches!(
        session.start().await,
        Err(MicrosoftError::NotConfigured)
    ));
    let session = server.session(clock.clone(), Some("fixture-client".to_owned()));
    assert!(matches!(
        session.finish(&code(), &|| true).await,
        Err(MicrosoftError::Cancelled)
    ));
    let mut code = code();
    code.expires_in = 1;
    code.interval = 0;
    assert!(matches!(
        session.finish(&code, &|| false).await,
        Err(MicrosoftError::CodeExpired)
    ));
    assert_eq!(clock.waits(), [Duration::from_secs(1)]);
    assert!(server.requests().await.is_empty());
}

#[tokio::test]
async fn cancellation_during_poll_delay_stops_before_requesting_tokens() {
    let server = Server::new(vec![]).await;
    let clock = Arc::new(Clock::default());
    let cancelled = Arc::new(AtomicBool::new(false));
    *clock.cancel_on_sleep.lock().unwrap() = Some(cancelled.clone());
    let session = server.session(clock, Some("fixture-client".to_owned()));
    assert!(matches!(
        session
            .finish(&code(), &|| cancelled.load(Ordering::SeqCst))
            .await,
        Err(MicrosoftError::Cancelled)
    ));
    assert!(server.requests().await.is_empty());
}

#[tokio::test]
async fn refresh_rejection_distinguishes_invalid_grant_from_outage() {
    for (status, body) in [
        (
            400,
            r#"{"error":"invalid_grant","error_description":"private-token"}"#,
        ),
        (503, r#"{"error":"authorization_pending"}"#),
        (429, "private-token"),
    ] {
        let server = Server::new(vec![reply("/token", status, body)]).await;
        let session = server.session(
            Arc::new(Clock::default()),
            Some("fixture-client".to_owned()),
        );
        let error = session.refresh("fixture-refresh").await.unwrap_err();
        if status == 400 {
            assert!(matches!(error, MicrosoftError::RefreshExpired));
        } else {
            assert!(matches!(
                error,
                MicrosoftError::Rejected {
                    stage: Stage::Refresh,
                    ..
                }
            ));
        }
        assert!(!error.to_string().contains("private-token"));
        server.requests().await;
    }
}

#[tokio::test]
async fn unexpected_device_responses_and_poll_outages_stop_without_echoing_server_text() {
    for start in [true, false] {
        for status in [200, 429, 503] {
            let path = if start { "/device" } else { "/token" };
            let server = Server::new(vec![reply(
                path,
                status,
                r#"{"error":"authorization_pending","error_description":"private-token"}"#,
            )])
            .await;
            let session = server.session(
                Arc::new(Clock::default()),
                Some("fixture-client".to_owned()),
            );
            let error = if start {
                session.start().await.err().unwrap()
            } else {
                session.finish(&code(), &|| false).await.unwrap_err()
            };
            let expected = if start {
                Stage::DeviceCode
            } else {
                Stage::DeviceToken
            };
            if status == 200 {
                assert!(
                    matches!(error, MicrosoftError::Response { stage, .. } if stage == expected)
                );
            } else {
                assert!(
                    matches!(error, MicrosoftError::Rejected { stage, .. } if stage == expected)
                );
            }
            assert!(!error.to_string().contains("private-token"));
            assert_eq!(server.requests().await.len(), 1);
        }
    }
}

#[tokio::test]
async fn zero_poll_interval_is_clamped_and_expiry_stops_without_an_extra_request() {
    let server = Server::new(
        (0..3)
            .map(|_| reply("/token", 400, r#"{"error":"authorization_pending"}"#))
            .collect(),
    )
    .await;
    let clock = Arc::new(Clock::default());
    let session = server.session(clock.clone(), Some("fixture-client".to_owned()));
    let mut code = code();
    code.interval = 0;
    code.expires_in = 4;
    assert!(matches!(
        session.finish(&code, &|| false).await,
        Err(MicrosoftError::CodeExpired)
    ));
    assert_eq!(clock.waits(), [Duration::from_secs(1); 4]);
    assert_eq!(server.requests().await.len(), 3);
}
#[tokio::test]
async fn device_redemption_returns_a_grant_before_any_exchange_request() {
    let server = Server::new(vec![reply("/token", 200, TOKEN)]).await;
    let session = server.session(
        Arc::new(Clock::default()),
        Some("fixture-client".to_owned()),
    );
    let grant = session.redeem(&code(), &|| false).await.unwrap();
    assert_eq!(grant.refresh_token(), "fixture-rotated-refresh");
    assert_eq!(server.requests().await.len(), 1);
}
