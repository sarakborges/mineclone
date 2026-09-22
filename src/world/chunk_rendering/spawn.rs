use bevy::{
    camera::visibility::NoCpuCulling,
    light::{NotShadowCaster, NotShadowReceiver},
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
        mesh_lighting::ChunkLightingCache,
        meshlet::ChunkMeshletMask,
        read::VoxelRead,
    },
};

use super::{
    ChunkMeshBuildContext, ChunkRenderContext, ChunkRenderCoord,
    pool::{ChunkMeshKey, ChunkRenderAllocation, ChunkRenderPool},
};

#[derive(Default)]
struct ColumnTintCache {
    grass: Option<Vec<Option<Color>>>,
    leaf: Option<Vec<Option<Color>>>,
    foliage: Option<Vec<Option<Color>>>,
}

impl ColumnTintCache {
    fn get_or_insert_with(
        &mut self,
        voxel: IVec3,
        tint: BlockTint,
        make: impl FnOnce() -> Color,
    ) -> Color {
        let local_x = voxel.x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let local_z = voxel.z.rem_euclid(CHUNK_SIZE as i32) as usize;
        let index = local_x + local_z * CHUNK_SIZE;
        let cache = match tint {
            BlockTint::None => return Color::WHITE,
            BlockTint::Grass => &mut self.grass,
            BlockTint::Leaf => &mut self.leaf,
            BlockTint::Foliage => &mut self.foliage,
        };
        let cache = cache
            .get_or_insert_with(|| vec![None; CHUNK_SIZE * CHUNK_SIZE]);
        cache[index].get_or_insert_with(make).clone()
    }
}

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
    if chunk.is_empty() {
        return Vec::new();
    }

    let lighting_cache = ChunkLightingCache::capture_if_worthwhile(
        context.world,
        coord * CHUNK_SIZE as i32,
        chunk,
    );
    let terrain_meshes = if chunk.has_terrain_content() {
        build_chunk_terrain_render_meshlets_with_lighting(
            coord,
            chunk,
            context,
            ChunkMeshletMask::ALL,
            lighting_cache.as_ref(),
        )
    } else {
        Vec::new()
    };
    let fluid_meshes = if chunk.has_fluid() {
        build_chunk_fluid_render_meshlets_with_lighting(
            coord,
            chunk,
            context,
            ChunkMeshletMask::ALL,
            lighting_cache.as_ref(),
        )
    } else {
        Vec::new()
    };

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
    if meshlets.is_all() {
        let lighting_cache = ChunkLightingCache::capture_if_worthwhile(
            context.world,
            coord * CHUNK_SIZE as i32,
            chunk,
        );
        return build_chunk_terrain_render_meshlets_with_lighting(
            coord,
            chunk,
            context,
            meshlets,
            lighting_cache.as_ref(),
        );
    }

    build_chunk_terrain_render_meshlets_with_lighting(
        coord,
        chunk,
        context,
        meshlets,
        None,
    )
}

fn build_chunk_terrain_render_meshlets_with_lighting<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
    meshlets: ChunkMeshletMask,
    lighting_cache: Option<&ChunkLightingCache>,
) -> Vec<BuiltChunkMesh> {
    if !chunk.has_terrain_content() || meshlets.is_empty() {
        return Vec::new();
    }

    let mut column_tints = ColumnTintCache::default();
    let mut meshes = build_chunk_meshlets(
        context.world,
        coord,
        chunk,
        context.blocks,
        context.texture_table,
        meshlets,
        lighting_cache,
        |voxel, cell, block| {
            let base_tint = if block.tint == BlockTint::None {
                Color::WHITE
            } else {
                column_tints.get_or_insert_with(voxel, block.tint, || {
                    let position =
                        Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
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
            lighting_cache,
            |voxel, definition| {
                let color = if definition.tint == BlockTint::None {
                    Color::WHITE
                } else {
                    column_tints.get_or_insert_with(
                        voxel,
                        definition.tint,
                        || {
                            let position =
                                Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
                            block_tint_at(
                                definition.tint,
                                position,
                                context.biome_field,
                                context.biomes,
                            )
                        },
                    )
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
    if meshlets.is_all() {
        let lighting_cache = ChunkLightingCache::capture_if_worthwhile(
            context.world,
            coord * CHUNK_SIZE as i32,
            chunk,
        );
        return build_chunk_fluid_render_meshlets_with_lighting(
            coord,
            chunk,
            context,
            meshlets,
            lighting_cache.as_ref(),
        );
    }

    build_chunk_fluid_render_meshlets_with_lighting(
        coord,
        chunk,
        context,
        meshlets,
        None,
    )
}

fn build_chunk_fluid_render_meshlets_with_lighting<W: VoxelRead + ?Sized>(
    coord: IVec3,
    chunk: &VoxelChunk,
    context: &ChunkMeshBuildContext<'_, W>,
    meshlets: ChunkMeshletMask,
    lighting_cache: Option<&ChunkLightingCache>,
) -> Vec<ChunkFluidMesh> {
    if !chunk.has_fluid() || meshlets.is_empty() {
        return Vec::new();
    }

    let mut column_tints =
        vec![None::<Vec<Option<[f32; 3]>>>; context.fluids.iter().count()];
    build_fluid_meshlets(
        context.world,
        coord,
        chunk,
        meshlets,
        lighting_cache,
        |voxel, fluid_id| {
            let local_x = voxel.x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let local_z = voxel.z.rem_euclid(CHUNK_SIZE as i32) as usize;
            let column_index = local_x + local_z * CHUNK_SIZE;
            let fluid_index = usize::from(fluid_id);
            let cache = column_tints[fluid_index]
                .get_or_insert_with(|| vec![None; CHUNK_SIZE * CHUNK_SIZE]);
            let slot = &mut cache[column_index];

            *slot.get_or_insert_with(|| {
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
        },
    )
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
            DespawnOnExit(GameState::Gameplay),
        ));

        if geometry_uses_gpu_culling(key, context) {
            entity_commands.insert(NoCpuCulling);
        }

        if !casts_shadow || layer_index > 0 {
            entity_commands.insert(NotShadowCaster);
        }

        entities.push(entity_commands.id());
    }

    (entities, mesh_handle)
}

fn geometry_uses_gpu_culling(
    key: ChunkMeshKey,
    context: &ChunkRenderContext<'_>,
) -> bool {
    match key {
        ChunkMeshKey::TerrainArray { alpha_blend, .. } => !alpha_blend,
        ChunkMeshKey::TerrainLegacy { block_id, .. } => {
            !context
                .blocks
                .get(block_id)
                .unwrap_or_else(|| panic!("missing block definition for {block_id}"))
                .alpha_blend
        }
        ChunkMeshKey::Layer { layer_id, .. } => {
            !context
                .layers
                .get(layer_id)
                .unwrap_or_else(|| panic!("missing layer definition for {layer_id}"))
                .alpha_blend
        }
        ChunkMeshKey::Fluid(_) => false,
    }
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
            NotShadowCaster,
            NotShadowReceiver,
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
