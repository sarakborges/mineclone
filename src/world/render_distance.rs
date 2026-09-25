use bevy::prelude::*;

pub const MIN_RENDER_DISTANCE_CHUNKS: i32 = 4;
pub const MAX_RENDER_DISTANCE_CHUNKS: i32 = 24;
pub const DEFAULT_RENDER_DISTANCE_CHUNKS: i32 = 12;
pub const DEFAULT_VERTICAL_RENDER_DISTANCE_CHUNKS: i32 = 2;

const MAX_VISIBILITY_SHOW_MARGIN_CHUNKS: i32 = 2;

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

pub(crate) fn chunk_visibility_radii(render_distance_chunks: i32) -> (i32, i32) {
    let nominal_radius = render_distance_chunks.max(1);
    let proportional_margin = ((nominal_radius + 5) / 6).max(1);
    let show_margin = proportional_margin.min(MAX_VISIBILITY_SHOW_MARGIN_CHUNKS);
    let show_radius = nominal_radius.saturating_add(show_margin);
    // Visibility hysteresis is deliberately fixed at one chunk. Scaling it with
    // render distance keeps thousands of invisible chunk meshes resident at high
    // settings without extending what the player can actually see.
    let hide_radius = show_radius.saturating_add(1);

    (show_radius, hide_radius)
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

    let delta_x = i64::from(coord.x) - i64::from(center.x);
    let delta_y = i64::from(coord.y) - i64::from(center.y);
    let delta_z = i64::from(coord.z) - i64::from(center.z);
    let horizontal_squared = delta_x * delta_x + delta_z * delta_z;
    let horizontal_radius = i64::from(horizontal_radius);

    horizontal_squared <= horizontal_radius * horizontal_radius
        && delta_y.abs() <= i64::from(vertical_radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_radii_scale_from_render_distance() {
        assert_eq!(chunk_visibility_radii(4), (5, 6));
        assert_eq!(chunk_visibility_radii(12), (14, 15));
        assert_eq!(chunk_visibility_radii(24), (26, 27));
    }

    #[test]
    fn volume_membership_respects_horizontal_and_vertical_bounds() {
        let center = IVec3::new(2, 5, -3);

        assert!(chunk_is_in_volume(
            center,
            center + IVec3::new(4, 2, 0),
            4,
            2
        ));
        assert!(!chunk_is_in_volume(
            center,
            center + IVec3::new(5, 0, 0),
            4,
            2
        ));
        assert!(!chunk_is_in_volume(
            center,
            center + IVec3::new(0, 3, 0),
            4,
            2
        ));
        assert!(!chunk_is_in_volume(
            center,
            IVec3::new(center.x, -1, center.z),
            4,
            2
        ));
    }
}
