use bevy::prelude::*;

use crate::{app::game_state::GameState, player::camera::GameplayCamera};

use super::{dynamic_lights::held_point_light, sun_lighting::sun_directional_light};

pub(super) struct LightingWarmupPlugin;

impl Plugin for LightingWarmupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Loading),
            spawn_directional_light_warmup,
        )
        .add_systems(
            Update,
            spawn_point_light_warmup.run_if(in_state(GameState::Loading)),
        );
    }
}

fn spawn_directional_light_warmup(mut commands: Commands) {
    commands.spawn((
        sun_directional_light(),
        Transform::default(),
        // Zero illuminance keeps this visually inert while still making the
        // PBR lighting path visible to the RenderApp during Loading.
        Visibility::Visible,
        DespawnOnExit(GameState::Loading),
    ));
}

fn spawn_point_light_warmup(
    mut commands: Commands,
    cameras: Query<Entity, Added<GameplayCamera>>,
) {
    for camera in &cameras {
        commands.entity(camera).with_children(|camera| {
            camera.spawn((
                held_point_light(0.0),
                Transform::default(),
                // Keep the zero-intensity light extractable so clustered-light
                // resources are prepared behind the loading transition.
                Visibility::Visible,
                DespawnOnExit(GameState::Loading),
            ));
        });
    }
}
