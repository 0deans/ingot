//! Native skin cache and save-dialog adapter; cache cleanup remains best effort.
use std::fs;
use tauri::Manager;
pub(crate) async fn get_skin_data_url<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    skin_url: String,
) -> Result<String, String> {
    use base64::Engine;

    if skin_url.trim().is_empty() {
        return Err("Skin URL is empty".to_string());
    }

    let url_hash = uuid::Uuid::new_v3(&uuid::Uuid::NAMESPACE_URL, skin_url.as_bytes());
    let filename = format!("{url_hash:x}.png");
    let cache_dir = app
        .path()
        .app_cache_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| format!("Failed to get cache dir: {e}"))?
        .join("skins");

    if !cache_dir.exists() {
        let _ = fs::create_dir_all(&cache_dir);
    }

    let file_path = cache_dir.join(&filename);

    if file_path.exists() {
        if let Ok(bytes) = fs::read(&file_path) {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
            return Ok(format!("data:image/png;base64,{encoded}"));
        }
    }

    let client = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let resp = client
        .get(&skin_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch skin: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Server returned HTTP {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read skin bytes: {e}"))?;

    let _ = fs::write(&file_path, &bytes);

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

pub(crate) async fn save_skin_to_downloads<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
    skin_url: String,
) -> Result<String, String> {
    use base64::Engine;

    if skin_url.trim().is_empty() {
        return Err("Skin URL is empty".to_string());
    }

    let bytes: Vec<u8> = if skin_url.starts_with("data:") {
        let comma_pos = skin_url
            .find(',')
            .ok_or_else(|| "Invalid data URL".to_string())?;
        let base64_data = &skin_url[comma_pos + 1..];
        base64::engine::general_purpose::STANDARD
            .decode(base64_data.trim())
            .map_err(|e| format!("Failed to decode base64 skin data: {e}"))?
    } else {
        let url_hash = uuid::Uuid::new_v3(&uuid::Uuid::NAMESPACE_URL, skin_url.as_bytes());
        let cache_filename = format!("{url_hash:x}.png");
        let cache_dir = app
            .path()
            .app_cache_dir()
            .or_else(|_| app.path().app_data_dir())
            .map_err(|e| format!("Failed to get cache dir: {e}"))?
            .join("skins");

        let cached_path = cache_dir.join(&cache_filename);
        if cached_path.exists() {
            fs::read(&cached_path).map_err(|e| format!("Failed to read cached skin: {e}"))?
        } else {
            let client = reqwest::Client::builder()
                .user_agent(crate::USER_AGENT)
                .build()
                .map_err(|e| format!("Failed to build client: {e}"))?;

            let resp = client
                .get(&skin_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch skin: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("Server returned HTTP {}", resp.status()));
            }

            let b = resp
                .bytes()
                .await
                .map_err(|e| format!("Failed to read skin bytes: {e}"))?;

            let _ = fs::create_dir_all(&cache_dir);
            let _ = fs::write(&cached_path, &b);
            b.to_vec()
        }
    };

    let download_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().app_data_dir())
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let clean_user = username
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let base_name = if clean_user.is_empty() {
        "minecraft-skin".to_string()
    } else {
        format!("{clean_user}-skin")
    };
    let default_file_name = format!("{base_name}.png");

    use tauri_plugin_dialog::DialogExt;
    let file_path = app
        .dialog()
        .file()
        .set_title("Save Skin As")
        .set_directory(&download_dir)
        .set_file_name(&default_file_name)
        .add_filter("PNG Image (*.png)", &["png"])
        .blocking_save_file();

    let target_path = match file_path {
        Some(path) => match path.into_path() {
            Ok(p) => p,
            Err(_) => return Err("Invalid destination file path".to_string()),
        },
        None => return Ok(String::new()), // User cancelled dialog
    };

    fs::write(&target_path, &bytes).map_err(|e| format!("Failed to write skin file: {e}"))?;

    Ok(target_path.to_string_lossy().to_string())
}
