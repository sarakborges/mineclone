use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry, color::Rgb, day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry,
    },
    world::{biome::CurrentBiome, day_night::DayNightClock, dimension::CurrentDimension},
};

#[derive(Resource)]
pub struct EnvironmentVisualState {
    pub sky_color: Color,
    pub fog_color: Color,
    pub sky_light_factor: f32,
}

impl Default for EnvironmentVisualState {
    fn default() -> Self {
        Self {
            sky_color: Color::srgb(0.38, 0.68, 1.0),
            fog_color: Color::srgb(0.52, 0.72, 0.90),
            sky_light_factor: 1.0,
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
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        return;
    };

    let sample = cycle.sample(clock.normalized_time);
    let mut sky = Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    let mut fog = Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    for influence in &current_biome.influences {
        let Some(biome) = biomes.get(&influence.id) else {
            continue;
        };
        let biome_sky = biome.visuals.sky_color.get(sample.phase).lerp(
            *biome.visuals.sky_color.get(sample.next_phase),
            sample.transition,
        );
        let biome_fog = biome.visuals.fog_color.get(sample.phase).lerp(
            *biome.visuals.fog_color.get(sample.next_phase),
            sample.transition,
        );

        sky.r += biome_sky.r * influence.weight;
        sky.g += biome_sky.g * influence.weight;
        sky.b += biome_sky.b * influence.weight;
        fog.r += biome_fog.r * influence.weight;
        fog.g += biome_fog.g * influence.weight;
        fog.b += biome_fog.b * influence.weight;
    }

    visuals.sky_color = sky.to_color();
    visuals.fog_color = fog.to_color();
    visuals.sky_light_factor = sample.sky_light_factor;
}
