use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{block_shape::is_stackable_layer, object::ObjectRegistry},
    player::item_stack::ItemStack,
    rendering::{
        block_model::{
            BlockModelMeshes, block_face_material_data, maximum_block_model_layers,
            set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_visual_content::BlockVisualContent,
    },
    world::{current_context::CurrentDimensionContext, tick::WorldTickClock},
    world_items::WorldItemSpawnRequest,
    world_objects::detached_object_drop_request,
};

use super::{
    block_face::BlockFace,
    cell::VoxelCell,
    edit::VoxelTopologyRuntime,
    orientation::source_face_for_cell_visual,
    read::VoxelRead,
    stackable_layer::stackable_layer_count,
};

pub(crate) const BLOCK_GRAVITY_TAG: &str = "gravity";

#[derive(Resource, Default)]
pub(crate) struct PendingBlockGravityUpdates {
    queue: VecDeque<IVec3>,
    queued: HashSet<IVec3>,
}

impl PendingBlockGravityUpdates {
    pub(crate) fn enqueue_voxel_edit(&mut self, position: IVec3) {
        self.enqueue(position);
        self.enqueue(position + IVec3::Y);
    }

    pub(crate) fn enqueue(&mut self, position: IVec3) {
        if position.y < 0 || !self.queued.insert(position) {
            return;
        }
        self.queue.push_back(position);
    }

    pub(crate) fn take_batch(&mut self) -> Vec<IVec3> {
        let batch_len = self.queue.len();
        let mut batch = Vec::with_capacity(batch_len);
        for _ in 0..batch_len {
            let Some(position) = self.queue.pop_front() else { break; };
            self.queued.remove(&position);
            batch.push(position);
        }
        batch
    }
}

#[derive(Component)]
struct FallingBlock {
    cell: VoxelCell,
    column: IVec2,
    velocity_y: f32,
}

pub(crate) struct BlockGravityPlugin;

