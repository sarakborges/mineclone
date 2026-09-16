use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    voxel::coordinates::chunk_coord_from_position,
    world::render_distance::RenderDistanceSettings,
};

use super::chunk_rendering::ChunkRenderCoord;

const FOG_OCCLUDED_RENDER_MARGIN_CHUNKS: i32 = 2;

#[derive(Default)]
pub(super) struct ChunkVisibilityState {
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
    let horizontal_radius = rendered_chunk_radius(render_distance.chunks());
    if state.center == Some(center) && state.horizontal_radius == horizontal_radius {
        return;
    }

    state.center = Some(center);
    state.horizontal_radius = horizontal_radius;

    for (coord, mut visibility) in &mut chunks {
        apply_chunk_visibility(center, horizontal_radius, coord.0, &mut visibility);
    }
}

pub(super) fn sync_new_chunk_visibility(
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    mut chunks: Query<(&ChunkRenderCoord, &mut Visibility), Added<ChunkRenderCoord>>,
) {
    let center = chunk_coord_from_position(player.translation).xz();
    let horizontal_radius = rendered_chunk_radius(render_distance.chunks());

    for (coord, mut visibility) in &mut chunks {
        apply_chunk_visibility(center, horizontal_radius, coord.0, &mut visibility);
    }
}

fn rendered_chunk_radius(render_distance_chunks: i32) -> i32 {
    render_distance_chunks.saturating_add(FOG_OCCLUDED_RENDER_MARGIN_CHUNKS)
}

fn apply_chunk_visibility(
    center: IVec2,
    horizontal_radius: i32,
    coord: IVec3,
    visibility: &mut Visibility,
) {
    let target = if chunk_is_inside_visible_radius(center, coord, horizontal_radius) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *visibility != target {
        *visibility = target;
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
    fn rendered_shell_extends_two_chunks_beyond_nominal_distance() {
        let center = IVec2::ZERO;
        let rendered_radius = rendered_chunk_radius(12);

        assert_eq!(rendered_radius, 14);
        assert!(chunk_is_inside_visible_radius(
            center,
            IVec3::new(14, 0, 0),
            rendered_radius
        ));
        assert!(!chunk_is_inside_visible_radius(
            center,
            IVec3::new(15, 0, 0),
            rendered_radius
        ));
    }

    #[test]
    fn rendered_shell_keeps_diagonal_circle_membership() {
        let center = IVec2::ZERO;
        let rendered_radius = rendered_chunk_radius(12);

        assert!(chunk_is_inside_visible_radius(
            center,
            IVec3::new(9, 0, 10),
            rendered_radius
        ));
        assert!(!chunk_is_inside_visible_radius(
            center,
            IVec3::new(10, 0, 10),
            rendered_radius
        ));
    }

    #[test]
    fn vertical_distance_does_not_hide_surface_while_flying() {
        assert!(chunk_is_inside_visible_radius(
            IVec2::ZERO,
            IVec3::new(3, 99, 4),
            rendered_chunk_radius(4)
        ));
    }
}
