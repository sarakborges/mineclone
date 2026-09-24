use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry, biome_surface_fluid::BiomeSurfaceFluid,
        biome_terrain::BiomeTerrain, dimension::DimensionDefinition,
        fluid::{FluidId, FluidRegistry},
    },
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
    },
    world::{
        biome_field::BiomeField, cave_connectivity::CaveConnectivityRegion,
        deterministic::{hash_string, mix_seed}, generation::GenerationColumnSample,
        noise::fractal_noise_2d, terrain::surface_height_from_sample,
    },
};

use super::{
    ChunkGenerationContext,
    index::{column_index, voxel_index},
};

pub(super) struct FluidPassContext<'a> {
    pub(super) fluids: &'a FluidRegistry,
    pub(super) biomes: &'a BiomeRegistry,
    pub(super) biome_field: &'a BiomeField,
    pub(super) sea_level: i32,
    pub(super) anchored_caves: Option<&'a CaveConnectivityRegion>,
    pub(super) sea_fluid: &'a str,
}

#[derive(Clone, Copy)]
struct AuthoredSurfaceFluidColumn {
    fluid_id: FluidId,
    surface_height: i32,
    crater_level: Option<f32>,
    spill_level: Option<u8>,
}

impl AuthoredSurfaceFluidColumn {
    fn fluid_at(self, world_y: i32) -> Option<FluidCell> {
        if world_y >= self.surface_height
            && let Some(crater_level) = self.crater_level
            && let Some(level) = fluid_level_for_surface(crater_level, world_y)
        {
            return Some(FluidCell::source(self.fluid_id, level));
        }

        if world_y == self.surface_height
            && let Some(spill_level) = self.spill_level
        {
            return Some(FluidCell::source(self.fluid_id, spill_level));
        }

        None
    }
}

pub(super) fn rasterize_fluid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    density: &[f32],
    pass: &FluidPassContext<'_>,
) {
    let sea_fluid_id = pass
        .fluids
        .id_of(pass.sea_fluid)
        .unwrap_or_else(|| panic!("dimension references missing seaFluid: {}", pass.sea_fluid));
    let underground_fluid_id = pass.anchored_caves.map(|_| sea_fluid_id);
    let mut placements = Vec::<([u8; 3], FluidCell, bool)>::new();

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let horizontal = Vec2::new(world_x as f32 + 0.5, world_z as f32 + 0.5);
            let column = &columns[column_index(local_x, local_z)];
            let surface_height = column.surface_height as f32;
            let sea_surface = (column.ocean_weight > f32::EPSILON
                && surface_height < pass.sea_level as f32)
                .then_some(pass.sea_level as f32);
            let maximum_world_y = chunk_origin.y + CHUNK_SIZE as i32 - 1;
            let authored_surface_fluid = (maximum_world_y >= column.surface_height)
                .then(|| authored_surface_fluid_column(horizontal, column, pass))
                .flatten();

            for local_y in 0..CHUNK_SIZE {
                if density[voxel_index(local_x, local_y, local_z)] > 0.0 {
                    continue;
                }

                let world_y = chunk_origin.y + local_y as i32;
                let (fluid, static_sea) = if let Some(authored) =
                    authored_surface_fluid.and_then(|column| column.fluid_at(world_y))
                {
                    (Some(authored), false)
                } else if let Some(level) =
                    sea_surface.and_then(|surface| fluid_level_for_surface(surface, world_y))
                {
                    (Some(FluidCell::source(sea_fluid_id, level)), true)
                } else if let (Some(caves), Some(fluid_id)) =
                    (pass.anchored_caves, underground_fluid_id)
                {
                    let position = Vec3::new(
                        world_x as f32 + 0.5,
                        world_y as f32 + 0.5,
                        world_z as f32 + 0.5,
                    );
                    (
                        caves.underground_water_at(position).and_then(|water| {
                            (world_y as f32 + 1.0 > water.bed_level)
                                .then(|| fluid_level_for_surface(water.water_level, world_y))
                                .flatten()
                                .map(|level| FluidCell::source(fluid_id, level))
                        }),
                        false,
                    )
                } else {
                    (None, false)
                };

                if let Some(fluid) = fluid {
                    placements.push((
                        [local_x as u8, local_y as u8, local_z as u8],
                        fluid,
                        static_sea,
                    ));
                }
            }
        }
    }

    if placements.is_empty() {
        return;
    }

    let static_sea_positions = placements
        .iter()
        .filter_map(|(position, _, static_sea)| static_sea.then_some(*position))
        .collect::<Vec<_>>();

    chunk.edit_initial_fluids(|chunk| {
        for ([x, y, z], fluid, _) in placements {
            chunk.set_fluid(x as usize, y as usize, z as usize, fluid);
        }
    });
    chunk.suppress_generated_fluid_frontiers(&static_sea_positions);
}

pub(super) fn authored_surface_fluid_id_at<'a>(
    position: IVec2,
    context: &'a ChunkGenerationContext<'_>,
) -> Option<&'a str> {
    authored_surface_fluid_id_for_position(
        position,
        context.dimension,
        context.biomes,
        context.biome_field,
    )
}

