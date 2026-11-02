//! Instance icon resolution and format conversion for OS and Steam shortcuts.

use base64::Engine;
use std::path::{Path, PathBuf};

use crate::minecraft::instance::models::{InstanceConfig, ModLoaderType};

/// Retrieves the bundled multi-resolution ICO icon for a mod loader if available.
pub fn get_default_loader_icon_ico(loader: &ModLoaderType) -> Option<&'static [u8]> {
    match loader {
        ModLoaderType::Vanilla => {
            Some(include_bytes!("../../../../icons/loaders/vanilla.ico").as_slice())
        }
        ModLoaderType::Fabric => {
            Some(include_bytes!("../../../../icons/loaders/fabric.ico").as_slice())
        }
        ModLoaderType::Forge => {
            Some(include_bytes!("../../../../icons/loaders/forge.ico").as_slice())
        }
        ModLoaderType::NeoForge => {
            Some(include_bytes!("../../../../icons/loaders/neoforge.ico").as_slice())
        }
        ModLoaderType::Quilt => {
            Some(include_bytes!("../../../../icons/loaders/quilt.ico").as_slice())
        }
        ModLoaderType::Horizon => {
            Some(include_bytes!("../../../../icons/loaders/horizon.ico").as_slice())
        }
    }
}

/// Retrieves the bundled PNG icon for a mod loader if available.
pub fn get_default_loader_icon_png(loader: &ModLoaderType) -> Option<&'static [u8]> {
    match loader {
        ModLoaderType::Vanilla => {
            Some(include_bytes!("../../../../icons/loaders/vanilla.png").as_slice())
        }
        ModLoaderType::Fabric => {
            Some(include_bytes!("../../../../icons/loaders/fabric.png").as_slice())
        }
        ModLoaderType::Forge => {
            Some(include_bytes!("../../../../icons/loaders/forge.png").as_slice())
        }
        ModLoaderType::NeoForge => {
            Some(include_bytes!("../../../../icons/loaders/neoforge.png").as_slice())
        }
        ModLoaderType::Quilt => {
            Some(include_bytes!("../../../../icons/loaders/quilt.png").as_slice())
        }
        ModLoaderType::Horizon => {
            Some(include_bytes!("../../../../icons/loaders/horizon.png").as_slice())
        }
    }
}

/// Encapsulates raw PNG image bytes in a standard Windows .ico container.
pub fn png_to_ico(png_bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    if png_bytes.len() < 24 || &png_bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid PNG image data",
        ));
    }

    let width = u32::from_be_bytes([png_bytes[16], png_bytes[17], png_bytes[18], png_bytes[19]]);
    let height = u32::from_be_bytes([png_bytes[20], png_bytes[21], png_bytes[22], png_bytes[23]]);
    let b_width = if width >= 256 { 0 } else { width as u8 };
    let b_height = if height >= 256 { 0 } else { height as u8 };

    let mut ico = Vec::with_capacity(22 + png_bytes.len());
    // ICONDIR
    ico.extend_from_slice(&0u16.to_le_bytes()); // Reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // Type: 1 for Icon
    ico.extend_from_slice(&1u16.to_le_bytes()); // Count: 1 image

    // ICONDIRENTRY
    ico.push(b_width);
    ico.push(b_height);
    ico.push(0); // Color count
    ico.push(0); // Reserved
    ico.extend_from_slice(&0u16.to_le_bytes()); // Planes: 0 for PNG
    ico.extend_from_slice(&32u16.to_le_bytes()); // Bit count: 32
    ico.extend_from_slice(&(png_bytes.len() as u32).to_le_bytes()); // Resource size
    ico.extend_from_slice(&22u32.to_le_bytes()); // Offset: 22

    ico.extend_from_slice(png_bytes);
    Ok(ico)
}

pub(super) fn decode_base64_image(spec: &str) -> Option<Vec<u8>> {
    let comma_pos = spec.find(',')?;
    let header = &spec[..comma_pos];
    if !header.starts_with("data:image/") || !header.contains(";base64") {
        return None;
    }
    let encoded = &spec[comma_pos + 1..];
    base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()
}

