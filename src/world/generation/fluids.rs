use bevy::prelude::*;

use crate::{
    content::fluid::FluidRegistry,
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
    },
    world::{
        cave_connectivity::CaveConnectivityRegion, generation::GenerationColumnSample,
        generation_region::GenerationRegion, hydrology::HydrologyWaterSample,
    },
};

use super::index::{column_index, voxel_index};

pub(super) struct FluidPassContext<'a> {
    pub(super) fluids: &'a FluidRegistry,
    pub(super) region: &'a GenerationRegion,
    pub(super) anchored_caves: Option<&'a CaveConnectivityRegion>,
    pub(super) underground_water_fluid: &'a str,
}

pub(super) fn rasterize_fluid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    density: &[f32],
    pass: &FluidPassContext<'_>,
) {
    let underground_fluid_id = pass.anchored_caves.map(|_| {
        pass.fluids
            .id_of(pass.underground_water_fluid)
            .unwrap_or_else(|| {
                panic!(
                    "underground hydrology references missing fluid: {}",
                    pass.underground_water_fluid
                )
            })
    });

    chunk.edit_content(|chunk| {
        for local_z in 0..CHUNK_SIZE {
            for local_x in 0..CHUNK_SIZE {
                let world_x = chunk_origin.x + local_x as i32;
                let world_z = chunk_origin.z + local_z as i32;
                let horizontal = Vec2::new(world_x as f32 + 0.5, world_z as f32 + 0.5);
                let surface_height = columns[column_index(local_x, local_z)].surface_height as f32;
                let surface_water = pass
                    .region
                    .hydrology
                    .water_at(horizontal)
                    .filter(|water| surface_water_is_supported(*water, surface_height));
                let surface_fluid_id = surface_water.as_ref().map(|water| {
                    pass.fluids.id_of(water.fluid_id).unwrap_or_else(|| {
                        panic!("hydrology references missing fluid: {}", water.fluid_id)
                    })
                });

                for local_y in 0..CHUNK_SIZE {
                    if density[voxel_index(local_x, local_y, local_z)] > 0.0 {
                        continue;
                    }

                    let world_y = chunk_origin.y + local_y as i32;
                    if let (Some(water), Some(fluid_id)) =
                        (surface_water.as_ref(), surface_fluid_id)
                        && world_y as f32 + 1.0 > water.bed_level
                        && let Some(level) = fluid_level_for_surface(water.water_level, world_y)
                    {
                        chunk.set_fluid(
                            local_x,
                            local_y,
                            local_z,
                            Some(FluidCell::source(fluid_id, level)),
                        );
                        continue;
                    }

                    let (Some(caves), Some(fluid_id)) = (pass.anchored_caves, underground_fluid_id)
                    else {
                        continue;
                    };
                    let position = Vec3::new(
                        world_x as f32 + 0.5,
                        world_y as f32 + 0.5,
                        world_z as f32 + 0.5,
                    );
                    let Some(water) = caves.underground_water_at(position) else {
                        continue;
                    };
                    if world_y as f32 + 1.0 <= water.bed_level {
                        continue;
                    }
                    let Some(level) = fluid_level_for_surface(water.water_level, world_y) else {
                        continue;
                    };

                    chunk.set_fluid(
                        local_x,
                        local_y,
                        local_z,
                        Some(FluidCell::source(fluid_id, level)),
                    );
                }
            }
        }
    });
}

fn surface_water_is_supported(water: HydrologyWaterSample<'_>, surface_height: f32) -> bool {
    // The original surface only decides whether there is a plausible floor.
    // Terrain above the water level is intentionally not a rejection condition:
    // hydrology may have carved that material away to create the lake/river and
    // rejecting the source afterwards leaves dry holes in otherwise valid channels.
    surface_height + 0.5 >= water.bed_level
}

fn fluid_level_for_surface(water_level: f32, world_y: i32) -> Option<u8> {
    let coverage = water_level - world_y as f32;
    if coverage <= 0.0 {
        return None;
    }

    let level = (coverage.clamp(0.0, 1.0) * MAX_FLUID_LEVEL as f32).ceil() as u8;
    Some(level.clamp(1, MAX_FLUID_LEVEL))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::hydrology::HydrologyWaterKind;

    fn sample(
        kind: HydrologyWaterKind,
        water_level: f32,
        bed_level: f32,
    ) -> HydrologyWaterSample<'static> {
        HydrologyWaterSample {
            fluid_id: "asteria:test/water",
            water_level,
            bed_level,
            strength: 1.0,
            kind,
        }
    }

    #[test]
    fn fractional_water_surface_produces_partial_fluid_level() {
        assert_eq!(fluid_level_for_surface(10.25, 10), Some(2));
        assert_eq!(fluid_level_for_surface(10.75, 10), Some(6));
        assert_eq!(fluid_level_for_surface(10.0, 9), Some(MAX_FLUID_LEVEL));
        assert_eq!(fluid_level_for_surface(10.0, 10), None);
    }

    #[test]
    fn surface_water_rejects_columns_without_a_supporting_floor() {
        assert!(!surface_water_is_supported(
            sample(HydrologyWaterKind::Lake, 80.0, 72.0),
            60.0,
        ));
    }

    #[test]
    fn carved_surface_above_water_does_not_remove_valid_sources() {
        assert!(surface_water_is_supported(
            sample(HydrologyWaterKind::Lake, 80.0, 70.0),
            90.0,
        ));
        assert!(surface_water_is_supported(
            sample(HydrologyWaterKind::River, 80.0, 74.0),
            90.0,
        ));
    }
}
