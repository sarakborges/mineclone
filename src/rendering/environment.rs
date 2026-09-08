use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry,
        day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry,
    },
    world::{
        biome::CurrentBiome,
        day_night::DayNightClock,
        dimension::CurrentDimension,
    },
};

#[derive(Resource)]
pub struct EnvironmentVisualState {
    pub sky_color: Color,
    pub fog_color: Color,
    pub ambient_color: Color,
    pub ambient_brightness: f32,
    pub sun_color: Color,
    pub sun_illuminance: f32,
    pub sun_rotation: Quat,
}

impl Default for EnvironmentVisualState {
    fn default() -> Self {
        Self {
            sky_color: Color::srgb(0.38, 0.68, 1.0),
            fog_color: Color::srgb(0.52, 0.72, 0.90),
            ambient_color: Color::WHITE,
            ambient_brightness: 400.0,
            sun_color: Color::WHITE,
            sun_illuminance: 80_000.0,
            sun_rotation: Quat::IDENTITY,
        }
    }
}

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentVisualState>().add_systems(
            Update,
            update_environment_visuals.run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn update_environment_visuals(
    current_dimension: Res<CurrentDimension>,
    current_biome: Res<CurrentBiome>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    clock: Res<DayNightClock>,
    mut visuals: ResMut<EnvironmentVisualState>,
) {
    let Some(dimension) = dimensions.get(&current_dimension.id) else {
        return;
    };
    let Some(biome) = biomes.get(&current_biome.id) else {
        return;
    };
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        return;
    };
    let Some(sample) = cycle.sample(clock.normalized_time) else {
        return;
    };

    visuals.sky_color = biome
        .visuals
        .sky_color
        .multiply(sample.sky_tint)
        .to_color();
    visuals.fog_color = biome
        .visuals
        .fog_color
        .multiply(sample.fog_tint)
        .to_color();
    visuals.ambient_color = sample.ambient_color.to_color();
    visuals.ambient_brightness = sample.ambient_brightness;
    visuals.sun_color = sample.sun_color.to_color();
    visuals.sun_illuminance = sample.sun_illuminance;

    let sun_angle = clock.normalized_time * std::f32::consts::TAU
        + cycle.sun_angle_offset_degrees.to_radians();
    visuals.sun_rotation = Quat::from_euler(EulerRot::XYZ, sun_angle, -0.45, 0.0);
}
