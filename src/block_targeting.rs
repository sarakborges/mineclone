use bevy::prelude::*;

use crate::{
    crosshair::CrosshairPlugin,
    game_state::GameState,
    target_highlight::TargetHighlightPlugin,
    target_hud::TargetHudPlugin,
    voxel_chunk::VoxelChunk,
    voxel_raycast::{raycast_voxels, VoxelHit},
};

const TARGET_RANGE: f32 = 5.0;

pub struct BlockTargetingPlugin;

impl Plugin for BlockTargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>()
            .add_plugins((CrosshairPlugin, TargetHighlightPlugin, TargetHudPlugin))
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
    chunk: Single<&VoxelChunk>,
    mut targeted: ResMut<TargetedBlock>,
) {
    targeted.0 = raycast_voxels(
        &chunk,
        camera.translation(),
        camera.forward().as_vec3(),
        TARGET_RANGE,
    );
}
