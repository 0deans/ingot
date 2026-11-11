//! Custom Steam Grid banner, capsule, hero, logo, and icon generator for Steam Deck & Big Picture Mode.

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use crate::instance::models::{InstanceConfig, ModLoaderType};
use crate::instance::shortcut::icons::resolve_instance_icon_bytes;
use crate::instance::shortcut::ShortcutError;

const PORTRAIT_WIDTH: u32 = 600;
const PORTRAIT_HEIGHT: u32 = 900;

const WIDE_WIDTH: u32 = 920;
const WIDE_HEIGHT: u32 = 430;

const HERO_WIDTH: u32 = 1920;
const HERO_HEIGHT: u32 = 620;

const LOGO_WIDTH: u32 = 600;
const LOGO_HEIGHT: u32 = 300;

const ICON_SIZE: u32 = 256;

#[derive(Debug, Clone, Copy)]
struct LoaderPalette {
    primary: [u8; 3],
    secondary: [u8; 3],
}

fn palette_for_loader(loader: &ModLoaderType) -> LoaderPalette {
    match loader {
        ModLoaderType::Vanilla => LoaderPalette {
            primary: [62, 180, 95],
            secondary: [22, 74, 42],
        },
        ModLoaderType::Fabric => LoaderPalette {
            primary: [219, 196, 150],
            secondary: [110, 88, 52],
        },
        ModLoaderType::Forge => LoaderPalette {
            primary: [224, 138, 52],
            secondary: [105, 52, 18],
        },
        ModLoaderType::NeoForge => LoaderPalette {
            primary: [242, 108, 34],
            secondary: [115, 42, 14],
        },
        ModLoaderType::Quilt => LoaderPalette {
            primary: [168, 85, 247],
            secondary: [68, 28, 115],
        },
        ModLoaderType::Horizon => LoaderPalette {
            primary: [56, 189, 248],
            secondary: [18, 72, 115],
        },
    }
}

fn palette_for_color_tag(tag: &str) -> Option<LoaderPalette> {
    let trimmed = tag.trim();
    if let Some(rgb) = parse_hex_color(trimmed) {
        let secondary = [
            (u16::from(rgb[0]) * 38 / 100) as u8,
            (u16::from(rgb[1]) * 38 / 100) as u8,
            (u16::from(rgb[2]) * 38 / 100) as u8,
        ];
        return Some(LoaderPalette {
            primary: rgb,
            secondary,
        });
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "red" => Some(LoaderPalette {
            primary: [239, 68, 68],
            secondary: [92, 18, 24],
        }),
        "orange" => Some(LoaderPalette {
            primary: [249, 115, 22],
            secondary: [98, 38, 12],
        }),
        "amber" => Some(LoaderPalette {
            primary: [245, 158, 11],
            secondary: [96, 58, 10],
        }),
        "emerald" => Some(LoaderPalette {
            primary: [16, 185, 129],
            secondary: [10, 74, 52],
        }),
        "cyan" => Some(LoaderPalette {
            primary: [6, 182, 212],
            secondary: [12, 68, 86],
        }),
        "blue" => Some(LoaderPalette {
            primary: [59, 130, 246],
            secondary: [18, 46, 102],
        }),
        "violet" => Some(LoaderPalette {
            primary: [139, 92, 246],
            secondary: [52, 26, 104],
        }),
        "pink" => Some(LoaderPalette {
            primary: [236, 72, 153],
            secondary: [92, 20, 58],
        }),
        "rose" => Some(LoaderPalette {
            primary: [244, 63, 94],
            secondary: [96, 18, 38],
        }),
        _ => None,
    }
}

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

#[derive(Debug, Clone)]
pub struct GeneratedGridAssets {
    pub portrait_path: PathBuf,
    pub wide_path: PathBuf,
    pub hero_path: PathBuf,
    pub logo_path: PathBuf,
    pub icon_path: PathBuf,
}

