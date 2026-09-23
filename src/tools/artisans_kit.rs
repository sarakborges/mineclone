//! The Artisan's Kit edits occupation only. Every resulting piece inherits its
//! parent macroblock's ID, texture, orientation and visual properties.

use bevy::prelude::*;

use crate::{
    app::keybinds::{KeybindAction, Keybinds},
    content::{block::BlockRegistry, builtin_ids::ARTISANS_KIT_TOOL_ID},
    gameplay::availability::world_interaction_available,
    player::{
        PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT,
        camera::GameplayCamera,
        hotbar::PlayerHotbar,
        viewmodel::ViewModelAnimation,
    },
    targeting::{ToolUse, ToolUseButton, block::{BlockTargetingSet, TargetedBlock}},
    voxel::{
        edit::VoxelMutationRuntime,
        log_variant::is_hollow_log_id,
        microblock::{
            MICROBLOCK_EDGE, ArtisansKitResolution, MicroblockMask, local_cell, parent_voxel,
        },
        raycast::raycast_micro_voxels,
    },
};

pub(super) struct ArtisansKitPlugin;

impl Plugin for ArtisansKitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArtisansKitResolution>().add_systems(
            Update,
            (cycle_artisans_kit_resolution, handle_artisans_kit_use.after(BlockTargetingSet::Interaction))
                .run_if(world_interaction_available),
        );
    }
}

fn cycle_artisans_kit_resolution(
    keys: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    hotbar: Res<PlayerHotbar>,
    mut resolution: ResMut<ArtisansKitResolution>,
) {
    if keys.just_pressed(keybinds.key_code(KeybindAction::ToolAction))
        && hotbar.item_at(hotbar.selected_slot()) == Some(ARTISANS_KIT_TOOL_ID)
    {
        *resolution = resolution.next();
    }
}

fn handle_artisans_kit_use(
    mut uses: MessageReader<ToolUse>,
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    resolution: Res<ArtisansKitResolution>,
    blocks: Res<BlockRegistry>,
    mut runtime: VoxelMutationRuntime,
    mut targeted: ResMut<TargetedBlock>,
    mut viewmodel: ResMut<ViewModelAnimation>,
) {
    for usage in uses.read() {
        if usage.tool_id != ARTISANS_KIT_TOOL_ID || usage.target.is_none() {
            continue;
        }

        let Some(hit) = raycast_micro_voxels(
            runtime.world(),
            camera.translation(),
            camera.forward().as_vec3(),
            8.0,
        ) else {
            continue;
        };
        // Preserve exclusive creature targeting and reject stale ray results.
        if !usage.target.is_some_and(|target| {
            target.voxel == hit.voxel
                && target.block_id == hit.block_id
                && target.normal == hit.normal
        }) || runtime.world().block_id_at(hit.voxel) != Some(hit.block_id)
        {
            continue;
        }
        if !blocks.get(hit.block_id).is_some_and(|block| block.can_fragment()) {
            continue;
        }

        let fine = match usage.button {
            ToolUseButton::Left => hit.fine,
            ToolUseButton::Right if hit.normal != IVec3::ZERO => hit.fine + hit.normal,
            ToolUseButton::Right => continue,
        };
        let voxel = parent_voxel(fine);
        if voxel.y < 0 || !runtime.world().is_loaded_at(voxel) {
            continue;
        }
        let placing = usage.button == ToolUseButton::Right;
        let Some(source) = runtime.cell_at(voxel) else {
            // The Artisan's Kit can restore a removed piece, never create a new parent
            // block in air (including at a neighboring macroblock boundary).
            continue;
        };
        if is_hollow_log_id(source.block_id) {
            continue;
        }
        if placing && (voxel != hit.voxel || !MicroblockMask::can_restore(source)) {
            continue;
        }
        if !blocks.get(source.block_id).is_some_and(|block| block.can_fragment())
            || !MicroblockMask::has_room(source)
        {
            continue;
        }
        let transient = MicroblockMask::is_transient_parent(source);
        let mut mask = MicroblockMask::from_cell(source);
        if !mask.edit(local_cell(fine), *resolution, placing) {
            continue;
        }
        if placing && piece_intersects_player(fine, *resolution, camera.translation()) {
            continue;
        }
        let updated = if transient && mask == MicroblockMask::EMPTY {
            None
        } else {
            Some(mask.apply_to_cell(source, transient))
        };
        if runtime.set_block(voxel, updated).is_some() {
            if placing {
                viewmodel.play_place();
            } else {
                viewmodel.play_break();
            }
            targeted.0 = None;
        }
    }
}

fn piece_intersects_player(fine: IVec3, precision: ArtisansKitResolution, eye: Vec3) -> bool {
    let edge = precision.cell_width() as i32;
    let snapped = IVec3::new(
        fine.x.div_euclid(edge) * edge,
        fine.y.div_euclid(edge) * edge,
        fine.z.div_euclid(edge) * edge,
    );
    let min = snapped.as_vec3() / MICROBLOCK_EDGE as f32;
    let max = min + Vec3::splat(edge as f32 / MICROBLOCK_EDGE as f32);
    let feet = eye.y - PLAYER_EYE_HEIGHT;
    let player_min = Vec3::new(eye.x - PLAYER_HALF_WIDTH, feet, eye.z - PLAYER_HALF_WIDTH);
    let player_max = Vec3::new(
        eye.x + PLAYER_HALF_WIDTH,
        feet + PLAYER_HEIGHT,
        eye.z + PLAYER_HALF_WIDTH,
    );
    player_min.x < max.x
        && player_max.x > min.x
        && player_min.y < max.y
        && player_max.y > min.y
        && player_min.z < max.z
        && player_max.z > min.z
}
