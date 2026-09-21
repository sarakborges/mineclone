use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{NoAutoAabb, NoCpuCulling},
    },
    light::NotShadowCaster,
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::{block::BlockTint, fluid::FluidId},
    rendering::block_tint::{block_tint_at, block_vertex_tint},
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid_mesh::{ChunkFluidMesh, build_fluid_meshlets},
        layer_mesh::{ChunkLayerMesh, build_layer_meshlets},
        mesh::{ChunkFaceMesh, ChunkTerrainBatch, build_chunk_meshlets},
        meshlet::ChunkMeshletMask,
        read::VoxelRead,
    },
};

use super::{
    ChunkMeshBuildContext, ChunkRenderContext, ChunkRenderCoord,
    pool::{ChunkMeshKey, ChunkRenderAllocation, ChunkRenderPool},
};

pub(crate) enum BuiltChunkMesh {
    Terrain(ChunkFaceMesh),
    Layer(ChunkLayerMesh),
    Fluid(ChunkFluidMesh),
}

impl BuiltChunkMesh {
    pub(super) fn key(&self) -> ChunkMeshKey {
        match self {
            Self::Terrain(mesh) => match mesh.batch {
                ChunkTerrainBatch::Array {
                    alpha_cutoff,
                    alpha_blend,
                    casts_shadow,
                } => ChunkMeshKey::TerrainArray {
                    alpha_cutoff,
                    alpha_blend,
                    casts_shadow,
                },
                ChunkTerrainBatch::Legacy {
                    block_id,
                    face,
                    casts_shadow,
                } => ChunkMeshKey::TerrainLegacy {
                    block_id,
                    face,
                    casts_shadow,
                },
            },
            Self::Layer(mesh) => ChunkMeshKey::Layer {
                layer_id: mesh.layer_id,
                casts_shadow: mesh.casts_shadow,
            },
            Self::Fluid(mesh) => ChunkMeshKey::Fluid(mesh.fluid_id),
        }
    }

    pub(super) fn mesh(&self) -> &Mesh {
        match self {
            Self::Terrain(mesh) => &mesh.mesh,
            Self::Layer(mesh) => &mesh.mesh,
            Self::Fluid(mesh) => &mesh.mesh,
        }
    }

    pub(super) fn into_mesh(self) -> Mesh {
        match self {
            Self::Terrain(mesh) => mesh.mesh,
            Self::Layer(mesh) => mesh.mesh,
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
    build_chunk_terrain_render_meshlets(coord, chunk, context, ChunkMeshletMask::ALL)
}

pub(super) fn build_chunk_terrain_render_meshlets<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
    meshlets: ChunkMeshletMask,
) -> Vec<BuiltChunkMesh> {
    let mut column_tints = HashMap::<(i32, i32, BlockTint), Color>::new();
    let mut meshes = build_chunk_meshlets(
        context.world,
        coord,
        chunk,
        context.blocks,
        context.texture_table,
        meshlets,
        |voxel, cell, block| {
            let base_tint = if block.tint == BlockTint::None {
                Color::WHITE
            } else {
                *column_tints
                    .entry((voxel.x, voxel.z, block.tint))
                    .or_insert_with(|| {
                        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
                        block_tint_at(
                            block.tint,
                            position,
                            context.biome_field,
                            context.biomes,
                        )
                    })
            };

            block_vertex_tint(base_tint, block, cell, context.secondary_properties)
        },
    )
    .into_iter()
    .map(BuiltChunkMesh::Terrain)
    .collect::<Vec<_>>();

    meshes.extend(
        build_layer_meshlets(
            context.world,
            coord,
            chunk,
            context.blocks,
            context.layers,
            meshlets,
            |voxel, definition| {
                let color = if definition.tint == BlockTint::None {
                    Color::WHITE
                } else {
                    *column_tints
                        .entry((voxel.x, voxel.z, definition.tint))
                        .or_insert_with(|| {
                            let position =
                                Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
                            block_tint_at(
                                definition.tint,
                                position,
                                context.biome_field,
                                context.biomes,
                            )
                        })
                };
                let tint = color.to_srgba();
                [tint.red, tint.green, tint.blue]
            },
        )
        .into_iter()
        .map(BuiltChunkMesh::Layer),
    );

    meshes
}

pub(super) fn build_chunk_fluid_render_meshes<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
) -> Vec<ChunkFluidMesh> {
    build_chunk_fluid_render_meshlets(coord, chunk, context, ChunkMeshletMask::ALL)
}

