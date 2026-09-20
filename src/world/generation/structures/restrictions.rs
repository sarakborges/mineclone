use bevy::prelude::*;

use crate::{
    content::{
        structure::StructureDefinition,
        structure_rules::StructureFluidPolicy,
    },
    voxel::coordinates::chunk_coord_from_world,
    world::{
        generation_region::generation_region_coord,
        terrain::surface_height_from_sample,
    },
};

use super::super::ChunkGenerationContext;

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

    true
}

fn ground_block_is_allowed(
    position: IVec2,
    allowed: &[String],
    context: &ChunkGenerationContext<'_>,
) -> bool {
    let surface = context
        .biome_field
        .sample_surface(position.as_vec2() + Vec2::splat(0.5));

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
            return allowed.iter().any(|candidate| candidate == block);
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
        .is_some_and(|(block, _)| allowed.iter().any(|candidate| candidate == block))
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
