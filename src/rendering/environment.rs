use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::color::Hsi,
    world::current_context::DayNightContext,
};

use super::biome_visuals::CurrentBiomeVisuals;

// The background is a flat sky color, not a sky gradient. Its terminal color
// must equal the fog color to avoid exposing unrendered chunks as silhouettes.
// Incorporate the biome's authored fog palette without losing the sky palette.
const HORIZON_FOG_COLOR_WEIGHT: f32 = 0.35;

#[derive(Resource, PartialEq)]
pub struct EnvironmentVisualState {
    pub sky_color: Hsi,
    pub fog_color: Hsi,
    pub sky_light_factor: f32,
}

impl EnvironmentVisualState {
    pub(crate) fn horizon_color(&self) -> Color {
        self.sky_color
            .lerp(self.fog_color, HORIZON_FOG_COLOR_WEIGHT)
            .to_color()
    }
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
    biome_visuals: CurrentBiomeVisuals<'w>,
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
    if !scene.day_night.inputs_changed() && !scene.biome_visuals.inputs_changed() {
        return;
    }

    let Some(sample) = scene.day_night.sample() else {
        return;
    };

    let next = EnvironmentVisualState {
        sky_color: scene.biome_visuals.blend_hsi(|biome| {
            biome.visuals.sky_color.get(sample.phase).lerp(
                *biome.visuals.sky_color.get(sample.next_phase),
                sample.transition,
            )
        }),
        fog_color: scene.biome_visuals.blend_hsi(|biome| {
            biome.visuals.fog_color.get(sample.phase).lerp(
                *biome.visuals.fog_color.get(sample.next_phase),
                sample.transition,
            )
        }),
        sky_light_factor: sample.sky_light_factor,
    };

    if *visuals != next {
        *visuals = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_horizon_uses_both_biome_palettes() {
        let visuals = EnvironmentVisualState {
            sky_color: Hsi::new(210.0, 1.0, 0.5),
            fog_color: Hsi::new(90.0, 1.0, 0.5),
            sky_light_factor: 1.0,
        };
        let horizon = visuals.horizon_color();

        assert_eq!(
            horizon,
            visuals
                .sky_color
                .lerp(visuals.fog_color, HORIZON_FOG_COLOR_WEIGHT)
                .to_color(),
        );
        assert_ne!(horizon, visuals.sky_color.to_color());
        assert_ne!(horizon, visuals.fog_color.to_color());
    }
}
