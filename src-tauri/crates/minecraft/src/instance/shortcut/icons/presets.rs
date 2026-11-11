//! Built-in 256x256 procedural preset icon rendering (`preset:<id>`).

use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

struct PresetStyle {
    bg_top: [u8; 3],
    bg_bottom: [u8; 3],
    accent: [u8; 3],
    pattern: [&'static str; 8],
}

/// Renders a built-in `preset:<id>` icon into a 256x256 PNG byte buffer.
pub fn render_preset_icon_png(preset_spec: &str) -> Option<Vec<u8>> {
    let preset_id = preset_spec
        .strip_prefix("preset:")?
        .trim()
        .to_ascii_lowercase();
    let style = preset_definition(&preset_id)?;

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
        let r = (f32::from(style.bg_top[0]) * (1.0 - t) + f32::from(style.bg_bottom[0]) * t) as u8;
        let g = (f32::from(style.bg_top[1]) * (1.0 - t) + f32::from(style.bg_bottom[1]) * t) as u8;
        let b = (f32::from(style.bg_top[2]) * (1.0 - t) + f32::from(style.bg_bottom[2]) * t) as u8;
        *pixel = Rgba([r, g, b, 255]);
    }

    // Render 8x8 symmetric pixel motif centered in the 256x256 badge (each cell = 20x20 px)
    let grid_origin = 48u32;
    let cell_size = 20u32;
    for (row_idx, row) in style.pattern.iter().enumerate() {
        for (col_idx, &ch) in row.as_bytes().iter().enumerate() {
            let color = match ch {
                b'#' => Some(Rgba([
                    style.accent[0],
                    style.accent[1],
                    style.accent[2],
                    255,
                ])),
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

fn preset_definition(id: &str) -> Option<PresetStyle> {
    match id {
        "creeper" => Some(PresetStyle {
            bg_top: [34, 139, 58],
            bg_bottom: [18, 78, 32],
            accent: [82, 214, 105],
            pattern: [
                "########", "#..##..#", "#..##..#", "###..###", "##....##", "##....##", "##.##.##",
                "########",
            ],
        }),
        "grass" => Some(PresetStyle {
            bg_top: [52, 168, 83],
            bg_bottom: [110, 71, 43],
            accent: [116, 224, 110],
            pattern: [
                "########", "########", "##.##.#.", "........", "........", "........", "........",
                "........",
            ],
        }),
        "sword" => Some(PresetStyle {
            bg_top: [18, 62, 89],
            bg_bottom: [12, 28, 46],
            accent: [56, 224, 248],
            pattern: [
                "      ##", "     ###", "    ### ", " + ###  ", "  +###  ", "  ++#   ", " +  +   ",
                "+       ",
            ],
        }),
        "pickaxe" => Some(PresetStyle {
            bg_top: [42, 48, 72],
            bg_bottom: [18, 22, 36],
            accent: [56, 189, 248],
            pattern: [
                " #####  ", "####### ", "##  +###", "   +  ##", "  +   # ", " +      ", "+       ",
                "        ",
            ],
        }),
        "redstone" => Some(PresetStyle {
            bg_top: [92, 18, 24],
            bg_bottom: [38, 10, 14],
            accent: [244, 63, 94],
            pattern: [
                "  ####  ", " ###### ", "###++###", "##++++##", "##++++##", "###++###", " ###### ",
                "  ####  ",
            ],
        }),
        "ender" => Some(PresetStyle {
            bg_top: [16, 68, 64],
            bg_bottom: [10, 28, 32],
            accent: [45, 212, 191],
            pattern: [
                "  ####  ", " ###### ", "##++++##", "#++..++#", "#++..++#", "##++++##", " ###### ",
                "  ####  ",
            ],
        }),
        "nether" => Some(PresetStyle {
            bg_top: [76, 29, 149],
            bg_bottom: [32, 12, 68],
            accent: [192, 132, 252],
            pattern: [
                "########", "#++++++#", "#+####+#", "#+#++#+#", "#+#++#+#", "#+####+#", "#++++++#",
                "########",
            ],
        }),
        "potion" => Some(PresetStyle {
            bg_top: [88, 28, 135],
            bg_bottom: [30, 16, 56],
            accent: [236, 72, 153],
            pattern: [
                "  ++++  ", "   ++   ", "  ####  ", " ###### ", "########", "###++###", "########",
                " ###### ",
            ],
        }),
        "command_block" => Some(PresetStyle {
            bg_top: [124, 58, 32],
            bg_bottom: [58, 26, 14],
            accent: [251, 146, 60],
            pattern: [
                "########", "#......#", "#.++++.#", "#.+##+.#", "#.+##+.#", "#.++++.#", "#......#",
                "########",
            ],
        }),
        "chest" => Some(PresetStyle {
            bg_top: [133, 77, 14],
            bg_bottom: [66, 36, 8],
            accent: [245, 158, 11],
            pattern: [
                "########", "########", "........", "###++###", "###++###", "########", "########",
                "########",
            ],
        }),
        "beacon" => Some(PresetStyle {
            bg_top: [14, 116, 144],
            bg_bottom: [12, 42, 64],
            accent: [103, 232, 249],
            pattern: [
                "  ++++  ", " ++++++ ", "++####++", "+######+", "+######+", "++####++", "........",
                "########",
            ],
        }),
        "book" => Some(PresetStyle {
            bg_top: [113, 63, 18],
            bg_bottom: [54, 28, 10],
            accent: [250, 204, 21],
            pattern: [
                " ###### ", "##++++##", "##+###+#", "##+++++#", "##+###+#", "##++++##", " ###### ",
                "        ",
            ],
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_builtin_presets_as_256px_png_and_normalizes_casing() {
        let preset_ids = [
            "creeper",
            "grass",
            "sword",
            "pickaxe",
            "redstone",
            "ender",
            "nether",
            "potion",
            "command_block",
            "chest",
            "beacon",
            "book",
        ];

        for id in preset_ids {
            let png = render_preset_icon_png(&format!("preset:{id}")).unwrap();
            let decoded = image::load_from_memory(&png).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (256, 256));
        }

        assert!(render_preset_icon_png("preset:  CREEPER ").is_some());
        assert!(render_preset_icon_png("creeper").is_none());
        assert!(render_preset_icon_png("preset:unknown").is_none());
    }
}
