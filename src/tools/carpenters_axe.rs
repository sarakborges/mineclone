use bevy::prelude::*;

use crate::{
    content::{
        block::BlockRegistry,
        tool_behavior::{LOG_HOLLOW_BEHAVIOR_ID, LOG_STRIP_BEHAVIOR_ID},
    },
    gameplay::availability::world_interaction_available,
    player::viewmodel::ViewModelAnimation,
    targeting::{
        ToolUse,
        block::{BlockTargetingSet, TargetedBlock},
    },
    voxel::{
        edit::VoxelMutationRuntime,
        log_variant::{LogVariant, log_variant, transformed_log_id},
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
        let behavior = usage.behavior_id.as_str();
        if !matches!(behavior, LOG_HOLLOW_BEHAVIOR_ID | LOG_STRIP_BEHAVIOR_ID) {
            continue;
        }
        let Some(hit) = usage.target else {
            continue;
        };
        if runtime.world().block_id_at(hit.voxel) != Some(hit.block_id) {
            continue;
        }
        let Some(cell) = runtime.cell_at(hit.voxel) else {
            continue;
        };
        let Some(current) = log_variant(cell.block_id) else {
            continue;
        };

        let target = match behavior {
            LOG_HOLLOW_BEHAVIOR_ID => {
                if MicroblockMask::is_modified(cell) {
                    continue;
                }
                match current {
                    LogVariant::Natural => LogVariant::Hollow,
                    LogVariant::Stripped => LogVariant::StrippedHollow,
                    LogVariant::Hollow | LogVariant::StrippedHollow => continue,
                }
            }
            LOG_STRIP_BEHAVIOR_ID => match current {
                LogVariant::Natural => LogVariant::Stripped,
                LogVariant::Hollow => LogVariant::StrippedHollow,
                LogVariant::Stripped | LogVariant::StrippedHollow => continue,
            },
            _ => continue,
        };
        let Some(target_id) = transformed_log_id(cell.block_id, target, &blocks) else {
            continue;
        };

        if runtime
            .set_block(hit.voxel, Some(cell.with_block_id(target_id)))
            .is_some()
        {
            viewmodel.play_hit();
            targeted.0 = None;
        }
    }
}