impl Plugin for BlockGravityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingBlockGravityUpdates>()
            .add_systems(OnEnter(GameState::Loading), reset_resource::<PendingBlockGravityUpdates>)
            .add_systems(OnExit(GameState::Gameplay), reset_resource::<PendingBlockGravityUpdates>)
            .add_systems(
                PostUpdate,
                (process_block_gravity, simulate_falling_blocks)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn process_block_gravity(
    mut commands: Commands,
    world_ticks: Res<WorldTickClock>,
    objects: Res<ObjectRegistry>,
    content: BlockVisualContent,
    block_meshes: Res<BlockModelMeshes>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
    dimension: CurrentDimensionContext,
    mut runtime: VoxelTopologyRuntime,
    mut item_spawns: MessageWriter<WorldItemSpawnRequest>,
) {
    if world_ticks.ticks_this_frame() == 0 {
        return;
    }
    let gravity_strength = dimension.definition().map_or(0.0, |definition| definition.gravity_strength);

    for position in runtime.take_block_gravity_batch() {
        let Some(cell) = runtime.read().cell_at(position) else { continue; };
        let Some(definition) = content.blocks.get(cell.block_id) else { continue; };
        let below = position - IVec3::Y;

        if is_stackable_layer(definition) {
            if below.y >= 0 && !runtime.read().is_loaded_at(below) {
                runtime.enqueue_block_gravity(position);
                continue;
            }
            if runtime.read().cell_at(below).is_some() {
                continue;
            }
            let Some(mutation) = runtime.set_block_detailed(position, None) else {
                runtime.enqueue_block_gravity(position);
                continue;
            };
            item_spawns.write(WorldItemSpawnRequest::dropped(
                stackable_layer_drop_stack(cell),
                position.as_vec3() + Vec3::splat(0.5),
            ));
            emit_detached_object_drops(
                position,
                mutation.previous_cell,
                mutation.detached_objects,
                &objects,
                &mut item_spawns,
            );
            continue;
        }

        if !definition.tags.iter().any(|tag| tag == BLOCK_GRAVITY_TAG)
            || gravity_strength <= 0.0
            || below.y < 0
        {
            continue;
        }
        if !runtime.read().is_loaded_at(below) {
            runtime.enqueue_block_gravity(position);
            continue;
        }
        if runtime.read().cell_at(below).is_some() {
            continue;
        }

        let Some(mutation) = runtime.set_block_detailed(position, None) else {
            runtime.enqueue_block_gravity(position);
            continue;
        };
        emit_detached_object_drops(
            position,
            mutation.previous_cell,
            mutation.detached_objects,
            &objects,
            &mut item_spawns,
        );
        spawn_falling_block(
            &mut commands,
            &content,
            &block_meshes,
            &mut materials,
            position,
            cell,
        );
    }
}

fn spawn_falling_block(
    commands: &mut Commands,
    content: &BlockVisualContent,
    block_meshes: &BlockModelMeshes,
    materials: &mut Assets<BlockModelMaterial>,
    position: IVec3,
    cell: VoxelCell,
) {
    let Some(block) = content.blocks.get(cell.block_id) else { return; };
    let tint = content
        .tint_at(cell.block_id, Vec2::new(position.x as f32 + 0.5, position.z as f32 + 0.5))
        .unwrap_or(Color::WHITE);

    commands
        .spawn((
            FallingBlock {
                cell,
                column: IVec2::new(position.x, position.z),
                velocity_y: 0.0,
            },
            Transform::from_translation(position.as_vec3() + Vec3::splat(0.5)),
            Visibility::default(),
            DespawnOnExit(GameState::Gameplay),
            Name::new(format!("Falling Block ({})", cell.block_id)),
        ))
        .with_children(|root| {
            for world_face in BlockFace::ALL {
                let source_face = source_face_for_cell_visual(world_face, cell, block);
                let layer_count = maximum_block_model_layers(&content.blocks, source_face);
                for layer_index in 0..layer_count {
                    let Some(mut material) = block_face_material_data(
                        source_face,
                        layer_index,
                        block,
                        &content.asset_server,
                        1.0,
                    ) else {
                        continue;
                    };
                    set_block_model_tint(&mut material, tint);
                    root.spawn((
                        Mesh3d(block_meshes.world_face_for_block(world_face, block)),
                        MeshMaterial3d(materials.add(material)),
                    ));
                }
            }
        });
}

fn simulate_falling_blocks(
    time: Res<Time>,
    dimension: CurrentDimensionContext,
    mut runtime: VoxelTopologyRuntime,
    mut commands: Commands,
    mut falling: Query<(Entity, &mut Transform, &mut FallingBlock)>,
) {
    let Some(dimension) = dimension.definition() else { return; };
    let dt = time.delta_secs().min(0.05);
    if dt <= 0.0 {
        return;
    }

    for (entity, mut transform, mut block) in &mut falling {
        if dimension.gravity_strength <= 0.0 {
            block.velocity_y = 0.0;
            continue;
        }

        block.velocity_y -= dimension.gravity_strength * dt;
        let target_y = transform.translation.y + block.velocity_y * dt;
        let current_bottom = transform.translation.y - 0.5;
        let target_bottom = target_y - 0.5;
        let highest_support = (current_bottom - 0.0001).floor() as i32;
        let lowest_support = (target_bottom - 0.0001).floor() as i32;
        let mut landing = None;
        let mut blocked_by_unloaded = false;

        for support_y in (lowest_support..=highest_support).rev() {
            if support_y < 0 {
                landing = Some(IVec3::new(block.column.x, 0, block.column.y));
                break;
            }
            let support = IVec3::new(block.column.x, support_y, block.column.y);
            if !runtime.read().is_loaded_at(support) {
                blocked_by_unloaded = true;
                break;
            }
            if runtime.read().cell_at(support).is_some() {
                landing = Some(support + IVec3::Y);
                break;
            }
        }

        if blocked_by_unloaded {
            block.velocity_y = 0.0;
            continue;
        }
        if let Some(voxel) = landing {
            transform.translation.y = voxel.y as f32 + 0.5;
            block.velocity_y = 0.0;
            if runtime.set_block(voxel, Some(block.cell)).is_some() {
                commands.entity(entity).despawn();
            }
            continue;
        }

        transform.translation.y = target_y;
    }
}

fn stackable_layer_drop_stack(cell: VoxelCell) -> ItemStack {
    let quantity = stackable_layer_count(cell).unwrap_or(1) as u32;
    ItemStack::new(cell.block_id).with_quantity(quantity)
}

fn emit_detached_object_drops(
    position: IVec3,
    previous_cell: Option<VoxelCell>,
    detached_objects: super::chunk::ObjectCells,
    objects: &ObjectRegistry,
    item_spawns: &mut MessageWriter<WorldItemSpawnRequest>,
) {
    for object in detached_objects {
        if let Some(drop) = detached_object_drop_request(position, previous_cell, object, objects) {
            item_spawns.write(drop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        content::block_orientation::BlockOrientation,
        voxel::{stackable_layer::stackable_layer_mask, texture_rotation::TextureRotation},
    };

    #[test]
    fn voxel_edit_wakes_changed_voxel_and_block_above_once() {
        let mut pending = PendingBlockGravityUpdates::default();
        let position = IVec3::new(3, 7, -2);
        pending.enqueue_voxel_edit(position);
        pending.enqueue_voxel_edit(position);
        assert_eq!(pending.take_batch(), vec![position, position + IVec3::Y]);
        assert!(pending.take_batch().is_empty());
    }

    #[test]
    fn negative_world_positions_are_not_queued() {
        let mut pending = PendingBlockGravityUpdates::default();
        pending.enqueue(IVec3::new(0, -1, 0));
        assert!(pending.take_batch().is_empty());
    }

    #[test]
    fn stackable_layer_drop_preserves_layer_count() {
        let cell = stackable_layer_mask(4).apply_to_cell(
            VoxelCell::oriented(
                "asteria:snow_layer",
                TextureRotation::default(),
                BlockOrientation::Y,
            ),
            false,
        );
        let stack = stackable_layer_drop_stack(cell);
        assert_eq!(stack.id(), "asteria:snow_layer");
        assert_eq!(stack.quantity(), 4);
    }
}
