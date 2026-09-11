mod hydrology;
mod noise;
mod volume;

use bevy::prelude::*;

use super::{
    biome_field::{BiomeField, VolumeBiomeSelection},
    cave_connectivity::CaveConnectivityRegion,
    generation_region::GenerationRegion,
    math::smoothstep,
};
use self::{
    hydrology::{cave_water_clearance, enforce_hydrology_water_volume},
    volume::volume_biome_density_delta,
};

pub(crate) use hydrology::{DensityColumnHydrology, sample_density_column_hydrology};

const CAVE_CONNECTOR_AIR_MARGIN: f32 = 5.5;
const CAVE_CONNECTOR_MINIMUM_SURFACE_DEPTH: f32 = -2.0;
const CAVE_CONNECTOR_FULL_STRENGTH_SURFACE_DEPTH: f32 = 1.5;
const CAVERN_MINIMUM_SURFACE_DEPTH: f32 = 12.0;
const CAVERN_FULL_STRENGTH_SURFACE_DEPTH: f32 = 20.0;

pub(crate) struct DensitySampleContext<'a> {
    region: &'a GenerationRegion,
    anchored_caves: Option<&'a CaveConnectivityRegion>,
    biome_field: &'a BiomeField,
}

impl<'a> DensitySampleContext<'a> {
    pub(crate) fn new(
        region: &'a GenerationRegion,
        anchored_caves: Option<&'a CaveConnectivityRegion>,
        biome_field: &'a BiomeField,
    ) -> Self {
        Self {
            region,
            anchored_caves,
            biome_field,
        }
    }
}

pub(crate) fn sample_density(
    base_density: f32,
    position: Vec3,
    volume: Option<VolumeBiomeSelection>,
    context: &DensitySampleContext<'_>,
) -> f32 {
    let column_hydrology = sample_density_column_hydrology(
        Vec2::new(position.x, position.z),
        context.region,
    );
    let hydrology_delta = context.region.hydrology.density_delta(position);

    sample_density_with_hydrology(
        base_density,
        position,
        volume,
        column_hydrology,
        hydrology_delta,
        context,
    )
}

pub(crate) fn sample_density_with_hydrology(
    base_density: f32,
    position: Vec3,
    volume: Option<VolumeBiomeSelection>,
    column_hydrology: DensityColumnHydrology,
    hydrology_delta: f32,
    context: &DensitySampleContext<'_>,
) -> f32 {
    let mut density = base_density + hydrology_delta;
    let water_clearance = cave_water_clearance(position.y, column_hydrology);
    let connector_depth_strength = depth_strength(
        base_density,
        CAVE_CONNECTOR_MINIMUM_SURFACE_DEPTH,
        CAVE_CONNECTOR_FULL_STRENGTH_SURFACE_DEPTH,
    ) * water_clearance;

    if connector_depth_strength > 0.0
        && let Some(connector) = context
            .anchored_caves
            .and_then(|caves| caves.connector_graph.sample(position))
            .map(|sample| smoothstep(sample.strength) * connector_depth_strength)
    {
        density += carve_density_delta(density, connector, CAVE_CONNECTOR_AIR_MARGIN);
    }

    let cavern_depth_strength = depth_strength(
        base_density,
        CAVERN_MINIMUM_SURFACE_DEPTH,
        CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
    ) * water_clearance;

    density += volume_biome_density_delta(
        density,
        position,
        volume,
        context.biome_field,
        cavern_depth_strength,
    );

    enforce_hydrology_water_volume(
        density,
        base_density,
        position,
        column_hydrology,
        context.biome_field.seed(),
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

fn carve_density_delta(density: f32, strength: f32, air_margin: f32) -> f32 {
    if strength <= 0.0 {
        return 0.0;
    }

    -(density.max(0.0) + air_margin) * strength.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_explicit_cave_strength_opens_deep_solid_density() {
        let density = 80.0;
        let carved = density + carve_density_delta(density, 1.0, 4.0);

        assert!(carved < 0.0);
    }

    #[test]
    fn cavern_carving_stays_suppressed_close_to_the_surface() {
        assert_eq!(
            depth_strength(
                0.0,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
            0.0
        );
        assert_eq!(
            depth_strength(
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ),
            0.0
        );
        assert!(
            depth_strength(
                16.0,
                CAVERN_MINIMUM_SURFACE_DEPTH,
                CAVERN_FULL_STRENGTH_SURFACE_DEPTH,
            ) > 0.0
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

    #[test]
    fn cave_connectors_can_open_a_surface_mouth() {
        let strength = depth_strength(
            0.5,
            CAVE_CONNECTOR_MINIMUM_SURFACE_DEPTH,
            CAVE_CONNECTOR_FULL_STRENGTH_SURFACE_DEPTH,
        );
        let density = 0.5;
        let carved = density + carve_density_delta(density, strength, CAVE_CONNECTOR_AIR_MARGIN);

        assert!(strength > 0.0);
        assert!(carved < 0.0);
    }
}
