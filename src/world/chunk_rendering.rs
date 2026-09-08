use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::biome::BiomeRegistry,
    voxel::{
        chunk::{VoxelChunk, CHUNK_SIZE},
        mesh::{build_chunk_mesh, BlockFace},
        world::VoxelWorld,
    },
};

use super::biome_field::BiomeField;

#[derive(Resource, Clone)]
pub struct TerrainMaterials {
    pub top: Handle<StandardMaterial>,
    pub bottom: Handle<StandardMaterial>,
    pub left: Handle<StandardMaterial>,
    pub right: Handle<StandardMaterial>,
    pub front: Handle<StandardMaterial>,
    pub back: Handle<StandardMaterial>,
}

impl TerrainMaterials {
    fn for_face(&self, face: BlockFace) -> &Handle<StandardMaterial> {
        match face {
            BlockFace::Right => &self.right,
            BlockFace::Left => &self.left,
            BlockFace::Top => &self.top,
            BlockFace::Bottom => &self.bottom,
            BlockFace::Front => &self.front,
            BlockFace::Back => &self.back,
        }
    }
}

struct ChunkRenderSlot {
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
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

    pub fn take(&mut self, coord: IVec3) -> Option<(Vec<Entity>, Vec<Handle<Mesh>>)> {
        self.active
            .remove(&coord)
            .map(|slot| (slot.entities, slot.meshes))
    }

    pub fn recycle_mesh_handle(&mut self, handle: Handle<Mesh>) {
        self.free_mesh_handles.push(handle);
    }

    pub fn clear(&mut self, meshes: &mut Assets<Mesh>) {
        for (_, slot) in self.active.drain() {
            for handle in slot.meshes {
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

    fn insert(
        &mut self,
        coord: IVec3,
        entities: Vec<Entity>,
        meshes: Vec<Handle<Mesh>>,
    ) {
        self.active
            .insert(coord, ChunkRenderSlot { entities, meshes });
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
    materials: &TerrainMaterials,
) {
    if render_pool.contains(coord) {
        return;
    }

    if chunk.is_empty() {
        render_pool.insert(coord, Vec::new(), Vec::new());
        return;
    }

    let face_meshes = build_chunk_mesh(world, coord, chunk, |voxel| {
        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let grass = biome_field.grass_color(position, biomes);
        [grass.r, grass.g, grass.b]
    });
    let chunk_size = CHUNK_SIZE as f32;
    let transform = Transform::from_translation(coord.as_vec3() * chunk_size);
    let mut entities = Vec::with_capacity(face_meshes.len());
    let mut mesh_handles = Vec::with_capacity(face_meshes.len());

    for face_mesh in face_meshes {
        let mesh_handle = render_pool.acquire_mesh_handle(meshes);
        meshes
            .insert(&mesh_handle, face_mesh.mesh)
            .expect("reserved chunk mesh handle should remain valid");

        let entity = commands
            .spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(materials.for_face(face_mesh.face).clone()),
                transform,
                DespawnOnExit(GameState::Gameplay),
            ))
            .id();

        entities.push(entity);
        mesh_handles.push(mesh_handle);
    }

    render_pool.insert(coord, entities, mesh_handles);
}

pub fn clear_chunk_render_pool(
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    render_pool.clear(&mut meshes);
}
