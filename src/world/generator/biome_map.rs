use std::{collections::BTreeMap, path::Path};

use image::{Rgba, RgbaImage};

use super::BiomeQueries;

const MAX_MAP_RESOLUTION: u32 = 4096;

#[derive(Clone, Debug)]
pub(crate) struct BiomeMapConfig {
    pub(crate) center_x: i32,
    pub(crate) center_z: i32,
    pub(crate) blocks_per_pixel: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) show_boundaries: bool,
    pub(crate) show_influences: bool,
}

impl Default for BiomeMapConfig {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_z: 0,
            blocks_per_pixel: 4,
            width: 512,
            height: 512,
            show_boundaries: true,
            show_influences: true,
        }
    }
}

impl BiomeMapConfig {
    pub(crate) fn validate(&self) {
        assert!(self.blocks_per_pixel > 0, "biome map scale must be positive");
        assert!(
            self.width > 0 && self.height > 0,
            "biome map resolution must be non-zero"
        );
        assert!(
            self.width <= MAX_MAP_RESOLUTION && self.height <= MAX_MAP_RESOLUTION,
            "biome map resolution cannot exceed {MAX_MAP_RESOLUTION} pixels per axis"
        );
        let _ = sample_origin(
            self.center_x,
            self.width,
            self.blocks_per_pixel,
            "horizontal",
        );
        let _ = sample_origin(
            self.center_z,
            self.height,
            self.blocks_per_pixel,
            "vertical",
        );
    }
}

#[derive(Debug)]
pub(crate) struct BiomeMapRender {
    image: RgbaImage,
    legend_json: String,
}

impl BiomeMapRender {
    pub(crate) fn image(&self) -> &RgbaImage {
        &self.image
    }

    pub(crate) fn legend_json(&self) -> &str {
        &self.legend_json
    }

    pub(crate) fn save(&self, output: &Path) -> Result<(), String> {
        self.image
            .save(output)
            .map_err(|error| format!("failed to save biome map {}: {error}", output.display()))?;
        let legend_path = output.with_extension("legend.json");
        std::fs::write(&legend_path, &self.legend_json).map_err(|error| {
            format!(
                "failed to save biome map legend {}: {error}",
                legend_path.display()
            )
        })?;
        Ok(())
    }
}

pub(crate) fn render_biome_map(
    queries: BiomeQueries<'_>,
    config: &BiomeMapConfig,
) -> BiomeMapRender {
    config.validate();
    let origin_x = sample_origin(
        config.center_x,
        config.width,
        config.blocks_per_pixel,
        "horizontal",
    );
    let origin_z = sample_origin(
        config.center_z,
        config.height,
        config.blocks_per_pixel,
        "vertical",
    );
    let samples = queries.sample_surface_grid(
        origin_x,
        origin_z,
        config.width,
        config.height,
        config.blocks_per_pixel,
    );

    let region_ranges = queries
        .biome_region_ranges()
        .map(|(id, min, max)| (id.as_str().to_owned(), min, max))
        .collect::<Vec<_>>();
    let colors = region_ranges
        .iter()
        .map(|(id, _, _)| (id.clone(), stable_biome_color(id)))
        .collect::<BTreeMap<_, _>>();
    let mut image = RgbaImage::new(config.width, config.height);
    for py in 0..config.height {
        for px in 0..config.width {
            let sample = samples
                .sample_at(px, py)
                .expect("validated biome map sample must exist");
            let mut color = colors[sample.primary().as_str()];
            if config.show_influences {
                color = blend_toward_white(color, 1.0 - sample.primary_weight());
            }
            let left_boundary = px > 0
                && samples
                    .sample_at(px - 1, py)
                    .is_some_and(|left| left.primary() != sample.primary());
            let top_boundary = py > 0
                && samples
                    .sample_at(px, py - 1)
                    .is_some_and(|top| top.primary() != sample.primary());
            if config.show_boundaries && (left_boundary || top_boundary) {
                color = [18, 18, 18];
            }
            image.put_pixel(px, py, Rgba([color[0], color[1], color[2], 255]));
        }
    }

    let legend = region_ranges
        .iter()
        .map(|(id, min, max)| {
            serde_json::json!({
                "id": id,
                "rgb": colors[id],
                "regionSize": {
                    "min": min,
                    "max": max
                }
            })
        })
        .collect::<Vec<_>>();
    let suppressed = queries
        .suppressed_biomes()
        .iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>();
    let legend_json = serde_json::to_string_pretty(&serde_json::json!({
        "center": [config.center_x, config.center_z],
        "blocksPerPixel": config.blocks_per_pixel,
        "resolution": [config.width, config.height],
        "sampleOrigin": [origin_x, origin_z],
        "boundaries": config.show_boundaries,
        "influences": config.show_influences,
        "biomes": legend,
        "suppressedBiomes": suppressed,
    }))
    .expect("biome map legend must serialize");

    BiomeMapRender { image, legend_json }
}

