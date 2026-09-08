use bevy::prelude::*;

use super::highlight::TargetHighlightPlugin;
use crate::{
    app::game_state::GameState,
    voxel::{
        raycast::{raycast_voxels, VoxelHit},
        world::VoxelWorld,
    },
};

const TARGET_RANGE: f32 = 5.0;

pub struct BlockTargetingPlugin;

impl Plugin for BlockTargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>()
            .add_plugins(TargetHighlightPlugin)
            .add_systems(
                Update,
                update_targeted_block.run_if(in_state(GameState::Gameplay)),
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
