use bevy::prelude::*;

use super::{
    ChunkRenderContext,
    pool::ChunkRenderPool,
    spawn::{
        BuiltChunkMesh, build_chunk_fluid_render_meshes, build_chunk_render_meshes,
        mesh_asset_bytes, spawn_chunk_mesh,
    },
};

pub fn refresh_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
    let Some(chunk) = context.world.chunk(coord) else {
        return;
    };

    if let Some((entities, mesh_handles)) = render_pool.take(coord) {
        for entity in entities {
            commands.entity(entity).despawn();
        }

        if !mesh_handles.is_empty() {
            commands.queue(move |world: &mut World| {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                for handle in mesh_handles {
                    let _ = meshes.remove(&handle);
                }
            });
        }
    }

    spawn_chunk_mesh(commands, meshes, render_pool, coord, chunk, context);
}

pub fn refresh_chunk_geometry_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
    let Some(chunk) = context.world.chunk(coord) else {
        return;
    };
    if chunk.is_empty() {
        if render_pool.contains(coord) {
            refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
        }
        return;
    }

    let build_context = context.mesh_build_context();
    let built_meshes = build_chunk_render_meshes(coord, chunk, &build_context);
    let replacement_keys = built_meshes
        .iter()
        .map(BuiltChunkMesh::key)
        .collect::<Vec<_>>();
    let mesh_bytes = built_meshes
        .iter()
        .map(|built| mesh_asset_bytes(built.mesh()))
        .sum();
    let replacements = built_meshes
        .into_iter()
        .map(BuiltChunkMesh::into_mesh)
        .collect::<Vec<_>>();

    if render_pool.replace_mesh_assets(
        coord,
        meshes,
        &replacement_keys,
        replacements,
        mesh_bytes,
    ) {
        return;
    }

    refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
}

pub fn refresh_chunk_lighting_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
    refresh_chunk_geometry_mesh(commands, meshes, render_pool, coord, context);
}

pub fn refresh_chunk_fluid_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    coord: IVec3,
    context: &ChunkRenderContext<'_>,
) {
    let Some(chunk) = context.world.chunk(coord) else {
        return;
    };

    let build_context = context.mesh_build_context();
    let fluid_meshes = build_chunk_fluid_render_meshes(coord, chunk, &build_context);
    let fluid_mesh_bytes = fluid_meshes
        .iter()
        .map(|fluid| mesh_asset_bytes(&fluid.mesh))
        .sum();
    let replacements = fluid_meshes
        .into_iter()
        .map(|fluid| (fluid.fluid_id, fluid.mesh))
        .collect::<Vec<_>>();

    if render_pool.replace_fluid_mesh_assets(
        coord,
        meshes,
        replacements,
        fluid_mesh_bytes,
    ) {
        return;
    }

    refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
}
