use bevy::prelude::*;

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry, DEFAULT_BLOCK_BREAK_TICKS},
        tool::{ToolDefinition, ToolRegistry},
    },
    gameplay::availability::world_interaction_available,
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        viewmodel::ViewModelAnimation,
    },
    voxel::edit::VoxelTopologyRuntime,
    world::tick::WorldTickClock,
};

use super::block::{BlockTargetingSet, TargetedBlock};

const MINING_SWING_INTERVAL_TICKS: u64 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MiningTarget {
    voxel: IVec3,
    block_id: &'static str,
    selected_item: Option<&'static str>,
}

#[derive(Resource, Default)]
pub(crate) struct BlockMiningState {
    target: Option<MiningTarget>,
    accumulated_work: f32,
    swing_ticks: u64,
}

impl BlockMiningState {
    fn begin(&mut self, target: MiningTarget) {
        self.target = Some(target);
        self.accumulated_work = 0.0;
        self.swing_ticks = 0;
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn progress_for(
        &self,
        voxel: IVec3,
        block_id: &'static str,
        required_work: f32,
    ) -> Option<f32> {
        let target = self.target?;
        if target.voxel != voxel || target.block_id != block_id {
            return None;
        }
        if required_work <= 0.0 {
            return Some(1.0);
        }
        Some((self.accumulated_work / required_work).clamp(0.0, 1.0))
    }
}

pub(super) struct BlockMiningPlugin;

impl Plugin for BlockMiningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlockMiningState>().add_systems(
            Update,
            advance_survival_mining
                .in_set(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn advance_survival_mining(
    buttons: Res<ButtonInput<MouseButton>>,
    player: Single<&GameMode, With<GameplayCamera>>,
    hotbar: Res<PlayerHotbar>,
    mut targeted: ResMut<TargetedBlock>,
    blocks: Res<BlockRegistry>,
    tools: Res<ToolRegistry>,
    world_ticks: Res<WorldTickClock>,
    mut runtime: VoxelTopologyRuntime,
    mut mining: ResMut<BlockMiningState>,
    mut viewmodel: ResMut<ViewModelAnimation>,
) {
    if *player.into_inner() != GameMode::Survival || !buttons.pressed(MouseButton::Left) {
        mining.reset();
        return;
    }

    let Some(hit) = targeted.0 else {
        mining.reset();
        return;
    };
    if runtime.world().block_id_at(hit.voxel) != Some(hit.block_id) {
        mining.reset();
        return;
    }

    let selected_item = hotbar.item_at(hotbar.selected_slot());
    let selected_tool = selected_item.and_then(|item_id| tools.get(item_id));

    // Tools without mining tags own their left-click action (brush, chisel, etc.)
    // and must not also mine the underlying block.
    if selected_tool.is_some_and(|tool| !tool.mining.is_mining_tool()) {
        mining.reset();
        return;
    }

    let target = MiningTarget {
        voxel: hit.voxel,
        block_id: hit.block_id,
        selected_item,
    };
    let started = mining.target != Some(target);
    if started {
        mining.begin(target);
        viewmodel.play_break_fast();
    }

    let elapsed_ticks = world_ticks.ticks_this_frame() as u64;
    if !started {
        mining.swing_ticks = mining.swing_ticks.saturating_add(elapsed_ticks);
        if mining.swing_ticks >= MINING_SWING_INTERVAL_TICKS {
            mining.swing_ticks %= MINING_SWING_INTERVAL_TICKS;
            viewmodel.play_break_fast();
        }
    }

    let Some(block) = blocks.get(hit.block_id) else {
        mining.reset();
        return;
    };
    let Some(speed) = effective_mining_speed(block, selected_tool) else {
        // A required tool mismatch is intentionally a hard gate: the player can
        // keep swinging forever, but mining work never advances.
        return;
    };

    let required_work = DEFAULT_BLOCK_BREAK_TICKS as f32 * block.mining.hardness;
    if required_work > 0.0 {
        mining.accumulated_work += world_ticks.ticks_this_frame() as f32 * speed;
        if mining.accumulated_work < required_work {
            return;
        }
    }

    if runtime.set_block(hit.voxel, None).is_some() {
        targeted.0 = None;
        viewmodel.play_break_fast();
    }
    mining.reset();
}

fn effective_mining_speed(
    block: &BlockDefinition,
    selected_tool: Option<&ToolDefinition>,
) -> Option<f32> {
    if !block.mining.required_tools.is_empty() {
        let tool = selected_tool?;
        return tool_matches_any(tool, &block.mining.required_tools).then_some(tool.mining.speed);
    }

    let preferred_speed = selected_tool
        .filter(|tool| tool_matches_any(tool, &block.mining.preferred_tools))
        .map(|tool| tool.mining.speed);

    Some(preferred_speed.unwrap_or(1.0))
}

fn tool_matches_any(tool: &ToolDefinition, accepted_categories: &[String]) -> bool {
    accepted_categories
        .iter()
        .any(|required| tool.mining.matches_category(required))
}
