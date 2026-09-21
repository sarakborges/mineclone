use bevy::prelude::*;

use crate::{
    content::{
        structure::StructureDefinition,
        structure_rules::{
            StructureFluidPolicy, StructureProximityMode, StructureProximityRestriction,
            StructureProximityTarget,
        },
    },
    voxel::coordinates::chunk_coord_from_world,
    world::{
        generation_region::generation_region_coord,
        hydrology::HydrologyWaterKind,
        terrain::surface_height_from_sample,
    },
};

use super::super::{ChunkGenerationContext, fluids::authored_surface_fluid_id_at};

pub(super) fn candidate_satisfies_restrictions(
    biome_id: &str,
    structure: &StructureDefinition,
    anchor: IVec2,
    origin_y: i32,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    let restrictions = &structure.restrictions;
    let base_y = origin_y + structure.min_y_offset();

    if restrictions.min_y.is_some_and(|minimum| base_y < minimum)
        || restrictions.max_y.is_some_and(|maximum| base_y > maximum)
    {
        return false;
    }

    if !restrictions.ground_blocks.is_empty()
        && structure.support_offsets().iter().any(|offset| {
            !ground_block_is_allowed(anchor + *offset, &restrictions.ground_blocks, context)
        })
    {
        return false;
    }

    if restrictions.required_biome_coverage > 0.0 {
        let footprint = structure.horizontal_footprint();
        let matching = footprint
            .iter()
            .filter(|offset| {
                context
                    .biome_field
                    .sample_surface((anchor + **offset).as_vec2() + Vec2::splat(0.5))
                    .primary_id
                    == biome_id
            })
            .count();
        let coverage = matching as f32 / footprint.len().max(1) as f32;
        if coverage + f32::EPSILON < restrictions.required_biome_coverage {
            return false;
        }
    }

    if structure.generation.fluid_policy == StructureFluidPolicy::Forbid
        && intersects_surface_fluid(structure, anchor, origin_y, context)
    {
        return false;
    }

    if restrictions
        .proximity
        .iter()
        .any(|rule| !proximity_rule_satisfied(anchor, rule, context))
    {
        return false;
    }

    true
}

fn ground_block_is_allowed(
    position: IVec2,
    allowed: &[String],
    context: &ChunkGenerationContext<'_>,
) -> bool {
    surface_block_matches(position, context, |block| {
        allowed.iter().any(|candidate| candidate == block)
    })
}

fn surface_block_matches(
    position: IVec2,
    context: &ChunkGenerationContext<'_>,
    matches: impl Fn(&str) -> bool,
) -> bool {
    let horizontal = position.as_vec2() + Vec2::splat(0.5);
    let surface = context.biome_field.sample_surface(horizontal);
    let surface_height =
        surface_height_from_sample(position, context.dimension, context.biome_field, &surface);
    let mut chunk_coord =
        chunk_coord_from_world(IVec3::new(position.x, (surface_height - 1).max(0), position.y));
    chunk_coord.y = chunk_coord.y.max(0);
    let region = context.region(generation_region_coord(chunk_coord));

    if let Some(water) = region
        .hydrology
        .supported_water_at(horizontal, surface_height as f32)
    {
        let surface_biome_id = context
            .biome_field
            .surface_biome_id(surface.identity_surface_index);
        let surface_biome = context
            .biomes
            .get(surface_biome_id)
            .unwrap_or_else(|| panic!("missing surface biome definition: {surface_biome_id}"));
        let hydrology_block = match water.kind {
            HydrologyWaterKind::River => surface_biome
                .hydrology
                .river_bed_block
                .as_deref()
                .or(surface_biome.hydrology.shore_block.as_deref()),
            HydrologyWaterKind::Lake => surface_biome
                .hydrology
                .lake_bed_block
                .as_deref()
                .or(surface_biome.hydrology.shore_block.as_deref()),
            HydrologyWaterKind::Ocean => context
                .dimension
                .hydrology
                .ocean_biome
                .as_deref()
                .and_then(|id| context.biomes.get(id))
                .and_then(|biome| {
                    biome
                        .hydrology
                        .ocean_bed_block
                        .as_deref()
                        .or(biome.hydrology.shore_block.as_deref())
                }),
        };
        if hydrology_block.is_some_and(&matches) {
            return true;
        }
    }

    if let Some(margin_index) = surface.surface_margin_index {
        let biome_id = context.biome_field.surface_biome_id(margin_index);
        let biome = context
            .biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing surface margin biome definition: {biome_id}"));
        if let Some(block) = biome
            .surface_margin
            .as_ref()
            .and_then(|margin| margin.block_at_depth(0))
        {
            return matches(block);
        }
    }

    surface
        .influences
        .iter()
        .filter_map(|influence| {
            let biome_id = context
                .biome_field
                .surface_biome_id(influence.surface_index);
            let biome = context
                .biomes
                .get(biome_id)
                .unwrap_or_else(|| panic!("missing surface biome definition: {biome_id}"));
            biome
                .surface_block_at_depth(0)
                .map(|block| (block, influence.weight))
        })
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .is_some_and(|(block, _)| matches(block))
}

