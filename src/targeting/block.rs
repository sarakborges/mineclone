use bevy::prelude::*;

use super::{
    highlight::TargetHighlightPlugin,
    interaction::BlockInteractionPlugin,
    placement_preview::PlacementPreviewPlugin,
};
use crate::{
    app::game_state::GameState,
    voxel::{
        raycast::{raycast_voxels, VoxelHit},
        world::VoxelWorld,
    },
};

const TARGET_RANGE: f32 = 8.0;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum BlockTargetingSet {
    Raycast,
    Interaction,
    Visuals,
}

pub struct BlockTargetingPlugin;

impl Plugin for BlockTargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>()
            .configure_sets(
                Update,
                (
                    BlockTargetingSet::Raycast,
                    BlockTargetingSet::Interaction,
                    BlockTargetingSet::Visuals,
                )
                    .chain(),
            )
            .add_plugins((TargetHighlightPlugin, BlockInteractionPlugin, PlacementPreviewPlugin))
            .add_systems(
                Update,
                update_targeted_block
                    .in_set(BlockTargetingSet::Raycast)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Resource, Default)]
pub struct TargetedBlock(pub Option<VoxelHit>);

fn update_targeted_block(
    camera: Single<&GlobalTransform, With<Camera3d>>,
    world: Res<VoxelWorld>,
    mut targeted: ResMut<TargetedBlock>,
) {
    targeted.0 = raycast_voxels(
        &world,
        camera.translation(),
        camera.forward().as_vec3(),
        TARGET_RANGE,
    );
}
