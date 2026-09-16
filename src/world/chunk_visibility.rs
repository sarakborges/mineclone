use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    voxel::coordinates::chunk_coord_from_position,
    world::render_distance::RenderDistanceSettings,
};

use super::chunk_rendering::ChunkRenderCoord;

#[derive(Default)]
struct ChunkVisibilityState {
    center: Option<IVec2>,
    horizontal_radius: i32,
}

pub(super) fn sync_chunk_visibility(
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    mut chunks: Query<(&ChunkRenderCoord, &mut Visibility)>,
    mut state: Local<ChunkVisibilityState>,
) {
    let center = chunk_coord_from_position(player.translation).xz();
    let horizontal_radius = render_distance.chunks();
    if state.center == Some(center) && state.horizontal_radius == horizontal_radius {
        return;
    }

    state.center = Some(center);
    state.horizontal_radius = horizontal_radius;

    for (coord, mut visibility) in &mut chunks {
        let target = if chunk_is_inside_visible_radius(center, coord.0, horizontal_radius) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != target {
            *visibility = target;
        }
    }
}

fn chunk_is_inside_visible_radius(center: IVec2, coord: IVec3, horizontal_radius: i32) -> bool {
    if horizontal_radius < 0 {
        return false;
    }

    let delta = coord.xz() - center;
    delta.length_squared() <= horizontal_radius * horizontal_radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_radius_matches_horizontal_render_circle() {
        let center = IVec2::ZERO;

        assert!(chunk_is_inside_visible_radius(
            center,
            IVec3::new(12, 0, 0),
            12
        ));
        assert!(!chunk_is_inside_visible_radius(
            center,
            IVec3::new(13, 0, 0),
            12
        ));
        assert!(chunk_is_inside_visible_radius(
            center,
            IVec3::new(8, 0, 8),
            12
        ));
    }

    #[test]
    fn vertical_distance_does_not_hide_surface_while_flying() {
        assert!(chunk_is_inside_visible_radius(
            IVec2::ZERO,
            IVec3::new(3, 99, 4),
            5
        ));
    }
}
