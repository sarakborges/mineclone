use bevy::prelude::*;

use crate::{
    content::structure::{StructureDefinition, StructureRotation},
    world::{
        new_world::WorldGenerationMode,
        terrain::surface_height_from_sample,
    },
};

use super::super::{ChunkGenerationContext, flat_surface_height};

/// Both world generation and /place use the exact same bottom-voxel footprint
/// and terrain-variation rule. The caller supplies ground samples from either
/// procedural density or the already-generated, possibly edited voxel world.
pub(crate) fn fit_structure_to_ground(
    anchor: IVec2,
    support_offsets: &[IVec2],
    minimum_offset_y: i32,
    max_slope: i32,
    mut ground_at: impl FnMut(IVec2) -> Option<i32>,
) -> Option<i32> {
    let mut minimum_ground_y = i32::MAX;
    let mut maximum_ground_y = i32::MIN;

    for &offset in support_offsets {
        let ground_y = ground_at(anchor + offset)?;
        minimum_ground_y = minimum_ground_y.min(ground_y);
        maximum_ground_y = maximum_ground_y.max(ground_y);
    }

    if minimum_ground_y == i32::MAX || maximum_ground_y - minimum_ground_y > max_slope {
        return None;
    }

    Some(minimum_ground_y - minimum_offset_y)
}

pub(super) fn compute_structure_origin_y(
    anchor: IVec2,
    structure: &StructureDefinition,
    rotation: StructureRotation,
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    if context.world_generation.mode() == WorldGenerationMode::Void {
        return None;
    }
    if context.world_generation.mode() == WorldGenerationMode::Flat {
        let ground_y = flat_surface_height(context.dimension) - 1;
        return fit_structure_to_ground(
            anchor,
            &structure.support_offsets_for_rotation(rotation),
            structure.ground_anchor_y_offset(),
            structure.restrictions.max_slope,
            |_| Some(ground_y),
        );
    }

    let support_offsets = structure.support_offsets_for_rotation(rotation);
    fit_structure_to_ground(
        anchor,
        &support_offsets,
        structure.ground_anchor_y_offset(),
        structure.restrictions.max_slope,
        |position| {
            if structure.restrictions.requires_dry_ground
                && super::restrictions::surface_has_fluid(position, context)
            {
                return None;
            }
            supported_surface_ground_y(position, context)
        },
    )
}

fn supported_surface_ground_y(
    position: IVec2,
    context: &ChunkGenerationContext<'_>,
) -> Option<i32> {
    let horizontal = position.as_vec2() + Vec2::splat(0.5);
    let surface = context.biome_field.sample_surface(horizontal);
    let surface_height = surface_height_from_sample(
        position,
        context.dimension,
        context.biome_field,
        &surface,
    );

    Some(surface_height - 1)
}
