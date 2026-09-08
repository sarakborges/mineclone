use bevy::prelude::*;

use crate::{
    player::camera::GameplayCamera,
    world::render_distance::RenderDistanceSettings,
};

use super::distance::fog_falloff;
use crate::rendering::environment::EnvironmentVisualState;

pub(super) fn attach_fog(
    mut commands: Commands,
    visuals: Res<EnvironmentVisualState>,
    render_distance: Res<RenderDistanceSettings>,
    cameras: Query<Entity, (With<GameplayCamera>, Without<DistanceFog>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert(DistanceFog {
            color: visuals.fog_color,
            falloff: fog_falloff(render_distance.chunks()),
            ..default()
        });
    }
}
