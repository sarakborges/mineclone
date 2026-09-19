use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    voxel::coordinates::chunk_coord_from_position,
    world::render_distance::{RenderDistanceSettings, chunk_visibility_radii},
};

use super::chunk_rendering::ChunkRenderCoord;

#[derive(Default)]
pub(super) struct ChunkVisibilityState {
    center: Option<IVec2>,
    show_radius: i32,
    hide_radius: i32,
}

pub(super) fn sync_chunk_visibility(
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    mut chunks: Query<(&ChunkRenderCoord, &mut Visibility)>,
    mut state: Local<ChunkVisibilityState>,
) {
    let center = chunk_coord_from_position(player.translation).xz();
    let (show_radius, hide_radius) = chunk_visibility_radii(render_distance.chunks());
    if state.center == Some(center)
        && state.show_radius == show_radius
        && state.hide_radius == hide_radius
    {
        return;
    }

    state.center = Some(center);
    state.show_radius = show_radius;
    state.hide_radius = hide_radius;

    for (coord, mut visibility) in &mut chunks {
        apply_chunk_visibility(center, show_radius, hide_radius, coord.0, &mut visibility);
    }
}

pub(super) fn sync_new_chunk_visibility(
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    mut chunks: Query<(&ChunkRenderCoord, &mut Visibility), Added<ChunkRenderCoord>>,
) {
    let center = chunk_coord_from_position(player.translation).xz();
    let (show_radius, hide_radius) = chunk_visibility_radii(render_distance.chunks());

    for (coord, mut visibility) in &mut chunks {
        apply_chunk_visibility(center, show_radius, hide_radius, coord.0, &mut visibility);
    }
}

fn apply_chunk_visibility(
    center: IVec2,
    show_radius: i32,
    hide_radius: i32,
    coord: IVec3,
    visibility: &mut Visibility,
) {
    let threshold = if *visibility == Visibility::Visible {
        hide_radius
    } else {
        show_radius
    };
    let target = if chunk_is_inside_visible_radius(center, coord, threshold) {
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
    fn hidden_chunk_only_enters_inside_show_radius() {
        let center = IVec2::ZERO;
        let (show_radius, hide_radius) = chunk_visibility_radii(12);
        let mut visibility = Visibility::Hidden;

        apply_chunk_visibility(
            center,
            show_radius,
            hide_radius,
            IVec3::new(15, 0, 0),
            &mut visibility,
        );
        assert_eq!(visibility, Visibility::Hidden);

        apply_chunk_visibility(
            center,
            show_radius,
            hide_radius,
            IVec3::new(14, 0, 0),
            &mut visibility,
        );
        assert_eq!(visibility, Visibility::Visible);
    }

    #[test]
    fn visible_chunk_stays_visible_through_hysteresis_band() {
        let center = IVec2::ZERO;
        let (show_radius, hide_radius) = chunk_visibility_radii(12);
        let mut visibility = Visibility::Visible;

        apply_chunk_visibility(
            center,
            show_radius,
            hide_radius,
            IVec3::new(15, 0, 0),
            &mut visibility,
        );
        assert_eq!(visibility, Visibility::Visible);

        apply_chunk_visibility(
            center,
            show_radius,
            hide_radius,
            IVec3::new(17, 0, 0),
            &mut visibility,
        );
        assert_eq!(visibility, Visibility::Hidden);
    }

    #[test]
    fn vertical_distance_does_not_hide_surface_while_flying() {
        let (show_radius, hide_radius) = chunk_visibility_radii(4);
        let mut visibility = Visibility::Hidden;

        apply_chunk_visibility(
            IVec2::ZERO,
            show_radius,
            hide_radius,
            IVec3::new(3, 99, 4),
            &mut visibility,
        );

        assert_eq!(visibility, Visibility::Visible);
    }
}
