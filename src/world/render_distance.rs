use bevy::prelude::*;

pub const MIN_RENDER_DISTANCE_CHUNKS: i32 = 4;
pub const MAX_RENDER_DISTANCE_CHUNKS: i32 = 8;
pub const DEFAULT_RENDER_DISTANCE_CHUNKS: i32 = 6;
pub const DEFAULT_VERTICAL_RENDER_DISTANCE_CHUNKS: i32 = 4;

#[derive(Resource)]
pub struct RenderDistanceSettings {
    horizontal_chunks: i32,
    vertical_chunks: i32,
}

impl Default for RenderDistanceSettings {
    fn default() -> Self {
        Self {
            horizontal_chunks: DEFAULT_RENDER_DISTANCE_CHUNKS,
            vertical_chunks: DEFAULT_VERTICAL_RENDER_DISTANCE_CHUNKS,
        }
    }
}

impl RenderDistanceSettings {
    pub fn chunks(&self) -> i32 {
        self.horizontal_chunks
    }

    pub fn vertical_chunks(&self) -> i32 {
        self.vertical_chunks
    }

    pub fn set_chunks(&mut self, chunks: i32) {
        self.horizontal_chunks =
            chunks.clamp(MIN_RENDER_DISTANCE_CHUNKS, MAX_RENDER_DISTANCE_CHUNKS);
    }
}

pub fn chunk_coords_in_volume(
    center: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
) -> Vec<IVec3> {
    assert!(center.y >= 0, "streaming center Y cannot be negative");
    assert!(
        horizontal_radius >= 0,
        "horizontal streaming radius cannot be negative"
    );
    assert!(
        vertical_radius >= 0,
        "vertical streaming radius cannot be negative"
    );

    let mut coords = Vec::new();
    let horizontal_radius_squared = horizontal_radius * horizontal_radius;
    let min_chunk_y = (center.y - vertical_radius).max(0);
    let max_chunk_y = center.y + vertical_radius;

    for y in min_chunk_y..=max_chunk_y {
        for z in -horizontal_radius..=horizontal_radius {
            for x in -horizontal_radius..=horizontal_radius {
                if x * x + z * z > horizontal_radius_squared {
                    continue;
                }

                coords.push(IVec3::new(center.x + x, y, center.z + z));
            }
        }
    }

    coords.sort_by_key(|coord| {
        let delta = *coord - center;
        delta.x * delta.x + delta.y * delta.y + delta.z * delta.z
    });

    coords
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_volume_never_crosses_below_zero() {
        let coords = chunk_coords_in_volume(IVec3::ZERO, 1, 4);

        assert!(coords.iter().all(|coord| coord.y >= 0));
    }

    #[test]
    fn streaming_volume_respects_vertical_radius() {
        let center = IVec3::new(3, 10, -2);
        let coords = chunk_coords_in_volume(center, 2, 3);

        assert!(coords.iter().all(|coord| (coord.y - center.y).abs() <= 3));
    }
}