fn sample_origin(center: i32, pixel_count: u32, blocks_per_pixel: u32, axis: &str) -> i32 {
    let span = i64::from(pixel_count)
        .checked_mul(i64::from(blocks_per_pixel))
        .expect("biome map span is too large");
    let first = i64::from(center) - span / 2 + i64::from(blocks_per_pixel) / 2;
    let last = first
        + i64::from(pixel_count - 1)
            .checked_mul(i64::from(blocks_per_pixel))
            .expect("biome map span is too large");
    assert!(
        first >= i64::from(i32::MIN) && last <= i64::from(i32::MAX),
        "biome map {axis} span exceeds world coordinate range"
    );
    first as i32
}

fn stable_biome_color(id: &str) -> [u8; 3] {
    let hash = hash_text(id);
    let hue = (hash % 360) as f64;
    let saturation = 0.56 + ((hash >> 9) & 0xff) as f64 / 255.0 * 0.22;
    let value = 0.70 + ((hash >> 17) & 0xff) as f64 / 255.0 * 0.20;
    hsv_to_rgb(hue, saturation, value)
}

fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let chroma = value * saturation;
    let segment = hue / 60.0;
    let x = chroma * (1.0 - ((segment % 2.0) - 1.0).abs());
    let (r, g, b) = match segment.floor() as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = value - chroma;
    [
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    ]
}

fn blend_toward_white(color: [u8; 3], transition_strength: f32) -> [u8; 3] {
    let amount = (transition_strength * 0.72).clamp(0.0, 0.72);
    color.map(|channel| {
        (f32::from(channel) + (255.0 - f32::from(channel)) * amount).round() as u8
    })
}

fn hash_text(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::biome::{BiomeDefinition, BiomeRegistry};
    use crate::world::generator::{
        WorldGenerator,
        foundation::{GenerationDimension, GenerationSeed, GenerationSnapshot},
    };

    fn generator(seed: u64) -> WorldGenerator {
        let mut registry = BiomeRegistry::default();
        for id in ["asteria:test/a", "asteria:test/b", "asteria:test/c"] {
            let definition: BiomeDefinition = serde_json::from_value(serde_json::json!({
                "id": id,
                "name": {
                    "english": id,
                    "portuguese_brazil": id,
                    "spanish": id
                },
                "regionSize": { "min": 128, "max": 256 }
            }))
            .expect("test biome must deserialize");
            registry.insert(definition);
        }
        WorldGenerator::from_snapshot(
            GenerationSnapshot::new(
                GenerationSeed::new(seed),
                GenerationDimension::new("asteria:test", 64, 1.0),
            ),
            &registry,
        )
    }

    #[test]
    fn rendered_map_is_deterministic_and_contains_legend() {
        let generator = generator(55);
        let config = BiomeMapConfig {
            center_x: 100,
            center_z: -200,
            blocks_per_pixel: 16,
            width: 32,
            height: 24,
            show_boundaries: true,
            show_influences: true,
        };
        let first = render_biome_map(generator.biomes(), &config);
        let second = render_biome_map(generator.biomes(), &config);
        assert_eq!(first.image().as_raw(), second.image().as_raw());
        assert_eq!(first.legend_json(), second.legend_json());
        assert!(first.legend_json().contains("asteria:test/a"));
        assert!(first.legend_json().contains("regionSize"));
    }

    #[test]
    fn stable_colors_do_not_depend_on_render_order() {
        assert_eq!(
            stable_biome_color("asteria:test/a"),
            stable_biome_color("asteria:test/a")
        );
        assert_ne!(
            stable_biome_color("asteria:test/a"),
            stable_biome_color("asteria:test/b")
        );
    }
}
