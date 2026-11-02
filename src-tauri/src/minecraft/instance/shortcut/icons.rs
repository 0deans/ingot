//! Instance icon resolution, built-in preset rendering, and format conversion for OS and Steam shortcuts.

use base64::Engine;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;
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

fn parse_loader_spec(spec: &str) -> Option<ModLoaderType> {
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
fn normalize_to_png_bytes(raw_bytes: &[u8]) -> Option<Vec<u8>> {
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

/// Renders a built-in `preset:<id>` icon into a 256x256 PNG byte buffer.
pub fn render_preset_icon_png(preset_spec: &str) -> Option<Vec<u8>> {
    let preset_id = preset_spec
        .strip_prefix("preset:")?
        .trim()
        .to_ascii_lowercase();
    let (bg_top, bg_bottom, accent, pattern) = preset_definition(&preset_id)?;

    let size = 256u32;
    let mut canvas = RgbaImage::new(size, size);

    for (x, y, pixel) in canvas.enumerate_pixels_mut() {
        let dx = if x < 36 {
            36 - x
        } else if x >= size - 36 {
            x - (size - 37)
        } else {
            0
        };
        let dy = if y < 36 {
            36 - y
        } else if y >= size - 36 {
            y - (size - 37)
        } else {
            0
        };
        if dx * dx + dy * dy > 36 * 36 {
            *pixel = Rgba([0, 0, 0, 0]);
            continue;
        }

        let t = (y as f32 / size as f32).clamp(0.0, 1.0);
        let r = (f32::from(bg_top[0]) * (1.0 - t) + f32::from(bg_bottom[0]) * t) as u8;
        let g = (f32::from(bg_top[1]) * (1.0 - t) + f32::from(bg_bottom[1]) * t) as u8;
        let b = (f32::from(bg_top[2]) * (1.0 - t) + f32::from(bg_bottom[2]) * t) as u8;
        *pixel = Rgba([r, g, b, 255]);
    }

    // Render 8x8 symmetric pixel motif centered in the 256x256 badge (each cell = 20x20 px)
    let grid_origin = 48u32;
    let cell_size = 20u32;
    for (row_idx, row) in pattern.iter().enumerate() {
        for (col_idx, &ch) in row.as_bytes().iter().enumerate() {
            let color = match ch {
                b'#' => Some(Rgba([accent[0], accent[1], accent[2], 255])),
                b'+' => Some(Rgba([255, 255, 255, 235])),
                b'.' => Some(Rgba([14, 14, 18, 230])),
                _ => None,
            };
            let Some(fill) = color else {
                continue;
            };
            let start_x = grid_origin + (col_idx as u32) * cell_size;
            let start_y = grid_origin + (row_idx as u32) * cell_size;
            for py in start_y..(start_y + cell_size) {
                for px in start_x..(start_x + cell_size) {
                    if px < size && py < size {
                        canvas.put_pixel(px, py, fill);
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .ok()?;
    Some(out)
}

#[allow(clippy::type_complexity)]
fn preset_definition(id: &str) -> Option<([u8; 3], [u8; 3], [u8; 3], [&'static str; 8])> {
    match id {
        "creeper" => Some((
            [34, 139, 58],
            [18, 78, 32],
            [82, 214, 105],
            [
                "########", "#..##..#", "#..##..#", "###..###", "##....##", "##....##", "##.##.##",
                "########",
            ],
        )),
        "grass" => Some((
            [52, 168, 83],
            [110, 71, 43],
            [116, 224, 110],
            [
                "########", "########", "##.##.#.", "........", "........", "........", "........",
                "........",
            ],
        )),
        "sword" => Some((
            [18, 62, 89],
            [12, 28, 46],
            [56, 224, 248],
            [
                "      ##", "     ###", "    ### ", " + ###  ", "  +###  ", "  ++#   ", " +  +   ",
                "+       ",
            ],
        )),
        "pickaxe" => Some((
            [42, 48, 72],
            [18, 22, 36],
            [56, 189, 248],
            [
                " #####  ", "####### ", "##  +###", "   +  ##", "  +   # ", " +      ", "+       ",
                "        ",
            ],
        )),
        "redstone" => Some((
            [92, 18, 24],
            [38, 10, 14],
            [244, 63, 94],
            [
                "  ####  ", " ###### ", "###++###", "##++++##", "##++++##", "###++###", " ###### ",
                "  ####  ",
            ],
        )),
        "ender" => Some((
            [16, 68, 64],
            [10, 28, 32],
            [45, 212, 191],
            [
                "  ####  ", " ###### ", "##++++##", "#++..++#", "#++..++#", "##++++##", " ###### ",
                "  ####  ",
            ],
        )),
        "nether" => Some((
            [76, 29, 149],
            [32, 12, 68],
            [192, 132, 252],
            [
                "########", "#++++++#", "#+####+#", "#+#++#+#", "#+#++#+#", "#+####+#", "#++++++#",
                "########",
            ],
        )),
        "potion" => Some((
            [88, 28, 135],
            [30, 16, 56],
            [236, 72, 153],
            [
                "  ++++  ", "   ++   ", "  ####  ", " ###### ", "########", "###++###", "########",
                " ###### ",
            ],
        )),
        "command_block" => Some((
            [124, 58, 32],
            [58, 26, 14],
            [251, 146, 60],
            [
                "########", "#......#", "#.++++.#", "#.+##+.#", "#.+##+.#", "#.++++.#", "#......#",
                "########",
            ],
        )),
        "chest" => Some((
            [133, 77, 14],
            [66, 36, 8],
            [245, 158, 11],
            [
                "########", "########", "........", "###++###", "###++###", "########", "########",
                "########",
            ],
        )),
        "beacon" => Some((
            [14, 116, 144],
            [12, 42, 64],
            [103, 232, 249],
            [
                "  ++++  ", " ++++++ ", "++####++", "+######+", "+######+", "++####++", "........",
                "########",
            ],
        )),
        "book" => Some((
            [113, 63, 18],
            [54, 28, 10],
            [250, 204, 21],
            [
                " ###### ", "##++++##", "##+###+#", "##+++++#", "##+###+#", "##++++##", " ###### ",
                "        ",
            ],
        )),
        _ => None,
    }
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
        if let Some(preset_png) = render_preset_icon_png(trimmed) {
            return preset_png;
        }
        if let Some(raw_bytes) = decode_base64_image(trimmed) {
            if let Some(png_bytes) = normalize_to_png_bytes(&raw_bytes) {
                return png_bytes;
            }
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
        } else if let Some(preset_png) = render_preset_icon_png(trimmed) {
            if let Ok(ico_bytes) = png_to_ico(&preset_png) {
                let _ = std::fs::write(&ico_path, ico_bytes);
                let _ = std::fs::write(&png_path, preset_png);
                return Some(ico_path);
            }
        } else if let Some(raw_bytes) = decode_base64_image(trimmed) {
            if let Some(png_bytes) = normalize_to_png_bytes(&raw_bytes) {
                if let Ok(ico_bytes) = png_to_ico(&png_bytes) {
                    let _ = std::fs::write(&ico_path, ico_bytes);
                    let _ = std::fs::write(&png_path, png_bytes);
                    return Some(ico_path);
                }
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
        if let Some(preset_png) = render_preset_icon_png(trimmed) {
            let _ = std::fs::write(&png_path, preset_png);
            return Some(png_path);
        }
        if let Some(raw_bytes) = decode_base64_image(trimmed) {
            if let Some(png_bytes) = normalize_to_png_bytes(&raw_bytes) {
                let _ = std::fs::write(&png_path, png_bytes);
                return Some(png_path);
            }
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
