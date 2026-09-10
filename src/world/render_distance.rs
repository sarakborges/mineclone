use bevy::prelude::*;

pub const MIN_RENDER_DISTANCE_CHUNKS: i32 = 4;
pub const MAX_RENDER_DISTANCE_CHUNKS: i32 = 16;
pub const DEFAULT_RENDER_DISTANCE_CHUNKS: i32 = 10;
pub const DEFAULT_VERTICAL_RENDER_DISTANCE_CHUNKS: i32 = 2;

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
    let min_chunk_y = (center.y - vertical_radius).max(0);
    let max_chunk_y = center.y + vertical_radius;

    for y in min_chunk_y..=max_chunk_y {
        for z in -horizontal_radius..=horizontal_radius {
            for x in -horizontal_radius..=horizontal_radius {
                let coord = IVec3::new(center.x + x, y, center.z + z);

                if chunk_is_in_volume(center, coord, horizontal_radius, vertical_radius) {
                    coords.push(coord);
                }
            }
        }
    }

    coords.sort_by_key(|coord| (*coord - center).length_squared());
    coords
}

pub(crate) fn chunk_is_in_volume(
    center: IVec3,
    coord: IVec3,
    horizontal_radius: i32,
    vertical_radius: i32,
) -> bool {
    if coord.y < 0 || horizontal_radius < 0 || vertical_radius < 0 {
        return false;
    }

    let delta = coord - center;
    let horizontal_squared = delta.x * delta.x + delta.z * delta.z;

    match (horizontal_radius, vertical_radius) {
        (0, 0) => horizontal_squared == 0 && delta.y == 0,
        (0, _) => horizontal_squared == 0 && delta.y.abs() <= vertical_radius,
        (_, 0) => delta.y == 0 && horizontal_squared <= horizontal_radius * horizontal_radius,
        _ => {
            let horizontal_radius_squared = horizontal_radius * horizontal_radius;
            let vertical_offset = delta.y.abs();

            horizontal_squared * vertical_radius + vertical_offset * horizontal_radius_squared
                <= horizontal_radius_squared * vertical_radius
        }
    }
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

    #[test]
    fn vertical_layers_taper_toward_the_extremes() {
        let center = IVec3::new(0, 8, 0);
        let coords = chunk_coords_in_volume(center, 6, 2);
        let center_layer = coords.iter().filter(|coord| coord.y == center.y).count();
        let adjacent_layer = coords
            .iter()
            .filter(|coord| coord.y == center.y + 1)
            .count();
        let top_layer = coords
            .iter()
            .filter(|coord| coord.y == center.y + 2)
            .count();

        assert!(adjacent_layer < center_layer);
        assert!(top_layer < adjacent_layer);
    }

    #[test]
    fn membership_matches_generated_volume() {
        let center = IVec3::new(2, 5, -3);
        let coords = chunk_coords_in_volume(center, 4, 2);

        assert!(
            coords
                .iter()
                .all(|coord| chunk_is_in_volume(center, *coord, 4, 2))
        );
        assert!(!chunk_is_in_volume(
            center,
            center + IVec3::new(4, 2, 0),
            4,
            2
        ));
    }
}
