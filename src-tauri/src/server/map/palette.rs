use super::models::{mix, Kind};

const DYES: [(&str, [u8; 3]); 16] = [
    ("light_blue", [58, 175, 217]),
    ("light_gray", [142, 142, 134]),
    ("white", [233, 236, 236]),
    ("orange", [240, 118, 19]),
    ("magenta", [189, 68, 179]),
    ("yellow", [248, 198, 39]),
    ("lime", [112, 185, 25]),
    ("pink", [237, 141, 172]),
    ("gray", [62, 68, 71]),
    ("cyan", [21, 137, 145]),
    ("purple", [121, 42, 172]),
    ("blue", [53, 57, 157]),
    ("brown", [114, 71, 40]),
    ("green", [84, 109, 27]),
    ("red", [161, 39, 34]),
    ("black", [20, 21, 25]),
];

const WOODS: [(&str, [u8; 3]); 12] = [
    ("dark_oak", [66, 43, 20]),
    ("pale_oak", [228, 218, 216]),
    ("oak", [162, 130, 78]),
    ("spruce", [114, 84, 48]),
    ("birch", [192, 175, 121]),
    ("jungle", [160, 115, 80]),
    ("acacia", [168, 90, 50]),
    ("mangrove", [117, 54, 48]),
    ("cherry", [226, 178, 172]),
    ("bamboo", [193, 173, 80]),
    ("crimson", [101, 48, 70]),
    ("warped", [43, 104, 99]),
];

