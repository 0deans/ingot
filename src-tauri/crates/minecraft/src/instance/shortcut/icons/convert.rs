//! Bundled loader icon lookup, data URL decoding, image normalization, and `.ico` encoding.

use base64::Engine;
use image::imageops::FilterType;
use image::ImageFormat;
use std::io::Cursor;

use crate::instance::models::ModLoaderType;

/// Retrieves the bundled multi-resolution ICO icon for a mod loader if available.
pub fn get_default_loader_icon_ico(loader: &ModLoaderType) -> Option<&'static [u8]> {
    match loader {
        ModLoaderType::Vanilla => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/vanilla.ico"
            ))
            .as_slice(),
        ),
        ModLoaderType::Fabric => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/fabric.ico"
            ))
            .as_slice(),
        ),
        ModLoaderType::Forge => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/forge.ico"
            ))
            .as_slice(),
        ),
        ModLoaderType::NeoForge => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/neoforge.ico"
            ))
            .as_slice(),
        ),
        ModLoaderType::Quilt => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/quilt.ico"
            ))
            .as_slice(),
        ),
        ModLoaderType::Horizon => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/horizon.ico"
            ))
            .as_slice(),
        ),
    }
}

/// Retrieves the bundled PNG icon for a mod loader if available.
pub fn get_default_loader_icon_png(loader: &ModLoaderType) -> Option<&'static [u8]> {
    match loader {
        ModLoaderType::Vanilla => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/vanilla.png"
            ))
            .as_slice(),
        ),
        ModLoaderType::Fabric => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/fabric.png"
            ))
            .as_slice(),
        ),
        ModLoaderType::Forge => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/forge.png"
            ))
            .as_slice(),
        ),
        ModLoaderType::NeoForge => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/neoforge.png"
            ))
            .as_slice(),
        ),
        ModLoaderType::Quilt => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/quilt.png"
            ))
            .as_slice(),
        ),
        ModLoaderType::Horizon => Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../icons/loaders/horizon.png"
            ))
            .as_slice(),
        ),
    }
}

pub(super) fn parse_loader_spec(spec: &str) -> Option<ModLoaderType> {
    let name = spec.strip_prefix("loader:")?.trim().to_ascii_lowercase();
    match name.as_str() {
        "vanilla" => Some(ModLoaderType::Vanilla),
        "fabric" => Some(ModLoaderType::Fabric),
        "forge" => Some(ModLoaderType::Forge),
        "neoforge" => Some(ModLoaderType::NeoForge),
        "quilt" => Some(ModLoaderType::Quilt),
        "horizon" => Some(ModLoaderType::Horizon),
        _ => None,
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
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());

    ico.push(b_width);
    ico.push(b_height);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&(png_bytes.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());

    ico.extend_from_slice(png_bytes);
    Ok(ico)
}

pub(super) fn decode_base64_image(spec: &str) -> Option<Vec<u8>> {
    let comma_pos = spec.find(',')?;
    let header = &spec[..comma_pos];
    if !header.starts_with("data:image/") || !header.contains(";base64") {
        return None;
    }

    // If an animated GIF data URL carries a pre-rendered PNG frame parameter (`;frame=<b64>`),
    // prefer that static PNG frame for OS `.ico` and Steam Grid rasterization.
    for segment in header.split(';') {
        if let Some(frame_b64) = segment.strip_prefix("frame=") {
            if let Ok(decoded_frame) =
                base64::engine::general_purpose::STANDARD.decode(frame_b64.trim())
            {
                if !decoded_frame.is_empty() {
                    return Some(decoded_frame);
                }
            }
        }
    }

    let encoded = &spec[comma_pos + 1..];
    base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()
}

/// Normalizes arbitrary image bytes (PNG or JPEG) into a 256x256 PNG byte buffer.
pub(super) fn normalize_to_png_bytes(raw_bytes: &[u8]) -> Option<Vec<u8>> {
    if raw_bytes.len() >= 24 && &raw_bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        return Some(raw_bytes.to_vec());
    }
    let decoded = image::load_from_memory(raw_bytes).ok()?;
    let resized = decoded.resize_exact(256, 256, FilterType::Lanczos3);
    let mut out = Vec::new();
    resized
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};

    fn sample_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let img = RgbaImage::new(width, height);
        let mut out = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn parses_loader_specs_case_insensitively() {
        assert_eq!(
            parse_loader_spec("loader:Fabric"),
            Some(ModLoaderType::Fabric)
        );
        assert_eq!(
            parse_loader_spec("loader: neoforge "),
            Some(ModLoaderType::NeoForge)
        );
        assert_eq!(parse_loader_spec("loader:unknown"), None);
        assert_eq!(parse_loader_spec("preset:creeper"), None);
    }

    #[test]
    fn decode_base64_image_prefers_static_frame_over_main_payload() {
        let b64 = base64::engine::general_purpose::STANDARD;
        let frame_bytes = b"frame-png-payload";
        let main_bytes = b"animated-gif-payload";
        let data_url = format!(
            "data:image/gif;frame={};base64,{}",
            b64.encode(frame_bytes),
            b64.encode(main_bytes)
        );

        assert_eq!(decode_base64_image(&data_url), Some(frame_bytes.to_vec()));

        let plain_url = format!("data:image/png;base64,{}", b64.encode(main_bytes));
        assert_eq!(decode_base64_image(&plain_url), Some(main_bytes.to_vec()));

        assert_eq!(decode_base64_image("data:text/plain;base64,YQ=="), None);
        assert_eq!(decode_base64_image("not-a-data-url"), None);
    }

    #[test]
    fn normalize_to_png_bytes_passes_through_png_and_resizes_jpeg() {
        let png = sample_png_bytes(32, 32);
        assert_eq!(normalize_to_png_bytes(&png), Some(png));

        let img = image::RgbImage::new(16, 16);
        let mut jpeg = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg)
            .unwrap();

        let normalized = normalize_to_png_bytes(&jpeg).unwrap();
        let decoded = image::load_from_memory(&normalized).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (256, 256));
    }

    #[test]
    fn png_to_ico_encodes_256px_dimensions_as_zero_and_small_dimensions_verbatim() {
        let png_256 = sample_png_bytes(256, 256);
        let ico_256 = png_to_ico(&png_256).unwrap();
        assert_eq!(ico_256[6], 0);
        assert_eq!(ico_256[7], 0);
        assert_eq!(&ico_256[22..], png_256.as_slice());

        let png_64 = sample_png_bytes(64, 48);
        let ico_64 = png_to_ico(&png_64).unwrap();
        assert_eq!(ico_64[6], 64);
        assert_eq!(ico_64[7], 48);
    }
}
