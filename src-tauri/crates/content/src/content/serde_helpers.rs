//! Resilient serde deserializers shared across content API clients and modpack parsers.
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

#[derive(Deserialize)]
#[serde(untagged)]
enum IntOrString {
    Int(u64),
    String(String),
}

fn parse_u64_value<T, E>(value: IntOrString) -> Result<T, E>
where
    T: TryFrom<u64> + FromStr,
    <T as TryFrom<u64>>::Error: std::fmt::Display,
    <T as FromStr>::Err: std::fmt::Display,
    E: serde::de::Error,
{
    match value {
        IntOrString::Int(n) => T::try_from(n).map_err(E::custom),
        IntOrString::String(s) => s.trim().parse::<T>().map_err(E::custom),
    }
}

fn parse_opt_u64_value<T, E>(value: Option<IntOrString>) -> Result<Option<T>, E>
where
    T: TryFrom<u64> + FromStr,
    <T as TryFrom<u64>>::Error: std::fmt::Display,
    <T as FromStr>::Err: std::fmt::Display,
    E: serde::de::Error,
{
    match value {
        None => Ok(None),
        Some(IntOrString::String(s)) if s.trim().is_empty() => Ok(None),
        Some(other) => parse_u64_value::<T, E>(other).map(Some),
    }
}

pub(crate) fn deserialize_u32_or_string<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    parse_u64_value::<u32, D::Error>(IntOrString::deserialize(deserializer)?)
}

pub(crate) fn deserialize_opt_u32_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    parse_opt_u64_value::<u32, D::Error>(Option::<IntOrString>::deserialize(deserializer)?)
}

pub(crate) fn deserialize_u64_or_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    parse_u64_value::<u64, D::Error>(IntOrString::deserialize(deserializer)?)
}

pub(crate) fn deserialize_opt_u64_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    parse_opt_u64_value::<u64, D::Error>(Option::<IntOrString>::deserialize(deserializer)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Sample {
        #[serde(deserialize_with = "deserialize_u32_or_string")]
        req_u32: u32,
        #[serde(default, deserialize_with = "deserialize_opt_u32_or_string")]
        opt_u32: Option<u32>,
        #[serde(deserialize_with = "deserialize_u64_or_string")]
        req_u64: u64,
        #[serde(default, deserialize_with = "deserialize_opt_u64_or_string")]
        opt_u64: Option<u64>,
    }

    #[test]
    fn parses_integers_and_trimmed_strings() {
        let parsed: Sample = serde_json::from_str(
            r#"{
                "req_u32": " 42 ",
                "opt_u32": 7,
                "req_u64": 18446744073709551615,
                "opt_u64": "9001"
            }"#,
        )
        .unwrap();
        assert_eq!(
            parsed,
            Sample {
                req_u32: 42,
                opt_u32: Some(7),
                req_u64: u64::MAX,
                opt_u64: Some(9001),
            }
        );
    }

    #[test]
    fn treats_null_missing_and_blank_strings_as_none_for_optional_fields() {
        let parsed: Sample = serde_json::from_str(
            r#"{
                "req_u32": 1,
                "opt_u32": "   ",
                "req_u64": "2"
            }"#,
        )
        .unwrap();
        assert_eq!(parsed.opt_u32, None);
        assert_eq!(parsed.opt_u64, None);

        let with_nulls: Sample = serde_json::from_str(
            r#"{
                "req_u32": 1,
                "opt_u32": null,
                "req_u64": 2,
                "opt_u64": ""
            }"#,
        )
        .unwrap();
        assert_eq!(with_nulls.opt_u32, None);
        assert_eq!(with_nulls.opt_u64, None);
    }

    #[test]
    fn rejects_invalid_null_required_and_overflow_values() {
        let overflow_u32 =
            serde_json::from_str::<Sample>(r#"{"req_u32": 4294967296, "req_u64": 1}"#);
        assert!(overflow_u32.is_err());

        let overflow_opt_u32 = serde_json::from_str::<Sample>(
            r#"{"req_u32": 1, "opt_u32": "4294967296", "req_u64": 1}"#,
        );
        assert!(overflow_opt_u32.is_err());

        let non_numeric = serde_json::from_str::<Sample>(r#"{"req_u32": 1, "req_u64": "abc"}"#);
        assert!(non_numeric.is_err());

        let null_required = serde_json::from_str::<Sample>(r#"{"req_u32": null, "req_u64": 1}"#);
        assert!(null_required.is_err());
    }
}
