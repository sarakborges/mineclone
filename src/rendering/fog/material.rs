use bevy::prelude::*;

use crate::{
    rendering::{environment::EnvironmentVisualState, terrain_material::TerrainMaterial},
    world::render_distance::RenderDistanceSettings,
};

use super::distance::fog_distances;

pub(super) fn sync_terrain_fog(
    visuals: Res<EnvironmentVisualState>,
    render_distance: Res<RenderDistanceSettings>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let linear = visuals.fog_color.to_linear();
    let fog_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
    let (start, end) = fog_distances(render_distance.chunks());
    let fog_distances = Vec4::new(start, end, 0.0, 0.0);

    for (_, material) in materials.iter_mut() {
        material.extension.fog_color = fog_color;
        material.extension.fog_distances = fog_distances;
    }
}
