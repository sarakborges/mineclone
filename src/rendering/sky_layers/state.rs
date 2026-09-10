use bevy::prelude::*;

use crate::{
    content::{biome::BiomeRegistry, color::Rgb},
    world::biome::CurrentBiome,
};

#[derive(Resource)]
pub(super) struct SkyLayerVisualState {
    pub star_density: f32,
    pub star_color: Rgb,
    pub cloud_density: f32,
    pub cloud_color: Rgb,
}

impl Default for SkyLayerVisualState {
    fn default() -> Self {
        Self {
            star_density: 0.0,
            star_color: Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            cloud_density: 0.0,
            cloud_color: Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
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
    let mut star_color = Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    let mut cloud_color = Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    for influence in &current_biome.influences {
        let Some(biome) = biomes.get(&influence.id) else {
            continue;
        };

        star_density += biome.visuals.stars.density * influence.weight;
        cloud_density += biome.visuals.clouds.density * influence.weight;
        star_color.r += biome.visuals.stars.color.r * influence.weight;
        star_color.g += biome.visuals.stars.color.g * influence.weight;
        star_color.b += biome.visuals.stars.color.b * influence.weight;
        cloud_color.r += biome.visuals.clouds.color.r * influence.weight;
        cloud_color.g += biome.visuals.clouds.color.g * influence.weight;
        cloud_color.b += biome.visuals.clouds.color.b * influence.weight;
    }

    visuals.star_density = star_density.clamp(0.0, 1.0);
    visuals.cloud_density = cloud_density.clamp(0.0, 1.0);
    visuals.star_color = star_color;
    visuals.cloud_color = cloud_color;
}
