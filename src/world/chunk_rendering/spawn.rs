use bevy::{light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    rendering::block_tint::{block_tint_at, block_vertex_tint},
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid_mesh::{ChunkFluidMesh, build_fluid_meshes},
        mesh::{ChunkFaceMesh, build_chunk_mesh},
        read::VoxelRead,
    },
};

use super::{
    ChunkMeshBuildContext, ChunkRenderContext,
    pool::{ChunkMeshKey, ChunkRenderAllocation, ChunkRenderPool},
};

pub(crate) enum BuiltChunkMesh {
    Terrain(ChunkFaceMesh),
    Fluid(ChunkFluidMesh),
}

impl BuiltChunkMesh {
    pub(super) fn key(&self) -> ChunkMeshKey {
        match self {
            Self::Terrain(mesh) => ChunkMeshKey::Terrain {
                block_id: mesh.block_id,
                face: mesh.face,
                casts_shadow: mesh.casts_shadow,
            },
            Self::Fluid(mesh) => ChunkMeshKey::Fluid(mesh.fluid_id),
        }
    }

    pub(super) fn mesh(&self) -> &Mesh {
        match self {
            Self::Terrain(mesh) => &mesh.mesh,
            Self::Fluid(mesh) => &mesh.mesh,
        }
    }

    pub(super) fn into_mesh(self) -> Mesh {
        match self {
            Self::Terrain(mesh) => mesh.mesh,
            Self::Fluid(mesh) => mesh.mesh,
        }
    }
}

pub(crate) fn build_chunk_render_meshes<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
) -> Vec<BuiltChunkMesh> {
    let terrain_meshes = build_chunk_terrain_render_meshes(coord, chunk, context);
    let fluid_meshes = build_chunk_fluid_render_meshes(coord, chunk, context);

    terrain_meshes
        .into_iter()
        .chain(fluid_meshes.into_iter().map(BuiltChunkMesh::Fluid))
        .collect()
}

pub(super) fn build_chunk_terrain_render_meshes<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
) -> Vec<BuiltChunkMesh> {
    build_chunk_mesh(
        context.world,
        coord,
        chunk,
        context.blocks,
        |voxel, cell| {
            let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
            let block = context
                .blocks
                .get(cell.block_id)
                .unwrap_or_else(|| panic!("missing block definition: {}", cell.block_id));
            let base_tint =
                block_tint_at(block.tint, position, context.biome_field, context.biomes);

            block_vertex_tint(base_tint, block, cell, context.secondary_properties)
        },
    )
    .into_iter()
    .map(BuiltChunkMesh::Terrain)
    .collect()
}

pub(super) fn build_chunk_fluid_render_meshes<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
) -> Vec<ChunkFluidMesh> {
    build_fluid_meshes(context.world, coord, chunk, |voxel, fluid_id| {
        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let fluid = context
            .fluids
            .get(fluid_id)
            .unwrap_or_else(|| panic!("missing fluid definition for id {fluid_id}"));

        context
            .biome_field
            .water_color(position, context.biomes, fluid.color)
            .to_srgb()
    })
}

pub fn spawn_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkRenderContext<'_>,
) {
    if render_pool.contains(coord) {
        return;
    }

    if chunk.is_empty() {
        render_pool.insert(coord, ChunkRenderAllocation::default());
        return;
    }

    let build_context = context.mesh_build_context();
    let built_meshes = build_chunk_render_meshes(coord, chunk, &build_context);
    spawn_built_chunk_meshes(commands, meshes, render_pool, coord, built_meshes, context);
}

pub(crate) fn spawn_built_chunk_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    built_meshes: Vec<BuiltChunkMesh>,
    context: &ChunkRenderContext<'_>,
) {
    if render_pool.contains(coord) {
        return;
    }

    if built_meshes.is_empty() {
        render_pool.insert(coord, ChunkRenderAllocation::default());
        return;
    }

    let transform = Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32);
    let mut entities = Vec::new();
    let mut mesh_handles = Vec::new();
    let mut mesh_keys = Vec::new();
    let mut fluid_ids = Vec::new();
    let mut pooled_mesh_bytes = 0;
    let mut fluid_mesh_bytes = 0;

    for built_mesh in built_meshes {
        mesh_keys.push(built_mesh.key());

        match built_mesh {
            BuiltChunkMesh::Terrain(face_mesh) => {
                pooled_mesh_bytes += mesh_asset_bytes(&face_mesh.mesh);
                let mesh_handle = meshes.add(face_mesh.mesh);
                let layer_materials = context
                    .terrain_materials
                    .for_face(face_mesh.block_id, face_mesh.face);

                for (layer_index, material) in layer_materials.iter().enumerate() {
                    let mut entity_commands = commands.spawn((
                        Mesh3d(mesh_handle.clone()),
                        MeshMaterial3d(material.clone()),
                        transform,
                        DespawnOnExit(GameState::Gameplay),
                    ));

                    if !face_mesh.casts_shadow || layer_index > 0 {
                        entity_commands.insert(NotShadowCaster);
                    }

                    entities.push(entity_commands.id());
                }

                mesh_handles.push(mesh_handle);
            }
            BuiltChunkMesh::Fluid(fluid_mesh) => {
                let bytes = mesh_asset_bytes(&fluid_mesh.mesh);
                pooled_mesh_bytes += bytes;
                fluid_mesh_bytes += bytes;
                fluid_ids.push(fluid_mesh.fluid_id);

                let mesh_handle = meshes.add(fluid_mesh.mesh);
                let entity = commands
                    .spawn((
                        Mesh3d(mesh_handle.clone()),
                        MeshMaterial3d(context.fluid_materials.get(fluid_mesh.fluid_id).clone()),
                        transform,
                        NotShadowCaster,
                        DespawnOnExit(GameState::Gameplay),
                    ))
                    .id();
                entities.push(entity);
                mesh_handles.push(mesh_handle);
            }
        }
    }

    render_pool.insert(
        coord,
        ChunkRenderAllocation {
            entities,
            meshes: mesh_handles,
            mesh_keys,
            fluid_ids,
            mesh_bytes: pooled_mesh_bytes,
            fluid_mesh_bytes,
        },
    );
}

pub(super) fn mesh_asset_bytes(mesh: &Mesh) -> usize {
    mesh.get_vertex_buffer_size()
        + mesh
            .get_index_buffer_bytes()
            .map_or(0, |indices| indices.len())
}
