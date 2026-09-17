//! The Chisel edits occupation only. Every resulting piece inherits its
//! parent macroblock's ID, texture, orientation and visual properties.

use bevy::prelude::*;

use crate::{
    content::builtin_ids::CHISEL_TOOL_ID,
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
        microblock::{
            CHISEL_MASK_PROPERTY, MICROBLOCK_EDGE, ChiselResolution, MicroblockMask,
            local_cell, parent_voxel,
        },
        raycast::raycast_micro_voxels,
    },
};

pub(super) struct ChiselPlugin;

impl Plugin for ChiselPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChiselResolution>().add_systems(
            Update,
            (cycle_chisel_resolution, handle_chisel_use.after(BlockTargetingSet::Interaction))
                .run_if(world_interaction_available),
        );
    }
}

fn cycle_chisel_resolution(
    keys: Res<ButtonInput<KeyCode>>,
    hotbar: Res<PlayerHotbar>,
    mut resolution: ResMut<ChiselResolution>,
) {
    if keys.just_pressed(KeyCode::KeyR)
        && hotbar.item_at(hotbar.selected_slot()) == Some(CHISEL_TOOL_ID)
    {
        *resolution = resolution.next();
    }
}

fn handle_chisel_use(
    mut uses: MessageReader<ToolUse>,
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    resolution: Res<ChiselResolution>,
    mut runtime: VoxelMutationRuntime,
    mut targeted: ResMut<TargetedBlock>,
    mut viewmodel: ResMut<ViewModelAnimation>,
) {
    for usage in uses.read() {
        if usage.tool_id != CHISEL_TOOL_ID || usage.target.is_none() {
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

        if usage.button == ToolUseButton::Left && *resolution == ChiselResolution::Full {
            if runtime.set_block(hit.voxel, None).is_some() {
                viewmodel.play_break();
                targeted.0 = None;
            }
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
        let existing = runtime.cell_at(voxel);
        let source = if let Some(cell) = existing {
            cell
        } else if placing && runtime.world().fluid_at(voxel).is_none() {
            // A temporary parent in empty space inherits the hit block's
            // material. Disk snapshots skip it instead of restoring a full cube.
            let Some(cell) = runtime.cell_at(hit.voxel) else {
                continue;
            };
            cell.without_secondary_property(CHISEL_MASK_PROPERTY)
        } else {
            continue;
        };
        if !MicroblockMask::has_room(source) {
            continue;
        }
        let transient = existing.is_none_or(MicroblockMask::is_transient_parent);
        let mut mask = existing.map_or(MicroblockMask::EMPTY, MicroblockMask::from_cell);
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

fn piece_intersects_player(fine: IVec3, precision: ChiselResolution, eye: Vec3) -> bool {
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
