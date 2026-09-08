use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::camera::GameplayCamera,
    voxel::chunk::CHUNK_SIZE,
};
use crate::world::render_distance::RENDER_DISTANCE_RADIUS;

const FOG_COLOR: Color = Color::srgb(0.02, 0.025, 0.04);

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_fog.run_if(in_state(GameState::Gameplay)));
    }
}

fn attach_fog(
    mut commands: Commands,
    cameras: Query<Entity, (With<GameplayCamera>, Without<DistanceFog>)>,
) {
    let chunk_size = CHUNK_SIZE as f32;
    let fog_end = RENDER_DISTANCE_RADIUS as f32 * chunk_size;
    let fog_start = (RENDER_DISTANCE_RADIUS as f32 - 1.0) * chunk_size;

    for entity in &cameras {
        commands.entity(entity).insert(DistanceFog {
            color: FOG_COLOR,
            falloff: FogFalloff::Linear {
                start: fog_start,
                end: fog_end,
            },
            ..default()
        });
    }
}
