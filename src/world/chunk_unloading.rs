use bevy::prelude::*;

use crate::{
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
    voxel::{
        coordinates::split_dimension_position, lighting::relight_after_chunk_unloads,
        neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
};

use super::{
    chunk_rendering::refresh_chunk_mesh,
    chunk_system_params::{ChunkContent, ChunkRenderer},
    render_distance::RenderDistanceSettings,
};

pub fn unload_chunk_meshes(
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    content: ChunkContent,
    mut renderer: ChunkRenderer,
    mut world: ResMut<VoxelWorld>,
) {
    let feet_position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_chunk = split_dimension_position(feet_position).chunk;
    let center = IVec3::new(player_chunk.x, player_chunk.y.max(0), player_chunk.z);
    let horizontal_radius = render_distance.chunks();
    let vertical_radius = render_distance.vertical_chunks();
    let horizontal_radius_squared = horizontal_radius * horizontal_radius;
    let to_unload = renderer
        .pool
        .active_coords()
        .filter(|coord| {
            let delta = *coord - center;
            let outside_horizontal =
                delta.x * delta.x + delta.z * delta.z > horizontal_radius_squared;
            let outside_vertical = delta.y.abs() > vertical_radius;

            outside_horizontal || outside_vertical
        })
        .collect::<Vec<_>>();

    for coord in &to_unload {
        let Some((entities, mesh_handles)) = renderer.pool.take(*coord) else {
            continue;
        };

        for mesh_handle in mesh_handles {
            let _ = renderer.meshes.remove(&mesh_handle);
        }

        for entity in entities {
            renderer.commands.entity(entity).despawn();
        }

        world.archive_chunk(*coord);
    }

    let mut chunks_to_remesh =
        relight_after_chunk_unloads(&mut world, &to_unload, &content.blocks, &content.fluids);

    for coord in &to_unload {
        for offset in CARDINAL_NEIGHBORS {
            let neighbor = *coord + offset;
            if renderer.pool.contains(neighbor) {
                chunks_to_remesh.insert(neighbor);
            }
        }
    }

    let render_context = content.render_context(
        &world,
        &renderer.terrain_materials,
        &renderer.fluid_materials,
    );

    for coord in chunks_to_remesh {
        refresh_chunk_mesh(
            &mut renderer.commands,
            &mut renderer.meshes,
            &mut renderer.pool,
            coord,
            &render_context,
        );
    }
}
