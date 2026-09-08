use bevy::prelude::*;

pub const MIN_RENDER_DISTANCE_CHUNKS: i32 = 4;
pub const MAX_RENDER_DISTANCE_CHUNKS: i32 = 24;
pub const DEFAULT_RENDER_DISTANCE_CHUNKS: i32 = 16;

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

pub fn chunk_coords_in_radius(center: IVec2, radius: i32) -> Vec<IVec2> {
    let mut coords = Vec::new();
    let radius_squared = radius * radius;

    for z in -radius..=radius {
        for x in -radius..=radius {
            if x * x + z * z <= radius_squared {
                coords.push(center + IVec2::new(x, z));
            }
        }
    }

    coords
}
