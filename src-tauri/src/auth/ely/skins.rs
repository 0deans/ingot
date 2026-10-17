//! Ely.by skin upload and wear operations.
use super::error::ElyError;
use reqwest::{multipart, Client};

pub(crate) fn extract_skin_id(json: &serde_json::Value) -> Option<u64> {
    if let Some(id) = json.get("id").and_then(|v| v.as_u64()) {
        return Some(id);
    }
    if let Some(id_str) = json.get("id").and_then(|v| v.as_str()) {
        if let Ok(id) = id_str.parse::<u64>() {
            return Some(id);
        }
    }
    if let Some(id) = json.get("skinId").and_then(|v| v.as_u64()) {
        return Some(id);
    }
    if let Some(url) = json.get("url").and_then(|v| v.as_str()) {
        if let Some(pos) = url.find("/skins/s") {
            let rest = &url[pos + 8..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u64>() {
                return Some(id);
            }
        }
        if let Some(pos) = url.find("/skins/") {
            let rest = &url[pos + 7..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse::<u64>() {
                return Some(id);
            }
        }
    }
    if let Some(extra) = json.get("extra") {
        if let Some(id) = extra.get("id").and_then(|v| v.as_u64()) {
            return Some(id);
        }
        if let Some(id) = extra.get("skinId").and_then(|v| v.as_u64()) {
            return Some(id);
        }
    }
    None
}

pub(crate) fn check_ely_web_response(json: &serde_json::Value) -> Result<(), ElyError> {
    let rejected = json
        .get("error")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|error| !error.is_empty() && !error.contains("success"));
    if rejected || json.get("success").and_then(serde_json::Value::as_bool) == Some(false) {
        return Err(ElyError::ApiRejected);
    }
    Ok(())
}
pub(crate) async fn wear_skin(
    client: &Client,
    ely_base: &str,
    skin_id: u64,
) -> Result<(), ElyError> {
    let res = client
        .put(format!("{ely_base}/api/legacy/users/skin"))
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("skinId={skin_id}"))
        .send()
        .await
        .map_err(|error| ElyError::Transport(error.without_url()))?;

    if !res.status().is_success() {
        return Err(ElyError::HttpStatus(res.status()));
    }

    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(|error| ElyError::ParseResponse(error.without_url()))?;

    check_ely_web_response(&json)?;
    Ok(())
}

pub(crate) async fn upload_skin(
    client: &Client,
    ely_base: &str,
    png_bytes: Vec<u8>,
) -> Result<(), ElyError> {
    let part = multipart::Part::bytes(png_bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|error| ElyError::Multipart(error.without_url()))?;

    let form = multipart::Form::new().part("file", part);

    let res = client
        .post(format!("{ely_base}/api/legacy/skins"))
        .header("X-Requested-With", "XMLHttpRequest")
        .multipart(form)
        .send()
        .await
        .map_err(|error| ElyError::Transport(error.without_url()))?;

    if !res.status().is_success() {
        return Err(ElyError::HttpStatus(res.status()));
    }

    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(|error| ElyError::ParseResponse(error.without_url()))?;

    check_ely_web_response(&json)?;

    // If Ely.by returns a skin ID, automatically wear the uploaded skin
    if let Some(skin_id) = extract_skin_id(&json) {
        wear_skin(client, ely_base, skin_id)
            .await
            .map_err(|source| ElyError::UploadedNotApplied {
                skin_id,
                source: Box::new(source),
            })?;
    }

    Ok(())
}
