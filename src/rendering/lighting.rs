use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    rendering::terrain_material::TerrainMaterial,
};

use super::environment::EnvironmentVisualState;

const MIN_AMBIENT_BRIGHTNESS: f32 = 6.0;
const MAX_AMBIENT_BRIGHTNESS: f32 = 220.0;
const DAYLIGHT_REFERENCE_ILLUMINANCE: f32 = 40_000.0;
const TERRAIN_NIGHT_BRIGHTNESS: f32 = 0.08;
const TERRAIN_LIGHT_TINT_STRENGTH: f32 = 0.24;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_lighting.run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn update_lighting(
    visuals: Res<EnvironmentVisualState>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    let daylight =
        (visuals.light_illuminance / DAYLIGHT_REFERENCE_ILLUMINANCE).clamp(0.0, 1.0);
    let ambient_daylight = daylight.sqrt();

    ambient_light.color = visuals.light_color;
    ambient_light.brightness = MIN_AMBIENT_BRIGHTNESS
        + (MAX_AMBIENT_BRIGHTNESS - MIN_AMBIENT_BRIGHTNESS) * ambient_daylight;

    let terrain_brightness = TERRAIN_NIGHT_BRIGHTNESS
        + (1.0 - TERRAIN_NIGHT_BRIGHTNESS) * daylight.powf(0.25);
    let light_color = visuals.light_color.to_srgba();
    let neutral = 1.0 - TERRAIN_LIGHT_TINT_STRENGTH;
    let terrain_color = Color::srgb(
        terrain_brightness
            * (neutral + light_color.red * TERRAIN_LIGHT_TINT_STRENGTH),
        terrain_brightness
            * (neutral + light_color.green * TERRAIN_LIGHT_TINT_STRENGTH),
        terrain_brightness
            * (neutral + light_color.blue * TERRAIN_LIGHT_TINT_STRENGTH),
    );

    for (_, material) in terrain_materials.iter_mut() {
        material.base.base_color = terrain_color;
    }
}
