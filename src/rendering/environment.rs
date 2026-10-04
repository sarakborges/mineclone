use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::color::Hsi,
    world::current_context::DayNightContext,
};

#[derive(Resource, PartialEq)]
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
    if !scene.day_night.inputs_changed() {
        return;
    }

    let Some(sample) = scene.day_night.sample() else {
        return;
    };

    // Biome-authored environment visuals were part of the deleted generator
    // stack. Keep presentation neutral until the new biome query/presentation
    // contract is introduced in Phase 3.
    let defaults = EnvironmentVisualState::default();
    let next = EnvironmentVisualState {
        sky_color: defaults.sky_color,
        fog_color: defaults.fog_color,
        sky_light_factor: sample.sky_light_factor,
    };

    if *visuals != next {
        *visuals = next;
    }
}
