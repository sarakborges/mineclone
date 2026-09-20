use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::app::game_state::GameState;

use super::{
    environment::EnvironmentVisualState,
    terrain_material::TerrainLightingBuffer,
};

#[derive(Resource, Default)]
struct AppliedSkyLightFactor(Option<f32>);

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppliedSkyLightFactor>()
            .add_systems(OnEnter(GameState::Gameplay), reset_applied_sky_light_factor)
            .add_systems(
                PostUpdate,
                sync_sky_light_factor
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(sky_light_factor_needs_sync),
            );
    }
}

fn reset_applied_sky_light_factor(mut applied: ResMut<AppliedSkyLightFactor>) {
    applied.0 = None;
}

fn sky_light_factor_needs_sync(
    visuals: Res<EnvironmentVisualState>,
    applied: Res<AppliedSkyLightFactor>,
) -> bool {
    applied.0 != Some(visuals.sky_light_factor.clamp(0.0, 1.0))
}

fn sync_sky_light_factor(
    visuals: Res<EnvironmentVisualState>,
    mut applied: ResMut<AppliedSkyLightFactor>,
    terrain_lighting: Res<TerrainLightingBuffer>,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let sky_light_factor = visuals.sky_light_factor.clamp(0.0, 1.0);
    if applied.0 == Some(sky_light_factor) {
        return;
    }

    terrain_lighting.set_sky_light_factor(&mut shader_buffers, sky_light_factor);
    applied.0 = Some(sky_light_factor);
}
