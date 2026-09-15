use bevy::prelude::*;

use super::{
    ChunkRenderContext,
    pool::{
        ChunkRenderPool, retire_chunk_render_allocation, retire_render_allocation_parts,
    },
    spawn::{
        BuiltChunkMesh, build_chunk_fluid_render_meshes, build_chunk_terrain_render_meshes,
        mesh_asset_bytes, spawn_chunk_mesh, spawn_fluid_meshes_into_existing_allocation,
        spawn_terrain_meshes_into_existing_allocation,
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

    retire_chunk_render_allocation(commands, render_pool, coord);
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
    let built_meshes = build_chunk_terrain_render_meshes(coord, chunk, &build_context);
    let replacement_keys = built_meshes
        .iter()
        .map(BuiltChunkMesh::key)
        .collect::<Vec<_>>();
    let terrain_mesh_bytes = built_meshes
        .iter()
        .map(|built| mesh_asset_bytes(built.mesh()))
        .sum();
    let mut replacements = built_meshes
        .into_iter()
        .map(BuiltChunkMesh::into_mesh)
        .collect::<Vec<_>>();

    if render_pool.replace_terrain_mesh_assets(
        coord,
        meshes,
        &replacement_keys,
        &mut replacements,
        terrain_mesh_bytes,
    ) {
        return;
    }

    let Some(detached) = render_pool.detach_terrain_render_allocation(coord) else {
        refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
        return;
    };
    retire_render_allocation_parts(commands, detached.entities, detached.meshes);
    spawn_terrain_meshes_into_existing_allocation(
        commands,
        meshes,
        render_pool,
        coord,
        replacement_keys,
        replacements,
        terrain_mesh_bytes,
        context,
    );
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
    let mut replacements = fluid_meshes
        .into_iter()
        .map(|fluid| (fluid.fluid_id, fluid.mesh))
        .collect::<Vec<_>>();

    if render_pool.replace_fluid_mesh_assets(
        coord,
        meshes,
        &mut replacements,
        fluid_mesh_bytes,
    ) {
        return;
    }

    let Some(detached) = render_pool.detach_fluid_render_allocation(coord) else {
        refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
        return;
    };
    retire_render_allocation_parts(commands, detached.entities, detached.meshes);
    spawn_fluid_meshes_into_existing_allocation(
        commands,
        meshes,
        render_pool,
        coord,
        replacements,
        fluid_mesh_bytes,
        context,
    );
}
