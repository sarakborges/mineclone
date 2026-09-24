use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry, DEFAULT_BLOCK_BREAK_TICKS},
        block_id::intern_block_id,
        builtin_ids::BIOME_TINT_METADATA_KEY,
        item::ItemRegistry,
        item_id::intern_item_id,
        layer::LayerRegistry,
        layer_id::intern_layer_id,
        object::ObjectRegistry,
        object_id::intern_object_id,
        tool::{ToolDefinition, ToolRegistry},
        tool_id::intern_tool_id,
        tool_behavior::MINE_TOOL_BEHAVIOR_ID,
    },
    gameplay::{
        availability::world_interaction_available,
        random::next_unit_f32,
    },
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::PlayerHotbar,
        item_stack::{ItemStack, MAX_STACK_SIZE},
        viewmodel::ViewModelAnimation,
    },
    voxel::edit::VoxelTopologyRuntime,
    world::tick::WorldTickClock,
    world_items::WorldItemSpawnRequest,
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

#[derive(SystemParam)]
struct MiningContent<'w> {
    blocks: Res<'w, BlockRegistry>,
    items: Res<'w, ItemRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
}

#[derive(SystemParam)]
struct MiningRuntime<'w> {
    targeted: ResMut<'w, TargetedBlock>,
    world: VoxelTopologyRuntime<'w>,
    mining: ResMut<'w, BlockMiningState>,
    viewmodel: ResMut<'w, ViewModelAnimation>,
    item_spawns: MessageWriter<'w, WorldItemSpawnRequest>,
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

    pub(crate) fn is_actively_mining(&self, voxel: IVec3, block_id: &'static str) -> bool {
        self.target
            .is_some_and(|target| target.voxel == voxel && target.block_id == block_id)
            && self.accumulated_work > 0.0
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

fn advance_survival_mining(
    buttons: Res<ButtonInput<MouseButton>>,
    player: Single<&GameMode, With<GameplayCamera>>,
    hotbar: Res<PlayerHotbar>,
    content: MiningContent,
    world_ticks: Res<WorldTickClock>,
    mut runtime: MiningRuntime,
) {
    if *player.into_inner() != GameMode::Survival || !buttons.pressed(MouseButton::Left) {
        runtime.mining.reset();
        return;
    }

    let Some(hit) = runtime.targeted.0 else {
        runtime.mining.reset();
        return;
    };
    if runtime.world.world().block_id_at(hit.voxel) != Some(hit.block_id) {
        runtime.mining.reset();
        return;
    }

    let selected_item = hotbar.item_at(hotbar.selected_slot());
    let selected_tool = selected_item.and_then(|item_id| content.tools.get(item_id));

    // The left-click behavior owns dispatch. Only tools explicitly configured
    // with the mining behavior may participate in the mining loop.
    if selected_tool.is_some_and(|tool| tool.left_behavior != MINE_TOOL_BEHAVIOR_ID) {
        runtime.mining.reset();
        return;
    }

    let target = MiningTarget {
        voxel: hit.voxel,
        block_id: hit.block_id,
        selected_item,
    };
    let started = runtime.mining.target != Some(target);
    if started {
        runtime.mining.begin(target);
        runtime.viewmodel.play_break_fast();
    }

    let elapsed_ticks = world_ticks.ticks_this_frame() as u64;
    if !started {
        runtime.mining.swing_ticks = runtime.mining.swing_ticks.saturating_add(elapsed_ticks);
        if runtime.mining.swing_ticks >= MINING_SWING_INTERVAL_TICKS {
            runtime.mining.swing_ticks %= MINING_SWING_INTERVAL_TICKS;
            runtime.viewmodel.play_break_fast();
        }
    }

    let Some(block) = content.blocks.get(hit.block_id) else {
        runtime.mining.reset();
        return;
    };
    let Some(speed) = effective_mining_speed(block, selected_tool) else {
        // A required tool mismatch is intentionally a hard gate: the player can
        // keep swinging forever, but mining work never advances.
        return;
    };

    let required_work = DEFAULT_BLOCK_BREAK_TICKS as f32 * block.mining.hardness;
    if required_work > 0.0 {
        runtime.mining.accumulated_work += world_ticks.ticks_this_frame() as f32 * speed;
        if runtime.mining.accumulated_work < required_work {
            return;
        }
    }

    let biome_tint = runtime
        .world
        .world()
        .cell_at(hit.voxel)
        .and_then(|cell| cell.secondary_property(BIOME_TINT_METADATA_KEY))
        .map(str::to_owned);

    if runtime.world.set_block(hit.voxel, None).is_some() {
        spawn_survival_loot(
            block,
            hit.voxel,
            biome_tint.as_deref(),
            world_ticks.current_tick(),
            &content,
            &mut runtime.item_spawns,
        );
        runtime.targeted.0 = None;
        runtime.viewmodel.play_break_fast();
    }
    runtime.mining.reset();
}

fn spawn_survival_loot(
    block: &BlockDefinition,
    voxel: IVec3,
    biome_tint: Option<&str>,
    current_tick: u64,
    content: &MiningContent<'_>,
    item_spawns: &mut MessageWriter<WorldItemSpawnRequest>,
) {
    let mut random_state = loot_random_seed(voxel, current_tick);
    let position = voxel.as_vec3() + Vec3::splat(0.5);

    for entry in block.loot_table.entries() {
        let succeeds = entry.chance >= 1.0 || next_unit_f32(&mut random_state) < entry.chance;
        if !succeeds {
            continue;
        }

        let item_id = resolve_loot_item_id(&entry.item, content);
        let mut remaining_quantity = entry.quantity;
        while remaining_quantity > 0 {
            let stack_quantity = remaining_quantity.min(MAX_STACK_SIZE);
            remaining_quantity -= stack_quantity;

            let mut stack = ItemStack::new(item_id).with_quantity(stack_quantity);
            if item_id == block.id.as_str()
                && let Some(biome_id) = biome_tint
            {
                stack = stack.with_metadata(BIOME_TINT_METADATA_KEY, biome_id);
            }

            item_spawns.write(WorldItemSpawnRequest::dropped(stack, position));
        }
    }
}

fn resolve_loot_item_id(item_id: &str, content: &MiningContent<'_>) -> &'static str {
    if content.items.get(item_id).is_some() {
        intern_item_id(item_id)
    } else if content.blocks.get(item_id).is_some() {
        intern_block_id(item_id)
    } else if content.layers.get(item_id).is_some() {
        intern_layer_id(item_id)
    } else if content.objects.get(item_id).is_some() {
        intern_object_id(item_id)
    } else if content.tools.get(item_id).is_some() {
        intern_tool_id(item_id)
    } else {
        unreachable!("loot references are validated during content loading: {item_id}")
    }
}

fn loot_random_seed(voxel: IVec3, current_tick: u64) -> u32 {
    let mut seed = (current_tick as u32) ^ ((current_tick >> 32) as u32).rotate_left(11);
    seed ^= (voxel.x as u32).wrapping_mul(0x9E37_79B9);
    seed ^= (voxel.y as u32).wrapping_mul(0x85EB_CA6B);
    seed ^= (voxel.z as u32).wrapping_mul(0xC2B2_AE35);
    if seed == 0 {
        0xA341_316C
    } else {
        seed
    }
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
