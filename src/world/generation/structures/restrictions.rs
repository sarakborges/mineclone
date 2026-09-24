use bevy::prelude::*;

use crate::{
    content::{
        structure::{StructureDefinition, StructureRotation},
        structure_rules::{
            StructureFluidPolicy, StructureProximityMode, StructureProximityRestriction,
            StructureProximityTarget,
        },
    },
    world::{
        biome_field::BiomeFieldSample,
        new_world::WorldGenerationMode,
        terrain::surface_height_from_sample,
    },
};

use super::super::{ChunkGenerationContext, fluids::authored_surface_fluid_id_at};

struct NormalSurfaceProbe<'a> {
    surface: BiomeFieldSample<'a>,
    surface_height: i32,
}

fn normal_surface_probe<'a>(
    position: IVec2,
    context: &ChunkGenerationContext<'a>,
) -> NormalSurfaceProbe<'a> {
    let horizontal = position.as_vec2() + Vec2::splat(0.5);
    let surface = context.biome_field.sample_surface(horizontal);
    let surface_height =
        surface_height_from_sample(position, context.dimension, context.biome_field, &surface);
    NormalSurfaceProbe {
        surface,
        surface_height,
    }
}

pub(super) fn candidate_satisfies_restrictions(
    biome_id: &str,
    structure: &StructureDefinition,
    rotation: StructureRotation,
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
        && structure
            .support_offsets_for_rotation(rotation)
            .iter()
            .any(|offset| {
                !ground_block_is_allowed(anchor + *offset, &restrictions.ground_blocks, context)
            })
    {
        return false;
    }

    if restrictions.required_biome_coverage > 0.0 {
        let footprint = structure.horizontal_footprint_for_rotation(rotation);
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
        && intersects_surface_fluid(structure, rotation, anchor, origin_y, context)
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
    if context.world_generation.mode() == WorldGenerationMode::Void {
        return false;
    }
    if context.world_generation.mode() == WorldGenerationMode::Flat {
        let surface = context
            .biome_field
            .sample_surface(position.as_vec2() + Vec2::splat(0.5));
        let biome_id = context
            .biome_field
            .surface_biome_id(surface.identity_surface_index);
        let biome = context
            .biomes
            .get(biome_id)
            .unwrap_or_else(|| panic!("missing flat-world biome definition: {biome_id}"));
        return biome.surface_block_at_depth(0).is_some_and(matches);
    }

    let probe = normal_surface_probe(position, context);

    if let Some(margin_index) = probe.surface.surface_margin_index {
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

    probe
        .surface
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

pub(super) fn surface_has_fluid(
    position: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    if context.world_generation.mode() != WorldGenerationMode::Normal {
        return false;
    }
    if authored_surface_fluid_id_at(position, context).is_some() {
        return true;
    }

    let probe = normal_surface_probe(position, context);
    ocean_surface_water_at(&probe, context)
}

fn surface_fluid_matches(
    position: IVec2,
    target_fluid: &str,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    if context.world_generation.mode() != WorldGenerationMode::Normal {
        return false;
    }
    if authored_surface_fluid_id_at(position, context).is_some_and(|fluid| fluid == target_fluid) {
        return true;
    }

    let probe = normal_surface_probe(position, context);
    ocean_surface_water_at(&probe, context) && context.dimension.sea_fluid == target_fluid
}

fn ocean_surface_water_at(
    probe: &NormalSurfaceProbe<'_>,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    if !context.world_generation.spawn_oceans()
        || probe.surface_height >= context.dimension.sea_level
    {
        return false;
    }

    let Some(ocean_index) = context.biome_field.ocean_surface_index() else {
        return false;
    };
    probe
        .surface
        .influences
        .iter()
        .any(|influence| influence.surface_index == ocean_index && influence.weight > f32::EPSILON)
}

fn intersects_surface_fluid(
    structure: &StructureDefinition,
    rotation: StructureRotation,
    anchor: IVec2,
    origin_y: i32,
    context: &ChunkGenerationContext<'_>,
) -> bool {
    if context.world_generation.mode() != WorldGenerationMode::Normal {
        return false;
    }

    structure.column_spans().iter().any(|span| {
        let position = anchor + rotation.rotate_horizontal(span.offset);
        let probe = normal_surface_probe(position, context);
        let structure_min = origin_y + span.min_y_offset;
        let structure_max = origin_y + span.max_y_offset;

        if ocean_surface_water_at(&probe, context) {
            return structure_max as f32 + 1.0 > probe.surface_height as f32
                && (structure_min as f32) < context.dimension.sea_level as f32;
        }

        authored_surface_fluid_id_at(position, context).is_some()
            && structure_max >= probe.surface_height
    })
}
