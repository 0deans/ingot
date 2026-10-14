//! Ely.by skin upload and wear operations.
use super::error::ElyError;
use reqwest::{multipart, Client};

pub(crate) fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result.trim().to_string()
}

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
    if let Some(err_val) = json.get("error") {
        if let Some(err_str) = err_val.as_str() {
            if !err_str.is_empty() && !err_str.contains("success") {
                let msg = json
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(strip_html_tags)
                    .unwrap_or_else(|| err_str.to_string());
                return Err(ElyError::Api(msg));
            }
        }
    }
    if let Some(success) = json.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let msg = json
                .get("message")
                .or_else(|| json.get("error"))
                .or_else(|| json.get("text"))
                .and_then(|t| t.as_str())
                .map(strip_html_tags)
                .unwrap_or_else(|| json.to_string());
            return Err(ElyError::Api(msg));
        }
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
        .map_err(ElyError::Transport)?;

    if !res.status().is_success() {
        return Err(ElyError::HttpStatus(res.status()));
    }

    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(ElyError::ParseResponse)?;

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
        .map_err(ElyError::Multipart)?;

    let form = multipart::Form::new().part("file", part);

    let res = client
        .post(format!("{ely_base}/api/legacy/skins"))
        .header("X-Requested-With", "XMLHttpRequest")
        .multipart(form)
        .send()
        .await
        .map_err(ElyError::Transport)?;

    if !res.status().is_success() {
        return Err(ElyError::HttpStatus(res.status()));
    }

    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(ElyError::ParseResponse)?;

    check_ely_web_response(&json)?;

    // If Ely.by returns a skin ID, automatically wear the uploaded skin
    if let Some(skin_id) = extract_skin_id(&json) {
        let wear_res = client
            .put(format!("{ely_base}/api/legacy/users/skin"))
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("skinId={skin_id}"))
            .send()
            .await;

        if let Ok(resp) = wear_res {
            if resp.status().is_success() {
                if let Ok(wear_json) = resp.json::<serde_json::Value>().await {
                    let _ = check_ely_web_response(&wear_json);
                }
            }
        }
    }

    Ok(())
}
