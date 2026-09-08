use std::collections::HashMap;

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

struct ChunkRenderSlot {
    entity: Entity,
    mesh: Option<Handle<Mesh>>,
}

#[derive(Resource, Default)]
pub struct ChunkRenderPool {
    active: HashMap<IVec3, ChunkRenderSlot>,
    free_mesh_handles: Vec<Handle<Mesh>>,
}

impl ChunkRenderPool {
    pub fn contains(&self, coord: IVec3) -> bool {
        self.active.contains_key(&coord)
    }

    pub fn active_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.active.keys().copied()
    }

    pub fn take(&mut self, coord: IVec3) -> Option<(Entity, Option<Handle<Mesh>>)> {
        self.active
            .remove(&coord)
            .map(|slot| (slot.entity, slot.mesh))
    }

    pub fn recycle_mesh_handle(&mut self, handle: Handle<Mesh>) {
        self.free_mesh_handles.push(handle);
    }

    pub fn clear(&mut self, meshes: &mut Assets<Mesh>) {
        for (_, slot) in self.active.drain() {
            if let Some(handle) = slot.mesh {
                let _ = meshes.remove(&handle);
                self.free_mesh_handles.push(handle);
            }
        }
    }

    fn acquire_mesh_handle(&mut self, meshes: &Assets<Mesh>) -> Handle<Mesh> {
        self.free_mesh_handles
            .pop()
            .unwrap_or_else(|| meshes.reserve_handle())
    }

    fn insert(&mut self, coord: IVec3, entity: Entity, mesh: Option<Handle<Mesh>>) {
        self.active.insert(coord, ChunkRenderSlot { entity, mesh });
    }
}

pub fn spawn_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    world: &VoxelWorld,
    coord: IVec3,
    chunk: &VoxelChunk,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    material: &Handle<StandardMaterial>,
) {
    if render_pool.contains(coord) {
        return;
    }

    if chunk.is_empty() {
        let entity = commands
            .spawn(DespawnOnExit(GameState::Gameplay))
            .id();
        render_pool.insert(coord, entity, None);
        return;
    }

    let mesh = build_chunk_mesh(world, coord, chunk, |voxel| {
        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let grass = biome_field.grass_color(position, biomes);
        [grass.r, grass.g, grass.b]
    });
    let mesh_handle = render_pool.acquire_mesh_handle(meshes);
    meshes
        .insert(&mesh_handle, mesh)
        .expect("reserved chunk mesh handle should remain valid");
    let chunk_size = CHUNK_SIZE as f32;
    let entity = commands
        .spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(coord.as_vec3() * chunk_size),
            DespawnOnExit(GameState::Gameplay),
        ))
        .id();

    render_pool.insert(coord, entity, Some(mesh_handle));
}

pub fn clear_chunk_render_pool(
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    render_pool.clear(&mut meshes);
}