/// Generates all 5 Steam Grid artwork files inside `grid_dir` for the given `app_id`.
pub fn generate_steam_grid_assets(
    grid_dir: &Path,
    app_id: u32,
    instance_dir: &Path,
    instance: &InstanceConfig,
) -> Result<GeneratedGridAssets, ShortcutError> {
    std::fs::create_dir_all(grid_dir)?;

    let icon_bytes = resolve_instance_icon_bytes(instance_dir, instance);
    let icon_image = image::load_from_memory(&icon_bytes)
        .map_err(|e| ShortcutError::Image(format!("Failed to decode instance icon: {e}")))?;

    let latest_screenshot = load_latest_screenshot(instance_dir);
    let palette = instance
        .color_tag
        .as_deref()
        .and_then(palette_for_color_tag)
        .unwrap_or_else(|| palette_for_loader(&instance.loader));

    let portrait_path = grid_dir.join(format!("{app_id}p.png"));
    let wide_path = grid_dir.join(format!("{app_id}.png"));
    let hero_path = grid_dir.join(format!("{app_id}_hero.png"));
    let logo_path = grid_dir.join(format!("{app_id}_logo.png"));
    let icon_path = grid_dir.join(format!("{app_id}_icon.png"));

    let mut portrait_canvas = render_backdrop(
        PORTRAIT_WIDTH,
        PORTRAIT_HEIGHT,
        latest_screenshot.as_ref(),
        palette,
    );
    draw_centered_icon_badge(&mut portrait_canvas, &icon_image, 220, 268, palette);
    write_png_atomic(grid_dir, &portrait_path, &portrait_canvas)?;

    let mut wide_canvas =
        render_backdrop(WIDE_WIDTH, WIDE_HEIGHT, latest_screenshot.as_ref(), palette);
    draw_centered_icon_badge(&mut wide_canvas, &icon_image, 160, 200, palette);
    write_png_atomic(grid_dir, &wide_path, &wide_canvas)?;

    let hero_canvas = render_backdrop(HERO_WIDTH, HERO_HEIGHT, latest_screenshot.as_ref(), palette);
    write_png_atomic(grid_dir, &hero_path, &hero_canvas)?;

    let mut logo_canvas = RgbaImage::from_pixel(LOGO_WIDTH, LOGO_HEIGHT, Rgba([0, 0, 0, 0]));
    draw_centered_icon_badge(&mut logo_canvas, &icon_image, 180, 224, palette);
    write_png_atomic(grid_dir, &logo_path, &logo_canvas)?;

    let square_icon = icon_image
        .resize_exact(ICON_SIZE, ICON_SIZE, FilterType::Lanczos3)
        .to_rgba8();
    write_png_atomic(grid_dir, &icon_path, &square_icon)?;

    Ok(GeneratedGridAssets {
        portrait_path,
        wide_path,
        hero_path,
        logo_path,
        icon_path,
    })
}

fn load_latest_screenshot(instance_dir: &Path) -> Option<DynamicImage> {
    let screenshots_dir = instance_dir.join("screenshots");
    let entries = std::fs::read_dir(screenshots_dir).ok()?;

    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let ext = path.extension()?.to_str()?;
            if !ext.eq_ignore_ascii_case("png")
                && !ext.eq_ignore_ascii_case("jpg")
                && !ext.eq_ignore_ascii_case("jpeg")
            {
                return None;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            Some((modified, path))
        })
        .collect();

    candidates.sort_by_key(|a| std::cmp::Reverse(a.0));

    for (_, path) in candidates.into_iter().take(3) {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(decoded) = image::load_from_memory(&bytes) {
                return Some(decoded);
            }
        }
    }

    None
}

fn render_backdrop(
    width: u32,
    height: u32,
    screenshot: Option<&DynamicImage>,
    palette: LoaderPalette,
) -> RgbaImage {
    if let Some(shot) = screenshot {
        let mut canvas = shot
            .resize_to_fill(width, height, FilterType::Triangle)
            .to_rgba8();
        apply_screenshot_vignette(&mut canvas, palette);
        return canvas;
    }

    render_procedural_backdrop(width, height, palette)
}

