use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::camera::GameplayCamera,
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::RENDER_DISTANCE_RADIUS,
};

use super::environment::EnvironmentVisualState;

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (attach_fog, update_fog_color)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn attach_fog(
    mut commands: Commands,
    visuals: Res<EnvironmentVisualState>,
    cameras: Query<Entity, (With<GameplayCamera>, Without<DistanceFog>)>,
) {
    let chunk_size = CHUNK_SIZE as f32;
    let fog_end = RENDER_DISTANCE_RADIUS as f32 * chunk_size;
    let fog_start = (RENDER_DISTANCE_RADIUS as f32 - 1.0) * chunk_size;

    for entity in &cameras {
        commands.entity(entity).insert(DistanceFog {
            color: visuals.fog_color,
            falloff: FogFalloff::Linear {
                start: fog_start,
                end: fog_end,
            },
            ..default()
        });
    }
}

fn update_fog_color(
    visuals: Res<EnvironmentVisualState>,
    mut fogs: Query<&mut DistanceFog, With<GameplayCamera>>,
) {
    for mut fog in &mut fogs {
        fog.color = visuals.fog_color;
    }
}
