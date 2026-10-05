use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};

use super::TerrainQueries;

const MAX_DEBUG_RESOLUTION: u32 = 4096;

#[derive(Clone, Debug)]
pub(crate) struct TerrainDebugConfig {
    pub(crate) center_x: i32,
    pub(crate) center_y: i32,
    pub(crate) center_z: i32,
    pub(crate) blocks_per_pixel: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Default for TerrainDebugConfig {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_y: 128,
            center_z: 0,
            blocks_per_pixel: 2,
            width: 512,
            height: 256,
        }
    }
}

impl TerrainDebugConfig {
    fn validate(&self) {
        assert!(
            self.blocks_per_pixel > 0,
            "terrain debug scale must be positive"
        );
        assert!(
            self.width > 0 && self.height > 0,
            "terrain debug resolution must be non-zero"
        );
        assert!(
            self.width <= MAX_DEBUG_RESOLUTION && self.height <= MAX_DEBUG_RESOLUTION,
            "terrain debug resolution cannot exceed {MAX_DEBUG_RESOLUTION} pixels per axis"
        );
        let _ = sample_origin(
            self.center_x,
            self.width,
            self.blocks_per_pixel,
            "X",
        );
        let _ = sample_origin(
            self.center_y,
            self.height,
            self.blocks_per_pixel,
            "Y",
        );
        let _ = sample_origin(
            self.center_z,
            self.height,
            self.blocks_per_pixel,
            "Z",
        );
    }
}

#[derive(Debug)]
pub(crate) struct TerrainDebugRender {
    surface: RgbaImage,
    density_slice: RgbaImage,
    metadata_json: String,
}

impl TerrainDebugRender {
    pub(crate) fn save(&self, output: &Path) -> Result<(), String> {
        self.surface.save(output).map_err(|error| {
            format!(
                "failed to save terrain surface debug {}: {error}",
                output.display()
            )
        })?;

        let density_path = companion_path(output, "-density");
        self.density_slice.save(&density_path).map_err(|error| {
            format!(
                "failed to save terrain density debug {}: {error}",
                density_path.display()
            )
        })?;

        let metadata_path = output.with_extension("debug.json");
        std::fs::write(&metadata_path, &self.metadata_json).map_err(|error| {
            format!(
                "failed to save terrain debug metadata {}: {error}",
                metadata_path.display()
            )
        })?;
        Ok(())
    }
}