fn apply_screenshot_vignette(canvas: &mut RgbaImage, palette: LoaderPalette) {
    let width = canvas.width() as f32;
    let height = canvas.height() as f32;
    let center_x = width * 0.5;
    let center_y = height * 0.5;
    let max_dist = (center_x * center_x + center_y * center_y).sqrt().max(1.0);

    for (x, y, pixel) in canvas.enumerate_pixels_mut() {
        let dx = x as f32 - center_x;
        let dy = y as f32 - center_y;
        let radial = ((dx * dx + dy * dy).sqrt() / max_dist).clamp(0.0, 1.0);
        let vertical = (y as f32 / height).clamp(0.0, 1.0);

        // Darken screenshot to 32-48% brightness and tint with loader secondary color
        let keep = 0.48 - 0.22 * radial - 0.08 * vertical;
        let tint_strength = 0.24 * radial + 0.10;

        let r = (f32::from(pixel[0]) * keep + f32::from(palette.secondary[0]) * tint_strength)
            .clamp(0.0, 255.0) as u8;
        let g = (f32::from(pixel[1]) * keep + f32::from(palette.secondary[1]) * tint_strength)
            .clamp(0.0, 255.0) as u8;
        let b = (f32::from(pixel[2]) * keep + f32::from(palette.secondary[2]) * tint_strength)
            .clamp(0.0, 255.0) as u8;

        *pixel = Rgba([r, g, b, 255]);
    }
}

fn render_procedural_backdrop(width: u32, height: u32, palette: LoaderPalette) -> RgbaImage {
    let mut canvas = RgbaImage::new(width, height);
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    let center_x = w * 0.5;
    let center_y = h * 0.46;
    let radius = (w.min(h) * 0.72).max(1.0);

    for (x, y, pixel) in canvas.enumerate_pixels_mut() {
        let nx = x as f32 / w;
        let ny = y as f32 / h;

        let dx = x as f32 - center_x;
        let dy = y as f32 - center_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let glow = (1.0 - (dist / radius)).clamp(0.0, 1.0);
        let smooth_glow = glow * glow;

        let diag = ((1.0 - nx) * 0.5 + (1.0 - ny) * 0.5).clamp(0.0, 1.0);

        let base_r = 11.0
            + f32::from(palette.secondary[0]) * 0.45 * diag
            + f32::from(palette.primary[0]) * 0.32 * smooth_glow;
        let base_g = 11.0
            + f32::from(palette.secondary[1]) * 0.45 * diag
            + f32::from(palette.primary[1]) * 0.32 * smooth_glow;
        let base_b = 15.0
            + f32::from(palette.secondary[2]) * 0.45 * diag
            + f32::from(palette.primary[2]) * 0.32 * smooth_glow;

        // Subtle 32px technical grid lines
        let is_grid = (x % 32 == 0) || (y % 32 == 0);
        let grid_boost = if is_grid { 7.0 } else { 0.0 };

        let r = (base_r + grid_boost).clamp(0.0, 255.0) as u8;
        let g = (base_g + grid_boost).clamp(0.0, 255.0) as u8;
        let b = (base_b + grid_boost).clamp(0.0, 255.0) as u8;

        *pixel = Rgba([r, g, b, 255]);
    }

    canvas
}

fn draw_centered_icon_badge(
    canvas: &mut RgbaImage,
    icon_image: &DynamicImage,
    icon_size: u32,
    badge_size: u32,
    palette: LoaderPalette,
) {
    let canvas_w = canvas.width();
    let canvas_h = canvas.height();
    let badge_x = canvas_w.saturating_sub(badge_size) / 2;
    let badge_y = canvas_h.saturating_sub(badge_size) / 2;
    let corner_radius = (badge_size / 6).max(12) as i32;

    draw_rounded_badge(
        canvas,
        badge_x,
        badge_y,
        badge_size,
        corner_radius,
        Rgba([16, 16, 22, 224]),
        Rgba([
            palette.primary[0],
            palette.primary[1],
            palette.primary[2],
            175,
        ]),
    );

    let resized_icon = icon_image
        .resize_exact(icon_size, icon_size, FilterType::Lanczos3)
        .to_rgba8();
    let icon_x = canvas_w.saturating_sub(icon_size) / 2;
    let icon_y = canvas_h.saturating_sub(icon_size) / 2;

    overlay_rgba(canvas, &resized_icon, icon_x, icon_y);
}

