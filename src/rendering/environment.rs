use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, color::Hsi},
    world::{biome::CurrentBiome, current_context::DayNightContext},
};

#[derive(Resource)]
pub struct EnvironmentVisualState {
    pub sky_color: Hsi,
    pub fog_color: Hsi,
    pub sky_light_factor: f32,
}

impl Default for EnvironmentVisualState {
    fn default() -> Self {
        Self {
            sky_color: Hsi::from_srgb([0.38, 0.68, 1.0]),
            fog_color: Hsi::from_srgb([0.52, 0.72, 0.90]),
            sky_light_factor: 1.0,
        }
    }
}

#[derive(SystemParam)]
struct EnvironmentScene<'w> {
    day_night: DayNightContext<'w>,
    current_biome: Res<'w, CurrentBiome>,
    biomes: Res<'w, BiomeRegistry>,
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
    scene: EnvironmentScene,
    mut visuals: ResMut<EnvironmentVisualState>,
) {
    let Some(sample) = scene.day_night.sample() else {
        return;
    };

    visuals.sky_color = Hsi::blend_weighted(scene.current_biome.influences.iter().filter_map(
        |influence| {
            let biome = scene.biomes.get(&influence.id)?;
            let color = biome.visuals.sky_color.get(sample.phase).lerp(
                *biome.visuals.sky_color.get(sample.next_phase),
                sample.transition,
            );
            Some((color, influence.weight))
        },
    ));
    visuals.fog_color = Hsi::blend_weighted(scene.current_biome.influences.iter().filter_map(
        |influence| {
            let biome = scene.biomes.get(&influence.id)?;
            let color = biome.visuals.fog_color.get(sample.phase).lerp(
                *biome.visuals.fog_color.get(sample.next_phase),
                sample.transition,
            );
            Some((color, influence.weight))
        },
    ));
    visuals.sky_light_factor = sample.sky_light_factor;
}
