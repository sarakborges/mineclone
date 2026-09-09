use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::RenderDistanceSettings,
};

const FOG_END_MARGIN_CHUNKS: f32 = 1.75;
const FOG_FADE_LENGTH_CHUNKS: f32 = 2.75;

pub(super) fn fog_falloff(render_distance_chunks: i32) -> FogFalloff {
    let chunk_size = CHUNK_SIZE as f32;
    let radius = render_distance_chunks as f32;
    let end = (radius - FOG_END_MARGIN_CHUNKS).max(2.0) * chunk_size;
    let start = (end - FOG_FADE_LENGTH_CHUNKS * chunk_size).max(0.0);

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
