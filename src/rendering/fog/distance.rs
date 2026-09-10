use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera, voxel::chunk::CHUNK_SIZE,
    world::render_distance::RenderDistanceSettings,
};

const FOG_START_MARGIN_CHUNKS: f32 = 2.0;
const FOG_END_MARGIN_CHUNKS: f32 = 1.25;

pub(super) fn fog_falloff(render_distance_chunks: i32) -> FogFalloff {
    let chunk_size = CHUNK_SIZE as f32;
    let radius = render_distance_chunks as f32;
    let start_chunks = (radius - FOG_START_MARGIN_CHUNKS).max(1.0);
    let end_chunks = (radius - FOG_END_MARGIN_CHUNKS).max(start_chunks + 0.5);

    FogFalloff::Linear {
        start: start_chunks * chunk_size,
        end: end_chunks * chunk_size,
    }
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
