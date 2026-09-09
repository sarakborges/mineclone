use bevy::prelude::*;

use crate::app::game_state::GameState;

use super::environment::EnvironmentVisualState;

const MIN_AMBIENT_BRIGHTNESS: f32 = 6.0;
const MAX_AMBIENT_BRIGHTNESS: f32 = 220.0;
const DAYLIGHT_REFERENCE_ILLUMINANCE: f32 = 40_000.0;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_sky_light)
            .add_systems(
                PostUpdate,
                update_sky_light.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct SkyLight;

fn spawn_sky_light(mut commands: Commands) {
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::NEG_Y);

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(rotation),
        SkyLight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_sky_light(
    visuals: Res<EnvironmentVisualState>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    mut sky_light: Single<&mut DirectionalLight, With<SkyLight>>,
) {
    let daylight =
        (visuals.light_illuminance / DAYLIGHT_REFERENCE_ILLUMINANCE).clamp(0.0, 1.0);
    let ambient_daylight = daylight.sqrt();

    ambient_light.color = visuals.light_color;
    ambient_light.brightness = MIN_AMBIENT_BRIGHTNESS
        + (MAX_AMBIENT_BRIGHTNESS - MIN_AMBIENT_BRIGHTNESS) * ambient_daylight;
    sky_light.color = visuals.light_color;
    sky_light.illuminance = visuals.light_illuminance;
}
