use bevy::prelude::*;

use crate::app::game_state::GameState;

use super::environment::EnvironmentVisualState;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_sun)
            .add_systems(
                PostUpdate,
                update_lighting.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct SunLight;

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.20,
            ..default()
        },
        Transform::default(),
        SunLight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_lighting(
    visuals: Res<EnvironmentVisualState>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    mut sun: Single<(&mut DirectionalLight, &mut Transform), With<SunLight>>,
) {
    ambient_light.color = visuals.ambient_color;
    ambient_light.brightness = visuals.ambient_brightness;

    sun.0.color = visuals.sun_color;
    sun.0.illuminance = visuals.sun_illuminance;
    sun.1.rotation = visuals.sun_rotation;
}