/// Maps a block id to how the map shows it
pub fn classify(id: &str) -> Kind {
    let name = id.strip_prefix("minecraft:").unwrap_or(id);
    let has = |s: &str| name.contains(s);

    // Look-through blocks: air, glass, small decorations and plants
    const CLEAR: [&str; 23] = [
        "air",
        "glass",
        "pane",
        "barrier",
        "structure_void",
        "torch",
        "rail",
        "button",
        "pressure_plate",
        "sign",
        "tripwire",
        "string",
        "lever",
        "ladder",
        "redstone_wire",
        "cobweb",
        "vine",
        "vines",
        "short_grass",
        "tall_grass",
        "fern",
        "dead_bush",
        "iron_bars",
    ];
    if name == "light" || CLEAR.iter().any(|c| name.ends_with(c)) && !has("block") {
        return Kind::Clear;
    }
    if has("water") || has("seagrass") || has("kelp") || name == "bubble_column" {
        return Kind::Water;
    }

    let c = |r: u8, g: u8, b: u8| Kind::Solid([r, g, b]);
    // Exact / specific matches first
    match name {
        "grass_block" => return c(109, 153, 60),
        "lava" => return c(207, 92, 20),
        "snow" | "snow_block" | "powder_snow" => return c(245, 250, 252),
        "ice" | "frosted_ice" => return c(160, 190, 245),
        "packed_ice" | "blue_ice" => return c(130, 165, 235),
        "sand" | "sandstone" | "cut_sandstone" | "smooth_sandstone" | "chiseled_sandstone" => {
            return c(219, 207, 163)
        }
        "suspicious_sand" => return c(210, 196, 150),
        "red_sand" | "red_sandstone" | "cut_red_sandstone" | "smooth_red_sandstone" => {
            return c(190, 103, 33)
        }
        "gravel" | "suspicious_gravel" => return c(136, 126, 126),
        "clay" => return c(160, 166, 179),
        "dirt" | "rooted_dirt" => return c(134, 96, 67),
        "coarse_dirt" => return c(119, 85, 59),
        "podzol" => return c(91, 63, 24),
        "mycelium" => return c(111, 99, 105),
        "farmland" => return c(118, 80, 50),
        "dirt_path" => return c(148, 122, 65),
        "mud" => return c(60, 57, 60),
        "moss_block" | "moss_carpet" => return c(89, 109, 45),
        "pale_moss_block" | "pale_moss_carpet" => return c(107, 112, 101),
        "netherrack" => return c(111, 54, 52),
        "soul_sand" | "soul_soil" => return c(81, 62, 50),
        "crimson_nylium" => return c(130, 31, 31),
        "warped_nylium" => return c(43, 114, 101),
        "nether_wart_block" => return c(115, 3, 2),
        "warped_wart_block" => return c(22, 119, 121),
        "glowstone" | "shroomlight" => return c(207, 160, 90),
        "magma_block" => return c(142, 63, 31),
        "end_stone" | "end_stone_bricks" => return c(219, 222, 158),
        "obsidian" | "crying_obsidian" => return c(20, 18, 30),
        "bedrock" => return c(85, 85, 85),
        "cactus" => return c(85, 127, 43),
        "pumpkin" | "carved_pumpkin" | "jack_o_lantern" => return c(198, 118, 24),
        "melon" => return c(111, 145, 30),
        "hay_block" => return c(166, 139, 12),
        "lily_pad" => return c(32, 128, 48),
        "sugar_cane" | "bamboo" => return c(120, 170, 70),
        "calcite" => return c(223, 224, 220),
        "tuff" => return c(108, 109, 102),
        "dripstone_block" | "pointed_dripstone" => return c(134, 107, 92),
        "prismarine" | "prismarine_bricks" | "dark_prismarine" => return c(76, 150, 140),
        "sculk" => return c(12, 29, 36),
        "amethyst_block" => return c(133, 97, 191),
        "terracotta" => return c(152, 94, 67),
        _ => {}
    }

    if has("leaves") {
        return match () {
            _ if has("birch") => c(96, 130, 60),
            _ if has("spruce") => c(60, 90, 60),
            _ if has("cherry") => c(229, 172, 194),
            _ if has("azalea") => c(90, 125, 50),
            _ if has("pale_oak") => c(140, 150, 140),
            _ if has("mangrove") => c(80, 120, 40),
            _ => c(56, 105, 38),
        };
    }
    // Dyed blocks: wool, concrete, carpets, beds, stained terracotta...
    if let Some((_, dye)) = DYES.iter().find(|(d, _)| name.starts_with(d)) {
        return if has("terracotta") {
            Kind::Solid(mix(*dye, [152, 94, 67], 0.55))
        } else {
            Kind::Solid(*dye)
        };
    }
    if let Some((_, wood)) = WOODS.iter().find(|(w, _)| name.starts_with(w)) {
        return Kind::Solid(*wood);
    }
    if has("deepslate") || has("blackstone") || has("basalt") {
        return c(70, 70, 76);
    }
    if has("copper") {
        return if has("oxidized") {
            c(82, 162, 132)
        } else if has("weathered") {
            c(108, 153, 110)
        } else if has("exposed") {
            c(161, 125, 103)
        } else {
            c(192, 107, 79)
        };
    }
    if has("quartz") || has("bone_block") {
        return c(235, 229, 222);
    }
    if has("brick") && !has("stone") {
        return c(150, 97, 83);
    }
    if has("mushroom") {
        return c(160, 80, 60);
    }
    if has("flower")
        || has("tulip")
        || has("orchid")
        || has("sapling")
        || has("bush")
        || has("roots")
    {
        return c(90, 140, 50);
    }
    if has("granite") {
        return c(149, 103, 85);
    }
    if has("diorite") {
        return c(188, 188, 188);
    }
    if has("andesite") {
        return c(136, 136, 136);
    }
    // Stone and everything else stone-like
    c(125, 125, 125)
}

/// The visible surface from blocks listed top-down: (color, height, is_water).
/// With `ceiling`, skips down to the first open space first (the Nether roof).
pub fn surface_of(
    blocks: impl Iterator<Item = (i32, Kind)>,
    ceiling: bool,
) -> Option<([u8; 3], i32, bool)> {
    const WATER: [u8; 3] = [52, 94, 196];
    let mut searching_for_air = ceiling;
    let mut water_top: Option<i32> = None;
    for (y, kind) in blocks {
        if searching_for_air {
            if kind == Kind::Clear {
                searching_for_air = false;
            }
            continue;
        }
        match kind {
            Kind::Clear => {}
            Kind::Water => {
                water_top.get_or_insert(y);
            }
            Kind::Solid(color) => {
                return Some(match water_top {
                    // Blend the lake/sea floor into the water by depth
                    Some(top) => {
                        let depth = (top - y).clamp(1, 24) as f32;
                        let t = (0.45 + depth / 24.0 * 0.5).min(0.95);
                        (mix(color, WATER, t), top, true)
                    }
                    None => (color, y, false),
                });
            }
        }
    }
    water_top.map(|top| (mix([20, 30, 60], WATER, 0.9), top, true))
}
