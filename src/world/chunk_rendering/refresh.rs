use bevy::prelude::*;

use super::{
    ChunkRenderContext,
    pool::ChunkRenderPool,
    spawn::{
        build_chunk_fluid_render_meshes, build_chunk_render_meshes, mesh_asset_bytes,
        spawn_chunk_mesh,
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

        // Entity despawns are deferred. Removing their mesh assets immediately
        // leaves the still-live render entities without a mesh for the rest of
        // the frame, which becomes visible as flicker during frequent remeshes.
        // Queue asset cleanup after the despawns so extraction only ever sees the
        // old complete mesh set or the newly spawned one.
        if !mesh_handles.is_empty() {
            commands.queue(move |world: &mut World| {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                for handle in mesh_handles {
                    let _ = meshes.remove(&handle);
                }
            });
        }
    }

    // refresh is also the creation path for a resident chunk that has not been
    // rendered yet. This lets streaming defer the only mesh build until after its
    // bounded lighting pass without ever depending on a second streaming frame.
    spawn_chunk_mesh(commands, meshes, render_pool, coord, chunk, context);
}

pub fn refresh_chunk_lighting_mesh(
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
        return;
    }

    let built_meshes = build_chunk_render_meshes(coord, chunk, context);
    let mesh_bytes = built_meshes
        .iter()
        .map(|built| mesh_asset_bytes(built.mesh()))
        .sum();
    let replacements = built_meshes
        .into_iter()
        .map(|built| built.into_mesh())
        .collect::<Vec<_>>();

    // Lighting is stored in vertex attributes, so a lighting-only refresh does
    // not need new render entities. Preserve the existing Mesh handles whenever
    // the stable mesh layout still matches. This keeps shadow casters resident
    // across lighting convergence and avoids a despawn/spawn flicker in both the
    // color and shadow passes.
    if render_pool.replace_mesh_assets(coord, meshes, replacements, mesh_bytes) {
        return;
    }

    // A layout mismatch means geometry actually changed (or the chunk has not
    // been rendered yet). Fall back to the full replacement path in that case.
    refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
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

    let fluid_meshes = build_chunk_fluid_render_meshes(coord, chunk, context);
    let fluid_mesh_bytes = fluid_meshes
        .iter()
        .map(|fluid| mesh_asset_bytes(&fluid.mesh))
        .sum();
    let replacements = fluid_meshes
        .into_iter()
        .map(|fluid| (fluid.fluid_id, fluid.mesh))
        .collect::<Vec<_>>();

    // Flowing water changes geometry frequently, but the terrain meshes in the
    // same chunk are unaffected. Update only the existing fluid mesh assets when
    // the fluid layout is stable; this avoids rebuilding/despawning the complete
    // chunk for every water-level step.
    if render_pool.replace_fluid_mesh_assets(
        coord,
        meshes,
        replacements,
        fluid_mesh_bytes,
    ) {
        return;
    }

    // Fluid appearing/disappearing or changing type requires entity/material
    // layout changes, so use the full path only for those structural transitions.
    refresh_chunk_mesh(commands, meshes, render_pool, coord, context);
}
