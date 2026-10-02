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
/// and maximum terrain-variation rule. World generation may additionally apply
/// authored minimum relief through `fit_structure_to_ground_with_slope_range`.
pub(crate) fn fit_structure_to_ground(
    anchor: IVec2,
    support_offsets: &[IVec2],
    minimum_offset_y: i32,
    max_slope: i32,
    ground_at: impl FnMut(IVec2) -> Option<i32>,
) -> Option<i32> {
    fit_structure_to_ground_with_slope_range(
        anchor,
        support_offsets,
        minimum_offset_y,
        0,
        max_slope,
        ground_at,
    )
}

fn fit_structure_to_ground_with_slope_range(
    anchor: IVec2,
    support_offsets: &[IVec2],
    minimum_offset_y: i32,
    min_slope: i32,
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

    if minimum_ground_y == i32::MAX {
        return None;
    }

    let slope = maximum_ground_y - minimum_ground_y;
    if slope < min_slope || slope > max_slope {
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
        return fit_structure_to_ground_with_slope_range(
            anchor,
            &structure.support_offsets_for_rotation(rotation),
            structure.ground_anchor_y_offset(),
            structure.restrictions.min_slope,
            structure.restrictions.max_slope,
            |_| Some(ground_y),
        );
    }

    let support_offsets = structure.support_offsets_for_rotation(rotation);
    fit_structure_to_ground_with_slope_range(
        anchor,
        &support_offsets,
        structure.ground_anchor_y_offset(),
        structure.restrictions.min_slope,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_slope_rejects_flat_ground() {
        let supports = [IVec2::ZERO, IVec2::new(4, 0)];
        let result = fit_structure_to_ground_with_slope_range(
            IVec2::ZERO,
            &supports,
            0,
            3,
            8,
            |_| Some(12),
        );

        assert_eq!(result, None);
    }

    #[test]
    fn minimum_slope_accepts_required_relief() {
        let supports = [IVec2::ZERO, IVec2::new(4, 0)];
        let result = fit_structure_to_ground_with_slope_range(
            IVec2::ZERO,
            &supports,
            1,
            3,
            8,
            |position| Some(if position.x == 0 { 12 } else { 17 }),
        );

        assert_eq!(result, Some(11));
    }

    #[test]
    fn maximum_slope_still_rejects_excessive_relief() {
        let supports = [IVec2::ZERO, IVec2::new(4, 0)];
        let result = fit_structure_to_ground(
            IVec2::ZERO,
            &supports,
            0,
            4,
            |position| Some(if position.x == 0 { 12 } else { 21 }),
        );

        assert_eq!(result, None);
    }
}
