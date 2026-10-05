use bevy::prelude::{IVec2, IVec3};

use crate::voxel::spatial_search::find_map_square_rings;

use super::generator::WorldGenerator;

/// Query-only destination preparation for untouched/generated world space.
///
/// This narrows a caller's destination to a generated surface candidate without
/// materializing chunks or reading mutable runtime state. Callers must still
/// perform a small runtime validation after streaming has made the destination
/// neighborhood resident.
pub(crate) fn find_generated_surface_destination(
    generator: &WorldGenerator,
    preferred_column: IVec2,
    max_radius: i32,
    mut accepts_column: impl FnMut(IVec2) -> bool,
) -> Option<IVec3> {
    assert!(max_radius >= 0, "destination search radius must be nonnegative");

    let (origin_x, width) = search_axis_bounds(preferred_column.x, max_radius);
    let (origin_z, depth) = search_axis_bounds(preferred_column.y, max_radius);
    let structure_bounds = generator
        .structures()
        .placements_intersecting(origin_x, origin_z, width, depth)
        .into_iter()
        .map(|placement| placement.horizontal_bounds())
        .collect::<Vec<_>>();

    find_map_square_rings(preferred_column, max_radius, 1, |column| {
        if !accepts_column(column)
            || structure_bounds
                .iter()
                .any(|(minimum, maximum)| column_inside_bounds(column, *minimum, *maximum))
        {
            return None;
        }

        generated_surface_feet_y(generator, column)
            .map(|feet_y| IVec3::new(column.x, feet_y, column.y))
    })
}

fn generated_surface_feet_y(generator: &WorldGenerator, column: IVec2) -> Option<i32> {
    let surface_y = generator.terrain().surface_at(column.x, column.y);
    if surface_y < 0 {
        return None;
    }

    let feet_y = surface_y.checked_add(1)?;
    let head_y = feet_y.checked_add(1)?;
    let materials = generator.materials();
    materials.solid_block_at(column.x, surface_y, column.y)?;

    for y in [feet_y, head_y] {
        if materials.solid_block_at(column.x, y, column.y).is_some()
            || materials
                .generated_fluid_at(column.x, y, column.y)
                .is_some()
        {
            return None;
        }
    }

    Some(feet_y)
}

fn search_axis_bounds(center: i32, radius: i32) -> (i32, u32) {
    let minimum = center.saturating_sub(radius);
    let maximum = center.saturating_add(radius);
    let width = u32::try_from(i64::from(maximum) - i64::from(minimum) + 1)
        .expect("destination search width must fit u32");
    (minimum, width)
}

fn column_inside_bounds(column: IVec2, minimum: IVec2, maximum: IVec2) -> bool {
    column.x >= minimum.x
        && column.x <= maximum.x
        && column.y >= minimum.y
        && column.y <= maximum.y
}
