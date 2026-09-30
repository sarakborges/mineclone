use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    voxel::coordinates::chunk_coord_from_position,
    world::render_distance::{RenderDistanceSettings, chunk_visibility_radii},
};

use super::chunk_rendering::ChunkRenderCoord;

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ChunkPresentationSelection {
    center: Option<IVec2>,
    hide_radius: i32,
    revision: u64,
}

impl ChunkPresentationSelection {
    pub(crate) fn sync_from_streaming(
        &mut self,
        center: Option<IVec3>,
        horizontal_radius: i32,
    ) {
        let center = center.map(|coord| coord.xz());
        let (_, hide_radius) = chunk_visibility_radii(horizontal_radius);
        if self.center == center && self.hide_radius == hide_radius {
            return;
        }

        self.center = center;
        self.hide_radius = hide_radius;
        self.revision = self
            .revision
            .checked_add(1)
            .expect("chunk presentation selection revision exhausted");
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn retains_render_mesh(&self, coord: IVec3) -> bool {
        self.center
            .is_some_and(|center| chunk_is_inside_visible_radius(center, coord, self.hide_radius))
    }
}

#[derive(Default)]
pub(super) struct ChunkVisibilityState {
    center: Option<IVec2>,
    show_radius: i32,
    hide_radius: i32,
}

pub(super) fn prime_chunk_visibility(
    center: IVec2,
    render_distance_chunks: i32,
    chunks: &mut Query<(&ChunkRenderCoord, &mut Visibility)>,
) {
    let (show_radius, hide_radius) = chunk_visibility_radii(render_distance_chunks);
    for (coord, mut visibility) in chunks.iter_mut() {
        apply_chunk_visibility(center, show_radius, hide_radius, coord.0, &mut visibility);
    }
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
    fn presentation_selection_tracks_streaming_pose_and_revision() {
        let mut selection = ChunkPresentationSelection::default();

        selection.sync_from_streaming(Some(IVec3::new(4, 7, -3)), 12);
        let first_revision = selection.revision();

        assert!(selection.retains_render_mesh(IVec3::new(18, 0, -3)));
        assert!(!selection.retains_render_mesh(IVec3::new(20, 0, -3)));

        selection.sync_from_streaming(Some(IVec3::new(4, 7, -3)), 12);
        assert_eq!(selection.revision(), first_revision);

        selection.sync_from_streaming(Some(IVec3::new(5, 7, -3)), 12);
        assert_eq!(selection.revision(), first_revision + 1);
        assert!(selection.retains_render_mesh(IVec3::new(19, 0, -3)));
    }

    #[test]
    fn presentation_selection_does_not_depend_on_authoritative_residency() {
        let mut selection = ChunkPresentationSelection::default();
        selection.sync_from_streaming(Some(IVec3::ZERO), 12);

        assert!(selection.retains_render_mesh(IVec3::new(13, 99, 0)));
    }

    #[test]
    fn hidden_chunk_only_enters_inside_show_radius() {
        let center = IVec2::ZERO;
        let (show_radius, hide_radius) = chunk_visibility_radii(12);
        let mut visibility = Visibility::Hidden;

        apply_chunk_visibility(
            center,
            show_radius,
            hide_radius,
            IVec3::new(14, 0, 0),
            &mut visibility,
        );
        assert_eq!(visibility, Visibility::Hidden);

        apply_chunk_visibility(
            center,
            show_radius,
            hide_radius,
            IVec3::new(13, 0, 0),
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
            IVec3::new(14, 0, 0),
            &mut visibility,
        );
        assert_eq!(visibility, Visibility::Visible);

        apply_chunk_visibility(
            center,
            show_radius,
            hide_radius,
            IVec3::new(15, 0, 0),
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
