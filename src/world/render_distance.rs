use bevy::prelude::*;

pub const MIN_RENDER_DISTANCE_CHUNKS: i32 = 4;
pub const MAX_RENDER_DISTANCE_CHUNKS: i32 = 24;
pub const DEFAULT_RENDER_DISTANCE_CHUNKS: i32 = 12;

#[derive(Resource)]
pub struct RenderDistanceSettings {
    horizontal_chunks: i32,
}

impl Default for RenderDistanceSettings {
    fn default() -> Self {
        Self {
            horizontal_chunks: DEFAULT_RENDER_DISTANCE_CHUNKS,
        }
    }
}

impl RenderDistanceSettings {
    pub fn chunks(&self) -> i32 {
        self.horizontal_chunks
    }

    pub fn set_chunks(&mut self, chunks: i32) {
        self.horizontal_chunks =
            chunks.clamp(MIN_RENDER_DISTANCE_CHUNKS, MAX_RENDER_DISTANCE_CHUNKS);
    }
}

pub(crate) fn chunk_visibility_radii(render_distance_chunks: i32) -> (i32, i32) {
    let nominal_radius = render_distance_chunks.max(1);
    // The player can stand anywhere inside the center chunk, so one extra chunk
    // is enough to cover the nominal world-space radius without exposing a gap at
    // the boundary. Fog already reaches full opacity inside the nominal radius;
    // keeping a second visible guard ring only renders geometry the player cannot
    // see.
    let show_radius = nominal_radius.saturating_add(1);
    // Keep one chunk of hysteresis so crossing a chunk boundary does not churn
    // visibility/residency immediately.
    let hide_radius = show_radius.saturating_add(1);

    (show_radius, hide_radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_radii_scale_from_render_distance() {
        assert_eq!(chunk_visibility_radii(4), (5, 6));
        assert_eq!(chunk_visibility_radii(12), (13, 14));
        assert_eq!(chunk_visibility_radii(24), (25, 26));
    }
}