fn draw_rounded_badge(
    canvas: &mut RgbaImage,
    left: u32,
    top: u32,
    size: u32,
    radius: i32,
    fill: Rgba<u8>,
    border: Rgba<u8>,
) {
    let size_i = size as i32;
    let border_width = 2i32;

    for dy in 0..size_i {
        for dx in 0..size_i {
            let dist = rounded_rect_sdf(dx, dy, size_i, size_i, radius);
            if dist > 0 {
                continue;
            }
            let px = left + dx as u32;
            let py = top + dy as u32;
            if px >= canvas.width() || py >= canvas.height() {
                continue;
            }
            let color = if dist >= -border_width { border } else { fill };
            let dst = canvas.get_pixel_mut(px, py);
            *dst = blend_pixel(*dst, color);
        }
    }
}

fn rounded_rect_sdf(x: i32, y: i32, width: i32, height: i32, radius: i32) -> i32 {
    let rx = if x < radius {
        radius - x
    } else if x >= width - radius {
        x - (width - radius - 1)
    } else {
        0
    };
    let ry = if y < radius {
        radius - y
    } else if y >= height - radius {
        y - (height - radius - 1)
    } else {
        0
    };

    if rx == 0 && ry == 0 {
        let edge_x = x.min(width - 1 - x);
        let edge_y = y.min(height - 1 - y);
        return -edge_x.min(edge_y);
    }

    let dist_sq = rx * rx + ry * ry;
    let r_sq = radius * radius;
    if dist_sq > r_sq {
        1
    } else if dist_sq >= (radius - 2).max(0).pow(2) {
        -1
    } else {
        -3
    }
}

fn overlay_rgba(canvas: &mut RgbaImage, foreground: &RgbaImage, offset_x: u32, offset_y: u32) {
    for (fx, fy, src) in foreground.enumerate_pixels() {
        if src[3] == 0 {
            continue;
        }
        let tx = offset_x + fx;
        let ty = offset_y + fy;
        if tx < canvas.width() && ty < canvas.height() {
            let dst = canvas.get_pixel_mut(tx, ty);
            *dst = blend_pixel(*dst, *src);
        }
    }
}

fn blend_pixel(dst: Rgba<u8>, src: Rgba<u8>) -> Rgba<u8> {
    let sa = f32::from(src[3]) / 255.0;
    let da = f32::from(dst[3]) / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.001 {
        return Rgba([0, 0, 0, 0]);
    }

    let r = (f32::from(src[0]) * sa + f32::from(dst[0]) * da * (1.0 - sa)) / out_a;
    let g = (f32::from(src[1]) * sa + f32::from(dst[1]) * da * (1.0 - sa)) / out_a;
    let b = (f32::from(src[2]) * sa + f32::from(dst[2]) * da * (1.0 - sa)) / out_a;

    Rgba([
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
        (out_a * 255.0).clamp(0.0, 255.0) as u8,
    ])
}

fn write_png_atomic(
    grid_dir: &Path,
    target_path: &Path,
    image: &RgbaImage,
) -> Result<(), ShortcutError> {
    let mut encoded = Vec::new();
    DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
        .map_err(|e| ShortcutError::Image(format!("Failed to encode grid PNG: {e}")))?;

    let mut staged = tempfile::NamedTempFile::new_in(grid_dir)?;
    staged.write_all(&encoded)?;
    staged.flush()?;
    staged.persist(target_path).map_err(|e| e.error)?;
    Ok(())
}
