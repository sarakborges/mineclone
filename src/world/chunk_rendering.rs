use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::biome::BiomeRegistry,
    voxel::{
        chunk::{VoxelChunk, CHUNK_SIZE},
        mesh::build_chunk_mesh,
        world::VoxelWorld,
    },
};

use super::biome_field::BiomeField;

#[derive(Resource, Clone)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

pub fn spawn_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &VoxelWorld,
    coord: IVec3,
    chunk: &VoxelChunk,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    material: &Handle<StandardMaterial>,
) {
    if chunk.is_empty() {
        return;
    }

    let mesh = meshes.add(build_chunk_mesh(world, coord, chunk, |voxel| {
        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let grass = biome_field.grass_color(position, biomes);
        [grass.r, grass.g, grass.b]
    }));
    let chunk_size = CHUNK_SIZE as f32;

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(coord.as_vec3() * chunk_size),
        DespawnOnExit(GameState::Gameplay),
    ));
}
