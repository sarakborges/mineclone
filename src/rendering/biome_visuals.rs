use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        biome::{BiomeDefinition, BiomeRegistry},
        color::Hsi,
    },
    world::biome::CurrentBiome,
};

#[derive(SystemParam)]
pub(crate) struct CurrentBiomeVisuals<'w> {
    current: Res<'w, CurrentBiome>,
    biomes: Res<'w, BiomeRegistry>,
}

impl CurrentBiomeVisuals<'_> {
    pub(crate) fn inputs_changed(&self) -> bool {
        self.current.is_changed() || self.biomes.is_changed()
    }

    pub(crate) fn weighted_scalar(
        &self,
        value: impl Fn(&BiomeDefinition) -> f32,
    ) -> f32 {
        let surface_strength = (1.0 - self.current.volume_strength).clamp(0.0, 1.0);
        let volume_strength = self.current.volume_strength.clamp(0.0, 1.0);
        let value = &value;

        self.current
            .surface_influences
            .iter()
            .filter_map(|influence| {
                self.biomes
                    .get(&influence.id)
                    .map(|biome| value(biome) * influence.weight * surface_strength)
            })
            .chain(self.current.volume_influences.iter().filter_map(|influence| {
                self.biomes
                    .get(&influence.id)
                    .map(|biome| value(biome) * influence.weight * volume_strength)
            }))
            .sum()
    }

    pub(crate) fn blend_hsi(
        &self,
        value: impl Fn(&BiomeDefinition) -> Hsi,
    ) -> Hsi {
        let surface_strength = (1.0 - self.current.volume_strength).clamp(0.0, 1.0);
        let volume_strength = self.current.volume_strength.clamp(0.0, 1.0);
        let value = &value;

        Hsi::blend_weighted(
            self.current
                .surface_influences
                .iter()
                .filter_map(|influence| {
                    self.biomes.get(&influence.id).map(|biome| {
                        (value(biome), influence.weight * surface_strength)
                    })
                })
                .chain(self.current.volume_influences.iter().filter_map(|influence| {
                    self.biomes.get(&influence.id).map(|biome| {
                        (value(biome), influence.weight * volume_strength)
                    })
                })),
        )
    }
}
