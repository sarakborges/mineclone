use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, block::BlockRegistry},
    voxel::world::VoxelWorld,
    world::biome_field::BiomeField,
};

use super::{
    materials::{FluidMaterials, TerrainMaterials},
    pool::ChunkRenderPool,
    spawn::spawn_chunk_mesh,
};

const CHUNK_NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub fn refresh_adjacent_chunk_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    world: &VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    terrain_materials: &TerrainMaterials,
    fluid_materials: &FluidMaterials,
) {
    for offset in CHUNK_NEIGHBORS {
        let neighbor = coord + offset;

        if render_pool.contains(neighbor) {
            refresh_chunk_mesh(
                commands,
                meshes,
                render_pool,
                world,
                neighbor,
                blocks,
                biomes,
                biome_field,
                terrain_materials,
                fluid_materials,
            );
        }
    }
}

pub fn refresh_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    world: &VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    terrain_materials: &TerrainMaterials,
    fluid_materials: &FluidMaterials,
) {
    if !render_pool.contains(coord) {
        return;
    }

    let Some(chunk) = world.chunk(coord) else {
        return;
    };

    if let Some((entities, mesh_handles)) = render_pool.take(coord) {
        for entity in entities {
            commands.entity(entity).despawn();
        }
        for handle in mesh_handles {
            let _ = meshes.remove(&handle);
        }
    }

    spawn_chunk_mesh(
        commands,
        meshes,
        render_pool,
        world,
        coord,
        chunk,
        blocks,
        biomes,
        biome_field,
        terrain_materials,
        fluid_materials,
    );
}
