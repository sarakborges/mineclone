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
}

impl Default for EnvironmentVisualState {
    fn default() -> Self {
        Self {
            sky_color: Color::srgb(0.38, 0.68, 1.0),
            fog_color: Color::srgb(0.52, 0.72, 0.90),
            ambient_color: Color::WHITE,
            ambient_brightness: 90.0,
            sun_color: Color::WHITE,
            sun_illuminance: 80_000.0,
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
    let Some(primary_biome) = biomes.get(&current_biome.id) else {
        return;
    };
    let secondary_biome = biomes
        .get(&current_biome.secondary_id)
        .unwrap_or(primary_biome);
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        return;
    };

    let sample = cycle.sample(clock.normalized_time);

    let primary_sky = primary_biome
        .visuals
        .sky_color
        .get(sample.phase)
        .lerp(*primary_biome.visuals.sky_color.get(sample.next_phase), sample.transition);
    let secondary_sky = secondary_biome
        .visuals
        .sky_color
        .get(sample.phase)
        .lerp(*secondary_biome.visuals.sky_color.get(sample.next_phase), sample.transition);
    let primary_fog = primary_biome
        .visuals
        .fog_color
        .get(sample.phase)
        .lerp(*primary_biome.visuals.fog_color.get(sample.next_phase), sample.transition);
    let secondary_fog = secondary_biome
        .visuals
        .fog_color
        .get(sample.phase)
        .lerp(*secondary_biome.visuals.fog_color.get(sample.next_phase), sample.transition);

    visuals.sky_color = primary_sky
        .lerp(secondary_sky, current_biome.secondary_weight)
        .to_color();
    visuals.fog_color = primary_fog
        .lerp(secondary_fog, current_biome.secondary_weight)
        .to_color();
    visuals.ambient_color = sample.ambient_color.to_color();
    visuals.ambient_brightness = sample.ambient_brightness;
    visuals.sun_color = sample.sun_color.to_color();
    visuals.sun_illuminance = sample.sun_illuminance;
}
