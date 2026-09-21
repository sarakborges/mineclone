use bevy::prelude::*;

use crate::{
    content::{
        builtin_ids::SHEARS_TOOL_ID,
        layer::LayerFace,
    },
    gameplay::availability::world_interaction_available,
    player::viewmodel::ViewModelAnimation,
    targeting::{ToolUse, ToolUseButton, block::BlockTargetingSet},
    voxel::edit::VoxelTopologyRuntime,
};

pub(super) struct ShearsPlugin;

impl Plugin for ShearsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_shears_use
                .after(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

fn handle_shears_use(
    mut uses: MessageReader<ToolUse>,
    mut runtime: VoxelTopologyRuntime,
    mut viewmodel: ResMut<ViewModelAnimation>,
) {
    for usage in uses.read() {
        if usage.tool_id != SHEARS_TOOL_ID || usage.button != ToolUseButton::Right {
            continue;
        }
        let Some(hit) = usage.target else {
            continue;
        };
        if runtime.world().block_id_at(hit.voxel) != Some(hit.block_id) {
            continue;
        }
        let Some(face) = LayerFace::from_normal(hit.normal) else {
            continue;
        };

        if runtime.remove_top_layer(hit.voxel, face).is_some() {
            viewmodel.play_break();
        }
    }
}
