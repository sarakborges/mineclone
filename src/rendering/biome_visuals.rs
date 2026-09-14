use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{biome::{BiomeDefinition, BiomeRegistry}, color::Hsi},
    world::biome::CurrentBiome,
};

#[derive(SystemParam)]
pub(crate) struct CurrentBiomeVisuals<'w> {
    current: Res<'w, CurrentBiome>,
    biomes: Res<'w, BiomeRegistry>,
}

impl CurrentBiomeVisuals<'_> {
    pub(crate) fn weighted_scalar(
        &self,
        value: impl Fn(&BiomeDefinition) -> f32,
    ) -> f32 {
        self.current
            .influences
            .iter()
            .filter_map(|influence| {
                self.biomes
                    .get(&influence.id)
                    .map(|biome| value(biome) * influence.weight)
            })
            .sum()
    }

    pub(crate) fn blend_hsi(
        &self,
        value: impl Fn(&BiomeDefinition) -> Hsi,
    ) -> Hsi {
        Hsi::blend_weighted(self.current.influences.iter().filter_map(|influence| {
            self.biomes
                .get(&influence.id)
                .map(|biome| (value(biome), influence.weight))
        }))
    }
}
