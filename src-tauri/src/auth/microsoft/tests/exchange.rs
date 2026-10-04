use super::{
    super::{error::Stage, MicrosoftError},
    support::*,
};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[tokio::test]
async fn refresh_grant_is_available_without_any_dependent_exchange_request() {
    let server = Server::new(vec![reply("/token", 200, TOKEN)]).await;
    let session = server.session(
        Arc::new(Clock::default()),
        Some("fixture-client".to_owned()),
    );
    let grant = session.refresh_grant("fixture-refresh").await.unwrap();
    assert_eq!(grant.refresh_token(), "fixture-rotated-refresh");
    assert!(!format!("{grant:?}").contains("fixture-rotated-refresh"));
    assert!(!format!("{grant:?}").contains("fixture-ms-access"));
    assert_eq!(server.requests().await.len(), 1);
}

#[tokio::test]
async fn refresh_grant_rejects_missing_or_empty_tokens_before_exchange() {
    for body in [
        r#"{"access_token":"secret-access"}"#,
        r#"{"access_token":"", "refresh_token":"secret-refresh"}"#,
        r#"{"access_token":"secret-access", "refresh_token":" "}"#,
    ] {
        let server = Server::new(vec![reply("/token", 200, body)]).await;
        let session = server.session(
            Arc::new(Clock::default()),
            Some("fixture-client".to_owned()),
        );
        let error = session.refresh_grant("fixture-refresh").await.unwrap_err();
        assert!(matches!(
            error,
            MicrosoftError::Response {
                stage: Stage::Refresh,
                ..
            } | MicrosoftError::Protocol {
                stage: Stage::Refresh,
                ..
            }
        ));
        assert!(!error.to_string().contains("secret-access"));
        assert!(!error.to_string().contains("secret-refresh"));
        assert_eq!(server.requests().await.len(), 1);
    }
}

#[tokio::test]
async fn refresh_fixture_exchanges_tokens_and_preserves_bearer_payloads_profile_and_expiry() {
    let clock = Arc::new(Clock::default());
    let mut script = chain();
    let delayed = clock.clone();
    script[4].effect = Some(Box::new(move || delayed.advance(Duration::from_secs(60))));
    let server = Server::new(script).await;
    let session = server.session(clock, Some("fixture-client".to_owned()));
    let result = session.refresh("fixture-refresh").await.unwrap();
    assert_eq!(result.access_token, "fixture-minecraft-access");
    assert_eq!(result.refresh_token, "fixture-rotated-refresh");
    assert_eq!(result.uuid, "fixture-id");
    assert_eq!(result.username, "FixturePlayer");
    assert_eq!(
        result.skin_url.as_deref(),
        Some("https://fixture.invalid/active")
    );
    assert_eq!(result.expires_at, 4600);
    let requests = server.requests().await;
    assert!(requests[0].body.contains("refresh_token=fixture-refresh"));
    let xbox: serde_json::Value = serde_json::from_str(&requests[1].body).unwrap();
    assert_eq!(xbox["Properties"]["RpsTicket"], "d=fixture-ms-access");
    let xsts: serde_json::Value = serde_json::from_str(&requests[2].body).unwrap();
    assert_eq!(xsts["Properties"]["UserTokens"][0], "fixture-xbox-token");
    let minecraft: serde_json::Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(
        minecraft["identityToken"],
        "XBL3.0 x=fixture-user-hash;fixture-xbox-token"
    );
    assert!(requests[4]
        .headers
        .to_lowercase()
        .contains("authorization: bearer fixture-minecraft-access"));
}

#[tokio::test]
async fn transport_and_malformed_success_are_distinct_at_each_exchange_stage() {
    for (index, expected) in [
        (0, Stage::Refresh),
        (1, Stage::XboxLive),
        (2, Stage::Xsts),
        (3, Stage::MinecraftLogin),
        (4, Stage::Profile),
    ] {
        for disconnected in [true, false] {
            let mut script = chain();
            script.truncate(index + 1);
            script[index].status = if disconnected { 0 } else { 200 };
            script[index].body = "private-token invalid-json".to_owned();
            let server = Server::new(script).await;
            let session = server.session(
                Arc::new(Clock::default()),
                Some("fixture-client".to_owned()),
            );
            let error = session.refresh("fixture-refresh").await.unwrap_err();
            if disconnected {
                assert!(
                    matches!(error, MicrosoftError::Transport { stage, .. } if stage == expected)
                );
            } else {
                assert!(
                    matches!(error, MicrosoftError::Response { stage, .. } if stage == expected)
                );
            }
            assert!(error.source().is_some());
            assert!(!error.to_string().contains("private-token"));
            server.requests().await;
        }
    }
}

