use bevy::{light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    rendering::block_tint::block_tint_at,
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
        render_pool.insert(coord, Vec::new(), Vec::new());
        return;
    }

    let face_meshes = build_chunk_mesh(
        context.world,
        coord,
        chunk,
        context.blocks,
        |voxel, block_id| {
            let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
            let tint =
                block_tint_at(block_id, position, context.biome_field, context.biomes).to_srgba();
            [tint.red, tint.green, tint.blue]
        },
    );
    let fluid_meshes = build_fluid_meshes(context.world, coord, chunk);
    let transform = Transform::from_translation(coord.as_vec3() * CHUNK_SIZE as f32);
    let mut entities = Vec::new();
    let mut mesh_handles = Vec::new();

    for face_mesh in face_meshes {
        let material = context
            .terrain_materials
            .for_face(face_mesh.block_id, face_mesh.face)
            .clone();
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

    render_pool.insert(coord, entities, mesh_handles);
}
