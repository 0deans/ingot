use super::*;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_RANGE};
use std::error::Error;

fn validate(value: &str) -> Result<(), DownloadError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, value.parse().unwrap());
    validate_content_range(&headers, 4, 7, 10)
}

#[test]
fn missing_header_has_a_distinct_protocol_error() {
    let error = validate_content_range(&HeaderMap::new(), 4, 7, 10).unwrap_err();
    assert!(matches!(
        error,
        DownloadError::ContentRange(ContentRangeError::MissingHeader)
    ));
}

#[test]
fn non_ascii_header_retains_the_parser_cause_without_raw_bytes() {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_RANGE,
        HeaderValue::from_bytes(b"bytes \xffsecret-header").unwrap(),
    );
    let error = validate_content_range(&headers, 4, 7, 10).unwrap_err();
    assert!(matches!(
        error,
        DownloadError::ContentRange(ContentRangeError::NonAscii(_))
    ));
    let cause = error.source().unwrap().source().unwrap();
    assert!(cause.is::<reqwest::header::ToStrError>());
    assert!(!format!("{error:?}").contains("secret-header"));
}

#[test]
fn malformed_headers_have_finite_reasons_and_do_not_echo_provider_data() {
    for (header, reason) in [
        ("secret_header 4-7/10", ContentRangeError::UnsupportedUnit),
        (
            "bytes secret_header",
            ContentRangeError::MissingTotalSeparator,
        ),
        (
            "bytes secret_header/10",
            ContentRangeError::MissingEndSeparator,
        ),
    ] {
        let error = validate(header).unwrap_err();
        let DownloadError::ContentRange(ref actual) = error else {
            panic!("Unexpected error");
        };
        assert_eq!(
            std::mem::discriminant(actual),
            std::mem::discriminant(&reason)
        );
        assert!(!format!("{error:?}").contains("secret_header"));
        assert!(!error.to_string().contains("secret_header"));
    }
}

#[test]
fn invalid_integer_fields_retain_field_identity_and_parse_cause() {
    for (header, expected) in [
        ("bytes secret-header-7/10", ContentRangeField::Start),
        ("bytes 4-secret-header/10", ContentRangeField::End),
        ("bytes 4-7/secret-header", ContentRangeField::Total),
        ("bytes 4-7/18446744073709551616", ContentRangeField::Total),
    ] {
        let error = validate(header).unwrap_err();
        match &error {
            DownloadError::ContentRange(ContentRangeError::InvalidNumber { field, .. }) => {
                assert_eq!(*field, expected)
            }
            _ => panic!("Expected numeric field error"),
        }
        assert!(error
            .source()
            .unwrap()
            .source()
            .unwrap()
            .is::<std::num::ParseIntError>());
        assert!(!format!("{error:?}").contains("secret-header"));
    }
}

#[test]
fn mismatches_keep_requested_and_received_offsets() {
    assert!(matches!(
        validate("bytes 5-8/10"),
        Err(DownloadError::ContentRange(
            ContentRangeError::RangeMismatch {
                expected_start: 4,
                expected_end: 7,
                actual_start: 5,
                actual_end: 8,
            }
        ))
    ));
    assert!(matches!(
        validate("bytes 4-7/11"),
        Err(DownloadError::ContentRange(
            ContentRangeError::TotalMismatch {
                expected: 10,
                actual: 11
            }
        ))
    ));
}

#[test]
fn matching_range_accepts_known_and_unspecified_totals() {
    validate("bytes 4-7/10").unwrap();
    validate("bytes 4-7/*").unwrap();
}