#[tokio::test]
async fn xsts_reasons_app_registration_and_missing_minecraft_profile_keep_actionable_categories() {
    for (index, status, body) in [
        (2, 401, r#"{"XErr":2148916233}"#),
        (2, 401, r#"{"XErr":2148916235}"#),
        (2, 401, r#"{"XErr":2148916236}"#),
        (2, 401, r#"{"XErr":2148916237}"#),
        (2, 401, r#"{"XErr":2148916238}"#),
        (2, 401, r#"{"XErr":123}"#),
        (3, 403, "private-token"),
        (4, 404, "private-token"),
    ] {
        let mut script = chain();
        script.truncate(index + 1);
        script[index].status = status;
        script[index].body = body.to_owned();
        let server = Server::new(script).await;
        let session = server.session(
            Arc::new(Clock::default()),
            Some("fixture-client".to_owned()),
        );
        let error = session.refresh("fixture-refresh").await.unwrap_err();
        match body {
            r#"{"XErr":2148916233}"# => {
                assert!(matches!(error, MicrosoftError::MissingXboxProfile))
            }
            r#"{"XErr":2148916235}"# => assert!(matches!(error, MicrosoftError::XboxRegion)),
            r#"{"XErr":2148916236}"# | r#"{"XErr":2148916237}"# => {
                assert!(matches!(error, MicrosoftError::AdultVerification))
            }
            r#"{"XErr":2148916238}"# => assert!(matches!(error, MicrosoftError::ChildAccount)),
            r#"{"XErr":123}"# => {
                assert!(matches!(error, MicrosoftError::XstsRejected { code: 123 }))
            }
            _ if index == 3 => assert!(matches!(error, MicrosoftError::AppRegistration)),
            _ => assert!(matches!(error, MicrosoftError::MissingMinecraftProfile)),
        }
        server.requests().await;
    }
}

#[tokio::test]
async fn missing_xbox_claims_and_empty_tokens_are_rejected_before_dependent_requests() {
    for (index, body, expected) in [
        (
            0,
            r#"{"access_token":"","refresh_token":"fixture"}"#,
            Stage::Refresh,
        ),
        (
            1,
            r#"{"Token":"","DisplayClaims":{"xui":[]}}"#,
            Stage::XboxLive,
        ),
        (
            2,
            r#"{"Token":"fixture","DisplayClaims":{"xui":[]}}"#,
            Stage::Xsts,
        ),
        (
            3,
            r#"{"access_token":"","expires_in":10}"#,
            Stage::MinecraftLogin,
        ),
        (4, r#"{"id":"","name":"Fixture"}"#, Stage::Profile),
    ] {
        let mut script = chain();
        script.truncate(index + 1);
        script[index].body = body.to_owned();
        let server = Server::new(script).await;
        let session = server.session(
            Arc::new(Clock::default()),
            Some("fixture-client".to_owned()),
        );
        assert!(
            matches!(session.refresh("fixture-refresh").await, Err(MicrosoftError::Protocol { stage, .. }) if stage == expected)
        );
        server.requests().await;
    }
}

#[tokio::test]
async fn cancellation_after_xbox_response_stops_the_remaining_exchange() {
    let flag = Arc::new(AtomicBool::new(false));
    let changed = flag.clone();
    let mut script = chain();
    script.truncate(2);
    script[1].effect = Some(Box::new(move || changed.store(true, Ordering::SeqCst)));
    let server = Server::new(script).await;
    let session = server.session(
        Arc::new(Clock::default()),
        Some("fixture-client".to_owned()),
    );
    assert!(matches!(
        session
            .finish(&code(), &|| flag.load(Ordering::SeqCst))
            .await,
        Err(MicrosoftError::Cancelled)
    ));
    assert_eq!(server.requests().await.len(), 2);
}

#[tokio::test]
async fn rejected_http_statuses_retain_exchange_stage_and_hide_response_bodies() {
    for (index, expected) in [
        (0, Stage::Refresh),
        (1, Stage::XboxLive),
        (2, Stage::Xsts),
        (3, Stage::MinecraftLogin),
        (4, Stage::Profile),
    ] {
        let mut script = chain();
        script.truncate(index + 1);
        script[index].status = 503;
        script[index].body = "private-token".to_owned();
        let server = Server::new(script).await;
        let session = server.session(
            Arc::new(Clock::default()),
            Some("fixture-client".to_owned()),
        );
        let error = session.refresh("fixture-refresh").await.unwrap_err();
        assert!(
            matches!(error, MicrosoftError::Rejected { stage, status } if stage == expected && status.as_u16() == 503)
        );
        assert!(!error.to_string().contains("private-token"));
        server.requests().await;
    }
}

#[tokio::test]
async fn large_token_lifetime_saturates_and_a_profile_without_skins_stays_compatible() {
    let mut script = chain();
    script[3].body =
        serde_json::json!({"access_token":"fixture-minecraft-access","expires_in":u64::MAX})
            .to_string();
    script[4].body = r#"{"id":"fixture-id","name":"FixturePlayer"}"#.to_owned();
    let server = Server::new(script).await;
    let session = server.session(
        Arc::new(Clock::default()),
        Some("fixture-client".to_owned()),
    );
    let result = session.refresh("fixture-refresh").await.unwrap();
    assert_eq!(result.expires_at, u64::MAX);
    assert!(result.skin_url.is_none());
    server.requests().await;
}
