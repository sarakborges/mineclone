use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::RenderDistanceSettings,
};

pub(super) fn fog_falloff(render_distance_chunks: i32) -> FogFalloff {
    let chunk_size = CHUNK_SIZE as f32;
    let radius = render_distance_chunks as f32;
    let end = radius * chunk_size;
    let start = (radius - 1.0).max(0.0) * chunk_size;

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
