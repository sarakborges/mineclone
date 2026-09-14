use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, color::Hsi},
    world::biome::CurrentBiome,
};

#[derive(Resource)]
pub(super) struct SkyLayerVisualState {
    pub star_density: f32,
    pub star_color: Hsi,
    pub cloud_density: f32,
    pub cloud_color: Hsi,
}

impl Default for SkyLayerVisualState {
    fn default() -> Self {
        Self {
            star_density: 0.0,
            star_color: Hsi::WHITE,
            cloud_density: 0.0,
            cloud_color: Hsi::WHITE,
        }
    }
}

pub(super) fn update_sky_layer_visuals(
    current_biome: Res<CurrentBiome>,
    biomes: Res<BiomeRegistry>,
    mut visuals: ResMut<SkyLayerVisualState>,
) {
    let mut star_density = 0.0;
    let mut cloud_density = 0.0;

    for influence in &current_biome.influences {
        let Some(biome) = biomes.get(&influence.id) else {
            continue;
        };
        star_density += biome.visuals.stars.density * influence.weight;
        cloud_density += biome.visuals.clouds.density * influence.weight;
    }

    visuals.star_density = star_density.clamp(0.0, 1.0);
    visuals.cloud_density = cloud_density.clamp(0.0, 1.0);
    visuals.star_color = Hsi::blend_weighted(current_biome.influences.iter().filter_map(|influence| {
        biomes
            .get(&influence.id)
            .map(|biome| (biome.visuals.stars.color, influence.weight))
    }));
    visuals.cloud_color = Hsi::blend_weighted(current_biome.influences.iter().filter_map(|influence| {
        biomes
            .get(&influence.id)
            .map(|biome| (biome.visuals.clouds.color, influence.weight))
    }));
}
