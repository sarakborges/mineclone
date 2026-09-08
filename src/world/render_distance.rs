use bevy::prelude::*;

pub const MIN_RENDER_DISTANCE_CHUNKS: i32 = 4;
pub const MAX_RENDER_DISTANCE_CHUNKS: i32 = 8;
pub const DEFAULT_RENDER_DISTANCE_CHUNKS: i32 = 6;

#[derive(Resource)]
pub struct RenderDistanceSettings {
    chunks: i32,
}

impl Default for RenderDistanceSettings {
    fn default() -> Self {
        Self {
            chunks: DEFAULT_RENDER_DISTANCE_CHUNKS,
        }
    }
}

impl RenderDistanceSettings {
    pub fn chunks(&self) -> i32 {
        self.chunks
    }

    pub fn set_chunks(&mut self, chunks: i32) {
        self.chunks = chunks.clamp(MIN_RENDER_DISTANCE_CHUNKS, MAX_RENDER_DISTANCE_CHUNKS);
    }
}

pub fn chunk_coords_in_cylinder(
    center: IVec3,
    horizontal_radius: i32,
    min_chunk_y: i32,
    max_chunk_y: i32,
) -> Vec<IVec3> {
    assert!(min_chunk_y >= 0, "minimum chunk Y cannot be negative");
    assert!(
        min_chunk_y <= max_chunk_y,
        "minimum chunk Y must be less than or equal to maximum chunk Y"
    );

    let mut coords = Vec::new();
    let radius_squared = horizontal_radius * horizontal_radius;

    for y in (min_chunk_y..=max_chunk_y).rev() {
        for z in -horizontal_radius..=horizontal_radius {
            for x in -horizontal_radius..=horizontal_radius {
                if x * x + z * z <= radius_squared {
                    coords.push(IVec3::new(center.x + x, y, center.z + z));
                }
            }
        }
    }

    coords
}
