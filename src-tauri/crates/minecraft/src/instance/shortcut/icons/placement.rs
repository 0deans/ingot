//! Instance icon specification resolution and OS-specific icon file placement.

use std::path::{Path, PathBuf};

use crate::instance::models::InstanceConfig;

use super::convert::{
    decode_base64_image, get_default_loader_icon_png, normalize_to_png_bytes, parse_loader_spec,
};
#[cfg(windows)]
use super::convert::{get_default_loader_icon_ico, png_to_ico};
use super::presets::render_preset_icon_png;

fn resolve_rendered_or_data_png(trimmed_spec: &str) -> Option<Vec<u8>> {
    if let Some(preset_png) = render_preset_icon_png(trimmed_spec) {
        return Some(preset_png);
    }
    let raw_bytes = decode_base64_image(trimmed_spec)?;
    normalize_to_png_bytes(&raw_bytes)
}

/// Resolves a normalized PNG byte buffer for an instance icon (supporting custom images, GIF frames, presets, and loader icons).
pub fn resolve_instance_icon_bytes(instance_dir: &Path, instance: &InstanceConfig) -> Vec<u8> {
    if let Some(icon_spec) = &instance.icon {
        let trimmed = icon_spec.trim();
        if let Some(loader) = parse_loader_spec(trimmed) {
            if let Some(bytes) = get_default_loader_icon_png(&loader) {
                return bytes.to_vec();
            }
        }
        if let Some(png_bytes) = resolve_rendered_or_data_png(trimmed) {
            return png_bytes;
        }
        let custom_path = PathBuf::from(trimmed);
        if custom_path.is_file() {
            if let Ok(raw_bytes) = std::fs::read(&custom_path) {
                if let Some(png_bytes) = normalize_to_png_bytes(&raw_bytes) {
                    return png_bytes;
                }
            }
        }
    }

    let png_path = instance_dir.join("icon.png");
    if png_path.is_file() {
        if let Ok(bytes) = std::fs::read(&png_path) {
            if bytes.len() > 400 {
                if let Some(png_bytes) = normalize_to_png_bytes(&bytes) {
                    return png_bytes;
                }
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
        let trimmed = icon_spec.trim();
        if let Some(loader) = parse_loader_spec(trimmed) {
            if let Some(loader_ico) = get_default_loader_icon_ico(&loader) {
                if std::fs::write(&ico_path, loader_ico).is_ok() {
                    if let Some(loader_png) = get_default_loader_icon_png(&loader) {
                        let _ = std::fs::write(&png_path, loader_png);
                    }
                    return Some(ico_path);
                }
            }
        } else if let Some(png_bytes) = resolve_rendered_or_data_png(trimmed) {
            if let Ok(ico_bytes) = png_to_ico(&png_bytes) {
                let _ = std::fs::write(&ico_path, ico_bytes);
                let _ = std::fs::write(&png_path, png_bytes);
                return Some(ico_path);
            }
        } else {
            let custom_path = PathBuf::from(trimmed);
            if custom_path.is_file() {
                if custom_path.extension().and_then(|e| e.to_str()) == Some("ico") {
                    return Some(custom_path);
                }
                if let Ok(raw_bytes) = std::fs::read(&custom_path) {
                    if let Some(png_bytes) = normalize_to_png_bytes(&raw_bytes) {
                        if let Ok(ico_bytes) = png_to_ico(&png_bytes) {
                            let _ = std::fs::write(&ico_path, ico_bytes);
                            return Some(ico_path);
                        }
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
                if let Some(png_bytes) = normalize_to_png_bytes(&bytes) {
                    if let Ok(ico_bytes) = png_to_ico(&png_bytes) {
                        if std::fs::write(&ico_path, ico_bytes).is_ok() {
                            return Some(ico_path);
                        }
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
        let trimmed = icon_spec.trim();
        if let Some(loader) = parse_loader_spec(trimmed) {
            if let Some(loader_png) = get_default_loader_icon_png(&loader) {
                let _ = std::fs::write(&png_path, loader_png);
                return Some(png_path);
            }
        }
        if let Some(png_bytes) = resolve_rendered_or_data_png(trimmed) {
            let _ = std::fs::write(&png_path, png_bytes);
            return Some(png_path);
        }
        let custom_path = PathBuf::from(trimmed);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::models::ModLoaderType;
    use base64::Engine;
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use std::io::Cursor;

    fn sample_instance(loader: ModLoaderType, icon: Option<&str>) -> InstanceConfig {
        InstanceConfig {
            id: "inst-icon".to_string(),
            name: "Icon Test".to_string(),
            game_version: "1.21.4".to_string(),
            loader,
            loader_version: None,
            java_path: None,
            memory_min_mb: None,
            memory_max_mb: None,
            icon: icon.map(ToString::to_string),
            group: None,
            color_tag: None,
            created_at: 0,
            last_played: None,
            total_play_time_seconds: 0,
            jvm_args: None,
            fullscreen: None,
            window_width: None,
            window_height: None,
            sync_options: None,
            sync_servers: None,
            sync_resource_packs: None,
            sync_command_history: None,
            sync_creative_hotbars: None,
            last_synced_at: None,
        }
    }

    #[test]
    fn resolves_instance_icon_bytes_for_loader_preset_data_url_and_directory_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();

        let loader_instance = sample_instance(ModLoaderType::Vanilla, Some("loader:fabric"));
        assert_eq!(
            resolve_instance_icon_bytes(temp_dir.path(), &loader_instance),
            get_default_loader_icon_png(&ModLoaderType::Fabric)
                .unwrap()
                .to_vec()
        );

        let preset_instance = sample_instance(ModLoaderType::Vanilla, Some("preset:creeper"));
        let preset_bytes = resolve_instance_icon_bytes(temp_dir.path(), &preset_instance);
        let decoded = image::load_from_memory(&preset_bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (256, 256));

        let mut custom_png = Vec::new();
        DynamicImage::ImageRgba8(RgbaImage::new(48, 48))
            .write_to(&mut Cursor::new(&mut custom_png), ImageFormat::Png)
            .unwrap();
        let data_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&custom_png)
        );
        let data_instance = sample_instance(ModLoaderType::Vanilla, Some(&data_url));
        assert_eq!(
            resolve_instance_icon_bytes(temp_dir.path(), &data_instance),
            custom_png
        );

        // Corrupt/tiny instance_dir/icon.png (<= 400 bytes) is ignored in favor of default loader icon.
        std::fs::write(temp_dir.path().join("icon.png"), vec![1u8; 64]).unwrap();
        let fallback_instance = sample_instance(ModLoaderType::Quilt, None);
        assert_eq!(
            resolve_instance_icon_bytes(temp_dir.path(), &fallback_instance),
            get_default_loader_icon_png(&ModLoaderType::Quilt)
                .unwrap()
                .to_vec()
        );
    }

    #[test]
    fn resolve_instance_icon_writes_preset_assets_in_instance_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let instance = sample_instance(ModLoaderType::Vanilla, Some("preset:sword"));

        let resolved = resolve_instance_icon(temp_dir.path(), &instance).unwrap();
        assert!(resolved.is_file());
        assert!(temp_dir.path().join("icon.png").is_file());
        #[cfg(windows)]
        {
            assert_eq!(resolved, temp_dir.path().join("icon.ico"));
        }
        #[cfg(not(windows))]
        {
            assert_eq!(resolved, temp_dir.path().join("icon.png"));
        }
    }
}
