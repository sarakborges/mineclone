use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    content::biome::BiomeObjectSpawnRule,
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        object::ObjectCell,
        texture_rotation::TextureRotation,
    },
    world::deterministic::{hash_string, hash_unit, mix_u32_components},
};

use super::{
    ChunkGenerationContext,
    columns::GenerationColumnSample,
    index::column_index,
};

pub(super) fn rasterize_surface_objects(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    context: &ChunkGenerationContext<'_>,
) {
    let horizontal_min = chunk_origin.xz();
    let horizontal_max = horizontal_min + IVec2::splat(CHUNK_SIZE as i32 - 1);
    let mut occupied = HashSet::<IVec2>::new();

    chunk.edit_structure_content(|chunk| {
        for biome in context.biomes.iter() {
            for spawn in &biome.object_spawns {
                visit_spawn_clusters(
                    context.biome_field.seed(),
                    &biome.id,
                    spawn,
                    horizontal_min,
                    horizontal_max,
                    |position| {
                        if !occupied.insert(position)
                            || position.x < horizontal_min.x
                            || position.x > horizontal_max.x
                            || position.y < horizontal_min.y
                            || position.y > horizontal_max.y
                        {
                            return;
                        }

                        let local_x = (position.x - horizontal_min.x) as usize;
                        let local_z = (position.y - horizontal_min.y) as usize;
                        let column = &columns[column_index(local_x, local_z)];
                        let column_biome = context
                            .biome_field
                            .surface_biome_id(column.identity_surface_index);
                        if column_biome != biome.id {
                            return;
                        }

                        let support_world_y = column.surface_height - 1;
                        let local_y = support_world_y - chunk_origin.y;
                        if !(0..CHUNK_SIZE as i32).contains(&local_y) {
                            return;
                        }
                        let local_y = local_y as usize;
                        let Some(support) = chunk.cell_at(local_x, local_y, local_z) else {
                            return;
                        };
                        if !spawn.ground_blocks.is_empty()
                            && !spawn
                                .ground_blocks
                                .iter()
                                .any(|block| block == support.block_id)
                        {
                            return;
                        }
                        if chunk.fluid_at(local_x, local_y, local_z).is_some() {
                            return;
                        }
                        let object_y = local_y + 1;
                        if object_y < CHUNK_SIZE
                            && chunk.fluid_at(local_x, object_y, local_z).is_some()
                        {
                            return;
                        }

                        let support_world =
                            IVec3::new(position.x, support_world_y, position.y);
                        let object = ObjectCell::new(
                            &spawn.object,
                            crate::content::object::ObjectPlacementFace::Top,
                            TextureRotation::for_position(support_world, true),
                        );
                        let _ = chunk.set_object(local_x, local_y, local_z, object);
                    },
                );
            }
        }
    });
}

fn visit_spawn_clusters(
    world_seed: u64,
    biome_id: &str,
    spawn: &BiomeObjectSpawnRule,
    minimum: IVec2,
    maximum: IVec2,
    mut visit: impl FnMut(IVec2),
) {
    let reach = spawn.cluster_radius + spawn.jitter;
    let min_cell = (minimum - IVec2::splat(reach)).div_euclid(IVec2::splat(spawn.spacing));
    let max_cell = (maximum + IVec2::splat(reach)).div_euclid(IVec2::splat(spawn.spacing));

    for cell_z in min_cell.y..=max_cell.y {
        for cell_x in min_cell.x..=max_cell.x {
            let cell = IVec2::new(cell_x, cell_z);
            let hash = spawn_hash(world_seed, biome_id, &spawn.object, cell);
            if hash_unit(hash) >= spawn.chance {
                continue;
            }

            let jitter_x = signed_jitter(hash.rotate_left(17), spawn.jitter);
            let jitter_z = signed_jitter(hash.rotate_left(41), spawn.jitter);
            let center = cell * spawn.spacing
                + IVec2::splat(spawn.spacing / 2)
                + IVec2::new(jitter_x, jitter_z);

            let count_range = u64::from(spawn.cluster_max - spawn.cluster_min) + 1;
            let count = spawn.cluster_min
                + (hash.rotate_left(7) % count_range) as u8;

            for member in 0..count {
                let member_hash = mix_u32_components(hash.rotate_left(23), [u32::from(member)]);
                let angle = hash_unit(member_hash.rotate_left(11)) * std::f32::consts::TAU;
                let radius = hash_unit(member_hash.rotate_left(37)).sqrt()
                    * spawn.cluster_radius as f32;
                let offset = Vec2::new(angle.cos(), angle.sin()) * radius;
                visit(center + offset.round().as_ivec2());
            }
        }
    }
}

fn spawn_hash(world_seed: u64, biome_id: &str, object_id: &str, cell: IVec2) -> u64 {
    mix_u32_components(
        world_seed ^ hash_string(biome_id).rotate_left(17) ^ hash_string(object_id),
        [cell.x as u32, cell.y as u32],
    )
}

fn signed_jitter(hash: u64, maximum: i32) -> i32 {
    if maximum == 0 {
        return 0;
    }
    let range = (maximum * 2 + 1) as u64;
    (hash % range) as i32 - maximum
}
