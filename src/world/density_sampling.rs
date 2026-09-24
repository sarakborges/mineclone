mod volume;

use bevy::prelude::*;

use super::{
    biome_field::{BiomeField, VolumeBiomeSelection},
    math::smoothstep,
};
use self::volume::volume_biome_density_delta;

const CAVERN_MINIMUM_SURFACE_DEPTH: f32 = 12.0;
const CAVERN_FULL_STRENGTH_SURFACE_DEPTH: f32 = 20.0;

pub(crate) struct DensitySampleContext<'a> {
    biome_field: &'a BiomeField,
    allow_caverns: bool,
    allow_solid_volume: bool,
}

impl<'a> DensitySampleContext<'a> {
    pub(crate) fn new(biome_field: &'a BiomeField) -> Self {
        Self {
            biome_field,
            allow_caverns: true,
            allow_solid_volume: true,
        }
    }

    pub(crate) fn with_volume_rules(
        mut self,
        allow_caverns: bool,
        allow_solid_volume: bool,
    ) -> Self {
        self.allow_caverns = allow_caverns;
        self.allow_solid_volume = allow_solid_volume;
        self
    }
}

pub(crate) fn sample_density(
    base_density: f32,
    position: Vec3,
    volume: Option<VolumeBiomeSelection>,
    context: &DensitySampleContext<'_>,
) -> f32 {
    let cavern_depth_strength = depth_strength(
        base_density,
        CAVERN_MINIMUM_SURFACE_DEPTH,
        CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
    );

    base_density + volume_biome_density_delta(
        base_density,
        position,
        volume,
        context.biome_field,
        cavern_depth_strength,
        context.allow_caverns,
        context.allow_solid_volume,
    )
}

fn depth_strength(base_density: f32, minimum_depth: f32, full_strength_depth: f32) -> f32 {
    if base_density <= minimum_depth {
        return 0.0;
    }
    if base_density >= full_strength_depth {
        return 1.0;
    }

    let progress = (base_density - minimum_depth) / (full_strength_depth - minimum_depth);
    smoothstep(progress.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cavern_carving_stays_suppressed_close_to_the_surface() {
        assert_eq!(
            depth_strength(0.0, CAVERN_MINIMUM_SURFACE_DEPTH, CAVERN_FULL_STRENGTH_SURFACE_DEPTH),
            0.0
        );
        assert!(
            depth_strength(16.0, CAVERN_MINIMUM_SURFACE_DEPTH, CAVERN_FULL_STRENGTH_SURFACE_DEPTH)
                > 0.0
        );
        assert_eq!(
            depth_strength(
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
            1.0
        );
    }
}