/// Resolves raw image bytes (PNG/JPEG) for an instance icon, falling back to the loader PNG.
pub fn resolve_instance_icon_bytes(instance_dir: &Path, instance: &InstanceConfig) -> Vec<u8> {
    if let Some(icon_spec) = &instance.icon {
        if let Some(bytes) = decode_base64_image(icon_spec) {
            return bytes;
        }
        let custom_path = PathBuf::from(icon_spec);
        if custom_path.is_file() {
            if let Ok(bytes) = std::fs::read(&custom_path) {
                if !bytes.is_empty() {
                    return bytes;
                }
            }
        }
    }

    let png_path = instance_dir.join("icon.png");
    if png_path.is_file() {
        if let Ok(bytes) = std::fs::read(&png_path) {
            if bytes.len() > 400 {
                return bytes;
            }
        }
    }

    get_default_loader_icon_png(&instance.loader)
        .unwrap_or_default()
        .to_vec()
}

#[cfg(windows)]
fn resolve_windows_instance_icon(
    instance_dir: &Path,
    instance: &InstanceConfig,
) -> Option<PathBuf> {
    let ico_path = instance_dir.join("icon.ico");
    let png_path = instance_dir.join("icon.png");

    if let Some(icon_spec) = &instance.icon {
        if let Some(bytes) = decode_base64_image(icon_spec) {
            if let Ok(ico_bytes) = png_to_ico(&bytes) {
                let _ = std::fs::write(&ico_path, ico_bytes);
                let _ = std::fs::write(&png_path, bytes);
                return Some(ico_path);
            }
        } else {
            let custom_path = PathBuf::from(icon_spec);
            if custom_path.is_file() {
                if custom_path.extension().and_then(|e| e.to_str()) == Some("ico") {
                    return Some(custom_path);
                }
                if let Ok(bytes) = std::fs::read(&custom_path) {
                    if let Ok(ico_bytes) = png_to_ico(&bytes) {
                        let _ = std::fs::write(&ico_path, ico_bytes);
                        return Some(ico_path);
                    }
                }
            }
        }
    }

    if let Some(loader_ico) = get_default_loader_icon_ico(&instance.loader) {
        if std::fs::write(&ico_path, loader_ico).is_ok() {
            if let Some(loader_png) = get_default_loader_icon_png(&instance.loader) {
                let _ = std::fs::write(&png_path, loader_png);
            }
            return Some(ico_path);
        }
    }

    if png_path.is_file() {
        if let Ok(bytes) = std::fs::read(&png_path) {
            if bytes.len() > 400 {
                if let Ok(ico_bytes) = png_to_ico(&bytes) {
                    if std::fs::write(&ico_path, ico_bytes).is_ok() {
                        return Some(ico_path);
                    }
                }
            }
        }
    }

    if ico_path.is_file() {
        if let Ok(meta) = std::fs::metadata(&ico_path) {
            if meta.len() > 1000 {
                return Some(ico_path);
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn resolve_unix_instance_icon(instance_dir: &Path, instance: &InstanceConfig) -> Option<PathBuf> {
    let png_path = instance_dir.join("icon.png");

    if let Some(icon_spec) = &instance.icon {
        if let Some(bytes) = decode_base64_image(icon_spec) {
            let _ = std::fs::write(&png_path, bytes);
            return Some(png_path);
        }
        let custom_path = PathBuf::from(icon_spec);
        if custom_path.is_file() {
            return Some(custom_path);
        }
    }

    if let Some(loader_png) = get_default_loader_icon_png(&instance.loader) {
        if std::fs::write(&png_path, loader_png).is_ok() {
            return Some(png_path);
        }
    }

    if png_path.is_file() {
        if let Ok(meta) = std::fs::metadata(&png_path) {
            if meta.len() > 400 {
                return Some(png_path);
            }
        }
    }

    None
}

/// Resolves the file path of an OS-compatible icon for the specified instance.
pub fn resolve_instance_icon(instance_dir: &Path, instance: &InstanceConfig) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        resolve_windows_instance_icon(instance_dir, instance)
    }
    #[cfg(not(windows))]
    {
        resolve_unix_instance_icon(instance_dir, instance)
    }
}
