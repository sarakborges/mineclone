use bevy::prelude::*;

use crate::app::game_state::GameState;

use super::{environment::EnvironmentVisualState, terrain_material::TerrainMaterial};

#[derive(Resource, Default)]
struct AppliedSkyLightFactor(Option<f32>);

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppliedSkyLightFactor>()
            .add_systems(OnEnter(GameState::Gameplay), reset_applied_sky_light_factor)
            .add_systems(
                PostUpdate,
                sync_sky_light_factor.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn reset_applied_sky_light_factor(mut applied: ResMut<AppliedSkyLightFactor>) {
    applied.0 = None;
}

fn sync_sky_light_factor(
    visuals: Res<EnvironmentVisualState>,
    mut applied: ResMut<AppliedSkyLightFactor>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let sky_light_factor = visuals.sky_light_factor.clamp(0.0, 1.0);
    if applied.0 == Some(sky_light_factor) {
        return;
    }

    for (_, material) in materials.iter_mut() {
        material.extension.sky_light_factor = sky_light_factor;
    }
    applied.0 = Some(sky_light_factor);
}
