use crate::server::files::ServerFileError;
use std::fs;
use std::path::Path;

pub fn save_server_icon(server_dir: &Path, base64_data: &str) -> Result<(), ServerFileError> {
    use base64::Engine;
    let clean_b64 = if let Some(idx) = base64_data.find(',') {
        &base64_data[idx + 1..]
    } else {
        base64_data
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(clean_b64)
        .map_err(|e| ServerFileError::InvalidProperty(format!("Invalid base64 icon data: {e}")))?;
    let icon_path = server_dir.join("server-icon.png");
    fs::write(icon_path, bytes).map_err(ServerFileError::Io)
}

pub fn get_server_icon_base64(server_dir: &Path) -> Option<String> {
    use base64::Engine;
    let icon_path = server_dir.join("server-icon.png");
    if icon_path.exists() {
        if let Ok(bytes) = fs::read(&icon_path) {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
            return Some(format!("data:image/png;base64,{}", encoded));
        }
    }
    None
}
