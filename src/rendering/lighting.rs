use bevy::prelude::*;

use crate::app::game_state::GameState;

use super::{environment::EnvironmentVisualState, terrain_material::TerrainMaterial};

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            sync_sky_light_factor.run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn sync_sky_light_factor(
    visuals: Res<EnvironmentVisualState>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let sky_light_factor = visuals.sky_light_factor.clamp(0.0, 1.0);

    for (_, material) in materials.iter_mut() {
        material.extension.sky_light_factor = sky_light_factor;
    }
}