pub(crate) fn render_terrain_debug(
    queries: TerrainQueries<'_>,
    config: &TerrainDebugConfig,
) -> TerrainDebugRender {
    config.validate();

    let origin_x = sample_origin(
        config.center_x,
        config.width,
        config.blocks_per_pixel,
        "X",
    );
    let origin_y = sample_origin(
        config.center_y,
        config.height,
        config.blocks_per_pixel,
        "Y",
    );
    let origin_z = sample_origin(
        config.center_z,
        config.height,
        config.blocks_per_pixel,
        "Z",
    );

    let surface_samples = queries.sample_surface_grid(
        origin_x,
        origin_z,
        config.width,
        config.height,
        config.blocks_per_pixel,
    );
    let mut surface = RgbaImage::new(config.width, config.height);
    let mut min_base_surface = f32::INFINITY;
    let mut max_base_surface = f32::NEG_INFINITY;
    let mut min_effective_surface = i32::MAX;
    let mut max_effective_surface = i32::MIN;
    let mut additive_columns = 0_u64;

    for pz in 0..config.height {
        for px in 0..config.width {
            let sample = surface_samples
                .sample_at(px, pz)
                .expect("validated terrain debug surface sample must exist");
            let base_y = floor_to_world_y(sample.base_surface());
            let effective_y = sample.surface_y();
            min_base_surface = min_base_surface.min(sample.base_surface());
            max_base_surface = max_base_surface.max(sample.base_surface());
            min_effective_surface = min_effective_surface.min(effective_y);
            max_effective_surface = max_effective_surface.max(effective_y);
            if effective_y > base_y {
                additive_columns += 1;
            }
            surface.put_pixel(
                px,
                pz,
                Rgba(surface_color(base_y, effective_y, config.center_y)),
            );
        }
    }

    let mut density_slice = RgbaImage::new(config.width, config.height);
    let mut solid_pixels = 0_u64;
    let mut empty_pixels = 0_u64;
    let mut carved_pixels = 0_u64;
    let mut additive_pixels = 0_u64;

    for py in 0..config.height {
        let y_index = config.height - 1 - py;
        let y = grid_axis(origin_y, y_index, config.blocks_per_pixel);
        for px in 0..config.width {
            let x = grid_axis(origin_x, px, config.blocks_per_pixel);
            let base_surface = queries.base_surface_at(x, config.center_z);
            let base_y = floor_to_world_y(base_surface);
            let effective_y = queries.surface_at(x, config.center_z);
            let density = queries.density_at(x, y, config.center_z);
            let base_density = base_surface - y as f32;

            let color = if sample_band_contains(y, config.blocks_per_pixel, effective_y)
                && effective_y != base_y
            {
                [24, 230, 236, 255]
            } else if sample_band_contains(y, config.blocks_per_pixel, base_y) {
                [244, 164, 54, 255]
            } else if density >= 0.0 && base_density < 0.0 {
                additive_pixels += 1;
                solid_pixels += 1;
                [40, 176, 210, 255]
            } else if density < 0.0 && base_density >= 0.0 {
                carved_pixels += 1;
                empty_pixels += 1;
                [150, 82, 205, 255]
            } else if density >= 0.0 {
                solid_pixels += 1;
                let shade = (70.0 + density.clamp(0.0, 48.0) * 2.6)
                    .round()
                    .clamp(70.0, 194.0) as u8;
                [shade, shade, shade, 255]
            } else {
                empty_pixels += 1;
                [230, 236, 244, 255]
            };
            density_slice.put_pixel(px, py, Rgba(color));
        }
    }

    let metadata_json = serde_json::to_string_pretty(&serde_json::json!({
        "center": [config.center_x, config.center_y, config.center_z],
        "blocksPerPixel": config.blocks_per_pixel,
        "resolution": [config.width, config.height],
        "surfaceMap": {
            "sampleOrigin": [origin_x, origin_z],
            "baseSurfaceRange": [min_base_surface, max_base_surface],
            "effectiveSurfaceRange": [min_effective_surface, max_effective_surface],
            "additiveColumns": additive_columns,
            "legend": {
                "baseOnly": "grayscale by effective world Y relative to centerY",
                "additiveSurface": [36, 184, 216]
            }
        },
        "densitySlice": {
            "fixedZ": config.center_z,
            "sampleOrigin": [origin_x, origin_y],
            "solidPixels": solid_pixels,
            "emptyPixels": empty_pixels,
            "carvedPixels": carved_pixels,
            "additivePixels": additive_pixels,
            "legend": {
                "baseSolid": "grayscale by positive final density",
                "empty": [230, 236, 244],
                "caveCarve": [150, 82, 205],
                "additiveMass": [40, 176, 210],
                "baseSurfaceCrossing": [244, 164, 54],
                "effectiveAdditiveCrossing": [24, 230, 236]
            }
        }
    }))
    .expect("terrain debug metadata must serialize");

    TerrainDebugRender {
        surface,
        density_slice,
        metadata_json,
    }
}

fn surface_color(base_y: i32, effective_y: i32, center_y: i32) -> [u8; 4] {
    if effective_y > base_y {
        return [36, 184, 216, 255];
    }
    let relative = effective_y.saturating_sub(center_y).clamp(-96, 96);
    let shade = (128 + relative) as u8;
    [shade, shade, shade, 255]
}

fn sample_band_contains(sample_y: i32, step: u32, target_y: i32) -> bool {
    let half = i64::from(step / 2);
    let low = i64::from(sample_y) - half;
    let high = low + i64::from(step) - 1;
    (low..=high).contains(&i64::from(target_y))
}

fn sample_origin(center: i32, pixel_count: u32, blocks_per_pixel: u32, axis: &str) -> i32 {
    let span = i64::from(pixel_count)
        .checked_mul(i64::from(blocks_per_pixel))
        .expect("terrain debug span is too large");
    let first = i64::from(center) - span / 2 + i64::from(blocks_per_pixel) / 2;
    let last = first
        + i64::from(pixel_count - 1)
            .checked_mul(i64::from(blocks_per_pixel))
            .expect("terrain debug span is too large");
    assert!(
        first >= i64::from(i32::MIN) && last <= i64::from(i32::MAX),
        "terrain debug {axis} span exceeds world coordinate range"
    );
    first as i32
}

fn grid_axis(origin: i32, index: u32, step: u32) -> i32 {
    let offset = i64::from(index) * i64::from(step);
    i32::try_from(i64::from(origin) + offset)
        .expect("validated terrain debug coordinate must fit i32")
}

fn floor_to_world_y(value: f32) -> i32 {
    if value <= i32::MIN as f32 {
        i32::MIN
    } else if value >= i32::MAX as f32 {
        i32::MAX
    } else {
        value.floor() as i32
    }
}

fn companion_path(output: &Path, suffix: &str) -> PathBuf {
    let parent = output.parent().unwrap_or_else(|| Path::new(""));
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("terrain-debug");
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    parent.join(format!("{stem}{suffix}.{extension}"))
}
