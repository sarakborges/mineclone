use bevy::{light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    rendering::block_tint::{apply_secondary_property_tint, block_tint_at},
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid_mesh::build_fluid_meshes,
        mesh::build_chunk_mesh,
    },
};

use super::{ChunkRenderContext, pool::ChunkRenderPool};

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
        render_pool.insert(coord, Vec::new(), Vec::new(), 0);
        return;
    }

    let face_meshes = build_chunk_mesh(
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
            let base_tint = block_tint_at(
                block.tint,
                position,
                context.biome_field,
                context.biomes,
            );
            let tint = apply_secondary_property_tint(
                base_tint,
                block,
                cell,
                context.secondary_properties,
            )
            .to_srgba();

            [tint.red, tint.green, tint.blue]
        },
    );
    let fluid_meshes = build_fluid_meshes(context.world, coord, chunk, |voxel, fluid_id| {
        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let fluid = context
            .fluids
            .get(fluid_id)
            .unwrap_or_else(|| panic!("missing fluid definition for id {fluid_id}"));
        let tint = context
            .biome_field
            .water_color(position, context.biomes, fluid.color);

        [tint.r, tint.g, tint.b]
    });
    let transform = Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32);
    let mut entities = Vec::new();
    let mut mesh_handles = Vec::new();
    let mut pooled_mesh_bytes = 0;

    for face_mesh in face_meshes {
        let material = context
            .terrain_materials
            .for_face(face_mesh.block_id, face_mesh.face)
            .clone();
        pooled_mesh_bytes += mesh_asset_bytes(&face_mesh.mesh);
        let mesh_handle = meshes.add(face_mesh.mesh);
        let mut entity_commands = commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material),
            transform,
            DespawnOnExit(GameState::Gameplay),
        ));

        if !face_mesh.casts_shadow {
            entity_commands.insert(NotShadowCaster);
        }

        entities.push(entity_commands.id());
        mesh_handles.push(mesh_handle);
    }

    for fluid_mesh in fluid_meshes {
        pooled_mesh_bytes += mesh_asset_bytes(&fluid_mesh.mesh);
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

    render_pool.insert(coord, entities, mesh_handles, pooled_mesh_bytes);
}

fn mesh_asset_bytes(mesh: &Mesh) -> usize {
    mesh.get_vertex_buffer_size()
        + mesh
            .get_index_buffer_bytes()
            .map_or(0, |indices| indices.len())
}
