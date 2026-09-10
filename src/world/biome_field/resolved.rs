use bevy::prelude::*;

use super::{BiomeField, BiomeInfluence, ResolvedBiomeFieldSample};

impl BiomeField {
    pub fn sample_resolved(&self, position: Vec3) -> ResolvedBiomeFieldSample<'_> {
        let surface = self.sample_surface(Vec2::new(position.x, position.z));
        let volume = self.sample_volume(position);

        let (primary_id, influences) = if let Some(volume_sample) = &volume {
            let volume_strength = volume_sample.strength.clamp(0.0, 1.0);
            let surface_strength = 1.0 - volume_strength;
            let mut influences = Vec::with_capacity(
                surface.influences.len() + volume_sample.influences.len(),
            );

            influences.extend(surface.influences.iter().filter_map(|influence| {
                let weight = influence.weight * surface_strength;
                (weight > 0.0).then_some(BiomeInfluence {
                    id: influence.id,
                    weight,
                })
            }));
            influences.extend(volume_sample.influences.iter().filter_map(|influence| {
                let weight = influence.weight * volume_strength;
                (weight > 0.0).then_some(BiomeInfluence {
                    id: influence.id,
                    weight,
                })
            }));

            let primary_id = if volume_strength >= 0.5 {
                volume_sample.primary_id
            } else {
                surface.primary_id
            };

            (primary_id, influences)
        } else {
            (surface.primary_id, surface.influences.clone())
        };

        ResolvedBiomeFieldSample {
            primary_id,
            influences,
            surface,
            volume,
        }
    }
}
