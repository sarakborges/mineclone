use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera, voxel::chunk::CHUNK_SIZE,
    world::render_distance::RenderDistanceSettings,
};

const FOG_START_RADIUS_FRACTION: f32 = 0.55;
const FOG_END_RADIUS_FRACTION: f32 = 0.90;

pub(super) fn fog_distances(render_distance_chunks: i32) -> (f32, f32) {
    let chunk_size = CHUNK_SIZE as f32;
    let radius = render_distance_chunks.max(1) as f32 * chunk_size;
    let start = (radius * FOG_START_RADIUS_FRACTION).max(chunk_size);
    let end = (radius * FOG_END_RADIUS_FRACTION).max(start + chunk_size);

    (start, end)
}

pub(super) fn fog_falloff(render_distance_chunks: i32) -> FogFalloff {
    let (start, end) = fog_distances(render_distance_chunks);
    FogFalloff::Linear { start, end }
}

pub(super) fn update_fog_distance(
    render_distance: Res<RenderDistanceSettings>,
    mut fogs: Query<&mut DistanceFog, With<GameplayCamera>>,
) {
    if !render_distance.is_changed() {
        return;
    }

    for mut fog in &mut fogs {
        fog.falloff = fog_falloff(render_distance.chunks());
    }
}
