use bevy::prelude::*;

use super::{
    highlight::TargetHighlightPlugin, interaction::BlockInteractionPlugin,
    placement_orientation::PlacementOrientationPlugin, placement_preview::PlacementPreviewPlugin,
};
use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    gameplay::availability::WorldInteractionState,
    player::camera::GameplayCamera,
    voxel::{
        raycast::{VoxelHit, raycast_voxels},
        world::VoxelWorld,
    },
};

const TARGET_RANGE: f32 = 8.0;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum BlockTargetingSet {
    Raycast,
    PlacementState,
    Interaction,
    Visuals,
}

pub struct BlockTargetingPlugin;

impl Plugin for BlockTargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>()
            .init_resource::<TargetingRaycastCache>()
            .configure_sets(
                Update,
                (
                    BlockTargetingSet::Raycast,
                    BlockTargetingSet::PlacementState,
                    BlockTargetingSet::Interaction,
                    BlockTargetingSet::Visuals,
                )
                    .chain(),
            )
            .add_plugins((
                TargetHighlightPlugin,
                PlacementOrientationPlugin,
                BlockInteractionPlugin,
                PlacementPreviewPlugin,
            ))
            .add_systems(
                OnEnter(GameState::Gameplay),
                reset_resource::<TargetingRaycastCache>,
            )
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

#[derive(Resource, Default)]
struct TargetingRaycastCache {
    last_inputs: Option<TargetingRaycastInputs>,
}

#[derive(Clone, Copy, PartialEq)]
struct TargetingRaycastInputs {
    origin: Vec3,
    direction: Vec3,
    block_content_revision: u64,
    interaction_available: bool,
}

fn update_targeted_block(
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    interaction: WorldInteractionState,
    mut cache: ResMut<TargetingRaycastCache>,
    mut targeted: ResMut<TargetedBlock>,
) {
    let interaction_available = interaction.available();
    let inputs = TargetingRaycastInputs {
        origin: camera.translation(),
        direction: camera.forward().as_vec3(),
        block_content_revision: world.block_content_revision(),
        interaction_available,
    };
    if cache.last_inputs == Some(inputs) {
        return;
    }
    cache.last_inputs = Some(inputs);

    let next = if interaction_available {
        raycast_voxels(&world, inputs.origin, inputs.direction, TARGET_RANGE)
    } else {
        None
    };

    if targeted.0 != next {
        targeted.0 = next;
    }
}