pub(crate) fn authored_surface_fluid_id_for_position<'a>(
    position: IVec2,
    dimension: &DimensionDefinition,
    biomes: &'a BiomeRegistry,
    biome_field: &BiomeField,
) -> Option<&'a str> {
    let horizontal = position.as_vec2() + Vec2::splat(0.5);
    let surface = biome_field.sample_surface(horizontal);
    let biome_id = biome_field.surface_biome_id(surface.identity_surface_index);
    let biome = biomes
        .get(biome_id)
        .unwrap_or_else(|| panic!("missing surface biome definition: {biome_id}"));
    let rule = biome.surface_fluid.as_ref()?;
    let surface_height =
        surface_height_from_sample(position, dimension, biome_field, &surface);
    let primary_terrain_strength = surface
        .influences
        .iter()
        .find(|influence| influence.surface_index == surface.primary_surface_index)
        .map_or(1.0, |influence| influence.terrain_strength);

    match rule {
        BiomeSurfaceFluid::VolcanoCrater {
            fluid,
            minimum_strength,
            level_offset,
            spill_minimum_strength,
            spill_maximum_strength,
            spill_scale,
            spill_width,
            ..
        } => {
            let BiomeTerrain::Volcano {
                base_height,
                height,
                crater_depth,
                ..
            } = biome
                .terrain
                .expect("validated volcano crater surface fluid requires volcano terrain")
            else {
                unreachable!("validated volcano crater surface fluid requires volcano terrain");
            };
            let crater_level = dimension.sea_level as f32
                + base_height
                + height
                - crater_depth
                + level_offset;
            let crater_present =
                primary_terrain_strength >= *minimum_strength
                    && crater_level > surface_height as f32;
            let spill_present =
                primary_terrain_strength >= *spill_minimum_strength
                    && primary_terrain_strength <= *spill_maximum_strength
                    && volcano_spill_channel(
                        horizontal,
                        *spill_scale,
                        *spill_width,
                        biome_field.seed(),
                        biome_id,
                    );

            (crater_present || spill_present).then_some(fluid.as_str())
        }
    }
}

fn authored_surface_fluid_column(
    horizontal: Vec2,
    column: &GenerationColumnSample,
    pass: &FluidPassContext<'_>,
) -> Option<AuthoredSurfaceFluidColumn> {
    let biome_id = pass
        .biome_field
        .surface_biome_id(column.identity_surface_index);
    let biome = pass
        .biomes
        .get(biome_id)
        .unwrap_or_else(|| panic!("missing surface biome definition: {biome_id}"));
    let rule = biome.surface_fluid.as_ref()?;

    match rule {
        BiomeSurfaceFluid::VolcanoCrater {
            fluid,
            minimum_strength,
            level_offset,
            spill_minimum_strength,
            spill_maximum_strength,
            spill_scale,
            spill_width,
            spill_level,
        } => {
            let BiomeTerrain::Volcano {
                base_height,
                height,
                crater_depth,
                ..
            } = biome
                .terrain
                .expect("validated volcano crater surface fluid requires volcano terrain")
            else {
                unreachable!("validated volcano crater surface fluid requires volcano terrain");
            };

            let fluid_id = resolve_authored_fluid_id(pass.fluids, fluid, biome_id);
            let strength = column.primary_terrain_strength.clamp(0.0, 1.0);
            let crater_level =
                pass.sea_level as f32 + base_height + height - crater_depth + level_offset;
            let crater_level = (strength >= *minimum_strength
                && crater_level > column.surface_height as f32)
                .then_some(crater_level);
            let spill_level = (strength >= *spill_minimum_strength
                && strength <= *spill_maximum_strength
                && volcano_spill_channel(
                    horizontal,
                    *spill_scale,
                    *spill_width,
                    pass.biome_field.seed(),
                    biome_id,
                ))
            .then_some(*spill_level);

            (crater_level.is_some() || spill_level.is_some()).then_some(
                AuthoredSurfaceFluidColumn {
                    fluid_id,
                    surface_height: column.surface_height,
                    crater_level,
                    spill_level,
                },
            )
        }
    }
}

fn resolve_authored_fluid_id(
    fluids: &FluidRegistry,
    fluid: &str,
    biome_id: &str,
) -> FluidId {
    fluids.id_of(fluid).unwrap_or_else(|| {
        panic!("biome {biome_id} surfaceFluid references missing fluid {fluid}")
    })
}

fn volcano_spill_channel(
    horizontal: Vec2,
    scale: f32,
    width: f32,
    world_seed: u64,
    biome_id: &str,
) -> bool {
    let seed = mix_seed(world_seed ^ hash_string(biome_id) ^ 0x6c61_7661_7370_696c);
    fractal_noise_2d(horizontal * scale, seed, 3).abs() <= width
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

    #[test]
    fn fractional_water_surface_produces_partial_fluid_level() {
        assert_eq!(fluid_level_for_surface(10.25, 10), Some(2));
        assert_eq!(fluid_level_for_surface(10.75, 10), Some(6));
        assert_eq!(fluid_level_for_surface(10.0, 9), Some(MAX_FLUID_LEVEL));
        assert_eq!(fluid_level_for_surface(10.0, 10), None);
    }
}