pub(super) fn build_chunk_fluid_render_meshlets<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
    meshlets: ChunkMeshletMask,
) -> Vec<ChunkFluidMesh> {
    let mut column_tints = HashMap::<(i32, i32, FluidId), [f32; 3]>::new();
    build_fluid_meshlets(context.world, coord, chunk, meshlets, |voxel, fluid_id| {
        *column_tints
            .entry((voxel.x, voxel.z, fluid_id))
            .or_insert_with(|| {
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
    let mesh_count = built_meshes.len();
    let mut entities = Vec::with_capacity(mesh_count);
    let mut mesh_handles = Vec::with_capacity(mesh_count);
    let mut mesh_keys = Vec::with_capacity(mesh_count);
    let mut fluid_ids = Vec::new();
    let mut pooled_mesh_bytes = 0;
    let mut fluid_mesh_bytes = 0;

    for built_mesh in built_meshes {
        let key = built_mesh.key();
        mesh_keys.push(key);

        match built_mesh {
            BuiltChunkMesh::Terrain(face_mesh) => {
                pooled_mesh_bytes += mesh_asset_bytes(&face_mesh.mesh);
                let (spawned_entities, mesh_handle) = spawn_geometry_mesh(
                    commands,
                    meshes,
                    coord,
                    transform,
                    key,
                    face_mesh.mesh,
                    context,
                );
                entities.extend(spawned_entities);
                mesh_handles.push(mesh_handle);
            }
            BuiltChunkMesh::Layer(layer_mesh) => {
                pooled_mesh_bytes += mesh_asset_bytes(&layer_mesh.mesh);
                let (spawned_entities, mesh_handle) = spawn_geometry_mesh(
                    commands,
                    meshes,
                    coord,
                    transform,
                    key,
                    layer_mesh.mesh,
                    context,
                );
                entities.extend(spawned_entities);
                mesh_handles.push(mesh_handle);
            }
            BuiltChunkMesh::Fluid(fluid_mesh) => {
                let bytes = mesh_asset_bytes(&fluid_mesh.mesh);
                pooled_mesh_bytes += bytes;
                fluid_mesh_bytes += bytes;
                fluid_ids.push(fluid_mesh.fluid_id);

                let (entity, mesh_handle) = spawn_fluid_mesh(
                    commands,
                    meshes,
                    coord,
                    transform,
                    fluid_mesh.fluid_id,
                    fluid_mesh.mesh,
                    context,
                );
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

#[expect(
    clippy::too_many_arguments,
    reason = "terrain replacement keeps render allocation state explicit and atomic"
)]
pub(super) fn spawn_terrain_meshes_into_existing_allocation(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    replacement_keys: Vec<ChunkMeshKey>,
    replacements: Vec<Mesh>,
    terrain_mesh_bytes: usize,
    context: &ChunkRenderContext<'_>,
) {
    debug_assert_eq!(replacement_keys.len(), replacements.len());

    let transform = Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32);
    let mut entities = Vec::with_capacity(replacements.len());
    let mut mesh_handles = Vec::with_capacity(replacements.len());

    for (key, mesh) in replacement_keys.iter().copied().zip(replacements) {
        let (spawned_entities, mesh_handle) =
            spawn_geometry_mesh(commands, meshes, coord, transform, key, mesh, context);
        entities.extend(spawned_entities);
        mesh_handles.push(mesh_handle);
    }

    render_pool.append_terrain_render_allocation(
        coord,
        entities,
        mesh_handles,
        replacement_keys,
        terrain_mesh_bytes,
    );
}

pub(super) fn spawn_fluid_meshes_into_existing_allocation(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    replacements: Vec<(FluidId, Mesh)>,
    fluid_mesh_bytes: usize,
    context: &ChunkRenderContext<'_>,
) {
    let transform = Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32);
    let mut entities = Vec::with_capacity(replacements.len());
    let mut mesh_handles = Vec::with_capacity(replacements.len());
    let mut fluid_ids = Vec::with_capacity(replacements.len());

    for (fluid_id, mesh) in replacements {
        let (entity, mesh_handle) =
            spawn_fluid_mesh(commands, meshes, coord, transform, fluid_id, mesh, context);
        entities.push(entity);
        mesh_handles.push(mesh_handle);
        fluid_ids.push(fluid_id);
    }

    render_pool.append_fluid_render_allocation(
        coord,
        entities,
        mesh_handles,
        fluid_ids,
        fluid_mesh_bytes,
    );
}

fn chunk_render_aabb() -> Aabb {
    const MARGIN: f32 = 0.01;
    Aabb::from_min_max(
        Vec3::splat(-MARGIN),
        Vec3::splat(CHUNK_SIZE as f32 + MARGIN),
    )
}

fn spawn_geometry_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    coord: IVec3,
    transform: Transform,
    key: ChunkMeshKey,
    mesh: Mesh,
    context: &ChunkRenderContext<'_>,
) -> (Vec<Entity>, Handle<Mesh>) {
    let mesh_handle = meshes.add(mesh);
    let (layer_materials, casts_shadow) = match key {
        ChunkMeshKey::TerrainArray {
            alpha_cutoff,
            alpha_blend,
            casts_shadow,
        } => (
            std::slice::from_ref(
                context
                    .terrain_materials
                    .for_array(alpha_blend, alpha_cutoff),
            ),
            casts_shadow,
        ),
        ChunkMeshKey::TerrainLegacy {
            block_id,
            face,
            casts_shadow,
        } => (
            context.terrain_materials.for_face(block_id, face),
            casts_shadow,
        ),
        ChunkMeshKey::Layer {
            layer_id,
            casts_shadow,
        } => (
            std::slice::from_ref(context.terrain_materials.for_layer(layer_id)),
            casts_shadow,
        ),
        ChunkMeshKey::Fluid(_) => {
            panic!("geometry mesh spawn cannot use a fluid mesh key");
        }
    };
    let mut entities = Vec::with_capacity(layer_materials.len());

    for (layer_index, material) in layer_materials.iter().enumerate() {
        let mut entity_commands = commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material.clone()),
            transform,
            ChunkRenderCoord(coord),
            Visibility::Hidden,
            chunk_render_aabb(),
            NoAutoAabb,
            DespawnOnExit(GameState::Gameplay),
        ));

        if !casts_shadow || layer_index > 0 {
            entity_commands.insert((NotShadowCaster, NoCpuCulling));
        }

        entities.push(entity_commands.id());
    }

    (entities, mesh_handle)
}

fn spawn_fluid_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    coord: IVec3,
    transform: Transform,
    fluid_id: FluidId,
    mesh: Mesh,
    context: &ChunkRenderContext<'_>,
) -> (Entity, Handle<Mesh>) {
    let mesh_handle = meshes.add(mesh);
    let entity = commands
        .spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(context.fluid_materials.get(fluid_id).clone()),
            transform,
            ChunkRenderCoord(coord),
            Visibility::Hidden,
            chunk_render_aabb(),
            NoAutoAabb,
            NoCpuCulling,
            NotShadowCaster,
            DespawnOnExit(GameState::Gameplay),
        ))
        .id();
    (entity, mesh_handle)
}

pub(super) fn mesh_asset_bytes(mesh: &Mesh) -> usize {
    mesh.get_vertex_buffer_size()
        + mesh
            .get_index_buffer_bytes()
            .map_or(0, |indices| indices.len())
}
