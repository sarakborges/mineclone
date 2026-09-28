use bevy::prelude::{IVec2, IVec3};

use crate::voxel::coordinates::ChunkCoord;

/// Owns the current streaming selection pose independently from logical
/// residency, generation queues and render presentation. The values here
/// describe where selection is centered and how its preload policy is oriented;
/// they do not imply that any chunk is resident or rendered.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct StreamingSelectionState {
    center: Option<ChunkCoord>,
    movement_direction: IVec2,
    horizontal_radius: i32,
    vertical_radius: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct StreamingSelectionSnapshot {
    center: Option<ChunkCoord>,
    movement_direction: IVec2,
    horizontal_radius: i32,
    vertical_radius: i32,
}

impl StreamingSelectionState {
    pub(super) fn center(&self) -> Option<IVec3> {
        self.center.map(ChunkCoord::as_ivec3)
    }

    pub(super) fn movement_direction(&self) -> IVec2 {
        self.movement_direction
    }

    pub(super) fn horizontal_radius(&self) -> i32 {
        self.horizontal_radius
    }

    pub(super) fn needs_rebuild(
        &self,
        center: IVec3,
        horizontal_radius: i32,
        vertical_radius: i32,
    ) -> bool {
        self.center != Some(ChunkCoord::from_ivec3(center))
            || self.horizontal_radius != horizontal_radius
            || self.vertical_radius != vertical_radius
    }

    pub(super) fn prepare_rebuild(
        &mut self,
        center: IVec3,
        allow_forward_preload: bool,
    ) -> StreamingSelectionSnapshot {
        let previous = self.snapshot();
        if allow_forward_preload {
            self.update_movement_direction(center);
        } else {
            self.movement_direction = IVec2::ZERO;
        }
        previous
    }

    pub(super) fn commit_rebuild(
        &mut self,
        center: IVec3,
        horizontal_radius: i32,
        vertical_radius: i32,
    ) {
        self.center = Some(ChunkCoord::from_ivec3(center));
        self.horizontal_radius = horizontal_radius;
        self.vertical_radius = vertical_radius;
    }

    fn snapshot(&self) -> StreamingSelectionSnapshot {
        StreamingSelectionSnapshot {
            center: self.center,
            movement_direction: self.movement_direction,
            horizontal_radius: self.horizontal_radius,
            vertical_radius: self.vertical_radius,
        }
    }

    fn update_movement_direction(&mut self, center: IVec3) {
        let Some(previous_center) = self.center.map(ChunkCoord::as_ivec3) else {
            return;
        };
        let delta = IVec2::new(
            center.x - previous_center.x,
            center.z - previous_center.z,
        );
        if delta == IVec2::ZERO {
            return;
        }

        self.movement_direction = IVec2::new(delta.x.signum(), delta.y.signum());
    }

    #[cfg(test)]
    pub(super) fn configured(
        center: Option<IVec3>,
        movement_direction: IVec2,
        horizontal_radius: i32,
        vertical_radius: i32,
    ) -> Self {
        Self {
            center: center.map(ChunkCoord::from_ivec3),
            movement_direction,
            horizontal_radius,
            vertical_radius,
        }
    }
}

impl StreamingSelectionSnapshot {
    pub(super) fn center(self) -> Option<IVec3> {
        self.center.map(ChunkCoord::as_ivec3)
    }

    pub(super) fn movement_direction(self) -> IVec2 {
        self.movement_direction
    }

    pub(super) fn horizontal_radius(self) -> i32 {
        self.horizontal_radius
    }

    pub(super) fn vertical_radius(self) -> i32 {
        self.vertical_radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_preload_direction_tracks_horizontal_chunk_motion() {
        let mut state = StreamingSelectionState::configured(
            Some(IVec3::new(4, 2, -3)),
            IVec2::NEG_Y,
            12,
            4,
        );

        let previous = state.prepare_rebuild(IVec3::new(5, 2, -2), true);

        assert_eq!(previous.center(), Some(IVec3::new(4, 2, -3)));
        assert_eq!(previous.movement_direction(), IVec2::NEG_Y);
        assert_eq!(state.movement_direction(), IVec2::ONE);
    }

    #[test]
    fn warp_selection_disables_forward_preload_direction() {
        let mut state = StreamingSelectionState::configured(
            Some(IVec3::ZERO),
            IVec2::X,
            12,
            4,
        );

        state.prepare_rebuild(IVec3::new(8, 0, 0), false);

        assert_eq!(state.movement_direction(), IVec2::ZERO);
    }

    #[test]
    fn committed_pose_controls_rebuild_detection() {
        let mut state = StreamingSelectionState::default();
        assert!(state.needs_rebuild(IVec3::ZERO, 12, 4));

        state.commit_rebuild(IVec3::ZERO, 12, 4);

        assert!(!state.needs_rebuild(IVec3::ZERO, 12, 4));
        assert!(state.needs_rebuild(IVec3::X, 12, 4));
        assert!(state.needs_rebuild(IVec3::ZERO, 13, 4));
        assert!(state.needs_rebuild(IVec3::ZERO, 12, 5));
    }
}