fn proximity_rule_satisfied(
    anchor: IVec2,
    rule: &StructureProximityRestriction,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    let minimum = rule.min_distance.unwrap_or(0);
    match rule.mode {
        StructureProximityMode::Required => {
            nearest_target_distance_squared(anchor, rule.max_distance, &rule.target, context)
                .is_some_and(|distance_squared| {
                    distance_squared >= i64::from(minimum) * i64::from(minimum)
                })
        }
        StructureProximityMode::Forbidden => !target_exists_in_annulus(
            anchor,
            minimum,
            rule.max_distance,
            &rule.target,
            context,
        ),
    }
}

fn nearest_target_distance_squared(
    anchor: IVec2,
    maximum_distance: u32,
    target: &StructureProximityTarget,
    context: &ChunkGenerationContext<'_>,
) -> Option<i64> {
    let radius = maximum_distance as i32;
    let maximum_squared = i64::from(maximum_distance) * i64::from(maximum_distance);
    let mut nearest = None;

    for z in -radius..=radius {
        for x in -radius..=radius {
            let distance_squared = i64::from(x) * i64::from(x) + i64::from(z) * i64::from(z);
            if distance_squared > maximum_squared
                || nearest.is_some_and(|current| distance_squared >= current)
            {
                continue;
            }
            if proximity_target_matches(anchor + IVec2::new(x, z), target, context) {
                nearest = Some(distance_squared);
            }
        }
    }

    nearest
}

fn target_exists_in_annulus(
    anchor: IVec2,
    minimum_distance: u32,
    maximum_distance: u32,
    target: &StructureProximityTarget,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    let radius = maximum_distance as i32;
    let minimum_squared = i64::from(minimum_distance) * i64::from(minimum_distance);
    let maximum_squared = i64::from(maximum_distance) * i64::from(maximum_distance);

    for z in -radius..=radius {
        for x in -radius..=radius {
            let distance_squared = i64::from(x) * i64::from(x) + i64::from(z) * i64::from(z);
            if distance_squared < minimum_squared || distance_squared > maximum_squared {
                continue;
            }
            if proximity_target_matches(anchor + IVec2::new(x, z), target, context) {
                return true;
            }
        }
    }

    false
}

fn proximity_target_matches(
    position: IVec2,
    target: &StructureProximityTarget,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    if let Some(block) = target.block.as_deref() {
        return surface_block_matches(position, context, |candidate| candidate == block);
    }
    if let Some(fluid) = target.fluid.as_deref() {
        return surface_fluid_matches(position, fluid, context);
    }
    unreachable!("validated proximity target must define block or fluid")
}

fn surface_fluid_matches(
    position: IVec2,
    target_fluid: &str,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    if authored_surface_fluid_id_at(position, context).is_some_and(|fluid| fluid == target_fluid) {
        return true;
    }

    let horizontal = position.as_vec2() + Vec2::splat(0.5);
    let surface = context.biome_field.sample_surface(horizontal);
    let surface_height =
        surface_height_from_sample(position, context.dimension, context.biome_field, &surface);
    let mut chunk_coord =
        chunk_coord_from_world(IVec3::new(position.x, (surface_height - 1).max(0), position.y));
    chunk_coord.y = chunk_coord.y.max(0);
    let region = context.region(generation_region_coord(chunk_coord));

    region
        .hydrology
        .supported_water_at(horizontal, surface_height as f32)
        .is_some_and(|water| water.fluid_id == target_fluid)
}

fn intersects_surface_fluid(
    structure: &StructureDefinition,
    anchor: IVec2,
    origin_y: i32,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    structure.column_spans().iter().any(|span| {
        let position = anchor + span.offset;
        let horizontal = position.as_vec2() + Vec2::splat(0.5);
        let surface = context.biome_field.sample_surface(horizontal);
        let surface_height = surface_height_from_sample(
            position,
            context.dimension,
            context.biome_field,
            &surface,
        );
        let mut chunk_coord = chunk_coord_from_world(IVec3::new(
            position.x,
            (surface_height - 1).max(0),
            position.y,
        ));
        chunk_coord.y = chunk_coord.y.max(0);
        let region = context.region(generation_region_coord(chunk_coord));
        let Some(water) = region
            .hydrology
            .supported_water_at(horizontal, surface_height as f32)
        else {
            return false;
        };

        let structure_min = origin_y + span.min_y_offset;
        let structure_max = origin_y + span.max_y_offset;
        structure_max as f32 + 1.0 > water.bed_level
            && (structure_min as f32) < water.water_level
    })
}
