use super::{api::decode_response, ClaimRejection, PlayitError};
use serde_json::json;

#[test]
fn provider_rejections_do_not_expose_secret_payloads() {
    let error = decode_response(json!({
        "status": "fail",
        "data": { "secret_key": "private-fixture-token", "message": "private-fixture-token" }
    }))
    .unwrap_err();
    assert!(matches!(
        error,
        PlayitError::Rejected(ClaimRejection::Other)
    ));
    assert!(!error.to_string().contains("private-fixture-token"));
    assert!(!format!("{error:?}").contains("private-fixture-token"));
}

#[test]
fn malformed_claim_exchange_data_does_not_expose_credentials() {
    let error =
        decode_response(json!({ "data": { "secret_key": "private-fixture-token" } })).unwrap_err();
    assert!(matches!(error, PlayitError::InvalidResponse));
    assert!(!format!("{error:?}").contains("private-fixture-token"));
}

#[test]
fn known_claim_rejections_preserve_retry_categories() {
    for (code, expected) in [
        ("CodeExpired", "expired"),
        ("InvalidCode", "invalid"),
        ("NotAccepted", "pending"),
    ] {
        let error =
            decode_response(json!({ "status": "fail", "data": { "type": code } })).unwrap_err();
        let category = match error {
            PlayitError::Rejected(ClaimRejection::Expired) => "expired",
            PlayitError::Rejected(ClaimRejection::InvalidCode) => "invalid",
            PlayitError::Rejected(ClaimRejection::NotAccepted) => "pending",
            _ => panic!("unexpected claim rejection category"),
        };
        assert_eq!(category, expected);
    }
}

#[test]
fn successful_envelope_requires_data() {
    assert!(matches!(
        decode_response(json!({ "status": "success" })),
        Err(PlayitError::InvalidResponse)
    ));
    assert_eq!(
        decode_response(json!({ "status": "success", "data": "UserAccepted" })).unwrap(),
        json!("UserAccepted")
    );
}
