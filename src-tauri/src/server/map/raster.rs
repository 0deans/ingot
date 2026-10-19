use super::models::{MapError, TILE};

/// One region's map before shading and encoding
#[derive(Clone)]
pub struct Raster {
    /// RGBA, alpha 0 where there's no data
    pub pixels: Vec<u8>,
    /// Surface height per pixel, for relief shading; i32::MIN = no data
    pub heights: Vec<i32>,
    pub is_water: Vec<bool>,
    /// When each chunk's data was taken (unix seconds); 0 = no data
    pub chunk_times: Vec<u32>,
}

impl Raster {
    pub fn empty() -> Self {
        Raster {
            pixels: vec![0; TILE * TILE * 4],
            heights: vec![i32::MIN; TILE * TILE],
            is_water: vec![false; TILE * TILE],
            chunk_times: vec![0; 1024],
        }
    }

    pub fn set(&mut self, px: usize, pz: usize, surface: Option<([u8; 3], i32, bool)>) {
        let i = pz * TILE + px;
        match surface {
            Some((color, height, water)) => {
                self.pixels[i * 4..i * 4 + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
                self.heights[i] = height;
                self.is_water[i] = water;
            }
            None => {
                self.pixels[i * 4..i * 4 + 4].fill(0);
                self.heights[i] = i32::MIN;
                self.is_water[i] = false;
            }
        }
    }

    /// Relief shading (lighter uphill from the north, darker downhill), then PNG
    pub fn to_png(&self) -> Result<Vec<u8>, MapError> {
        let mut pixels = self.pixels.clone();
        for pz in 1..TILE {
            for px in 0..TILE {
                let i = pz * TILE + px;
                let north = self.heights[i - TILE];
                if self.heights[i] == i32::MIN || north == i32::MIN || self.is_water[i] {
                    continue;
                }
                let factor = match self.heights[i].cmp(&north) {
                    std::cmp::Ordering::Greater => 1.12,
                    std::cmp::Ordering::Less => 0.84,
                    std::cmp::Ordering::Equal => 1.0,
                };
                for c in &mut pixels[i * 4..i * 4 + 3] {
                    *c = (*c as f32 * factor).min(255.0) as u8;
                }
            }
        }
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, TILE as u32, TILE as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_compression(png::Compression::Fast);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&pixels)?;
        }
        Ok(out)
    }
}
