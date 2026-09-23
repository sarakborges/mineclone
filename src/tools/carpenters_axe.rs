use bevy::prelude::*;

use crate::{
    content::{block::BlockRegistry, builtin_ids::CARPENTERS_AXE_TOOL_ID},
    gameplay::availability::world_interaction_available,
    player::viewmodel::ViewModelAnimation,
    targeting::{ToolUse, ToolUseButton, block::{BlockTargetingSet, TargetedBlock}},
    voxel::{
        edit::VoxelMutationRuntime,
        log_state::{
            HOLLOW_LOG_PROPERTY, LOG_STATE_ENABLED, STRIPPED_LOG_PROPERTY, is_hollow, is_stripped,
        },
        microblock::MicroblockMask,
    },
};

pub(super) struct CarpentersAxePlugin;

impl Plugin for CarpentersAxePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_carpenters_axe_use
                .after(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

fn handle_carpenters_axe_use(
    mut uses: MessageReader<ToolUse>,
    blocks: Res<BlockRegistry>,
    mut runtime: VoxelMutationRuntime,
    mut targeted: ResMut<TargetedBlock>,
    mut viewmodel: ResMut<ViewModelAnimation>,
) {
    for usage in uses.read() {
        if usage.tool_id != CARPENTERS_AXE_TOOL_ID {
            continue;
        }
        let Some(hit) = usage.target else {
            continue;
        };
        if runtime.world().block_id_at(hit.voxel) != Some(hit.block_id) {
            continue;
        }
        let Some(block) = blocks.get(hit.block_id) else {
            continue;
        };
        if !block.is_log() {
            continue;
        }
        let Some(cell) = runtime.cell_at(hit.voxel) else {
            continue;
        };

        let updated = match usage.button {
            ToolUseButton::Left => {
                if is_hollow(cell) || MicroblockMask::is_modified(cell) {
                    continue;
                }
                cell.with_secondary_property(HOLLOW_LOG_PROPERTY, LOG_STATE_ENABLED)
            }
            ToolUseButton::Right => {
                if is_stripped(cell) {
                    continue;
                }
                cell.with_secondary_property(STRIPPED_LOG_PROPERTY, LOG_STATE_ENABLED)
            }
        };

        if runtime.set_block(hit.voxel, Some(updated)).is_some() {
            viewmodel.play_hit();
            targeted.0 = None;
        }
    }
}
