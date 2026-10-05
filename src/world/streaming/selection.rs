use bevy::{platform::collections::HashSet, prelude::*};

use crate::{
    voxel::chunk::CHUNK_SIZE,
    world::generator::WorldGenerator,
};

const SURFACE_PADDING_BELOW_CHUNKS: i32 = 2;
const SURFACE_PADDING_ABOVE_CHUNKS: i32 = 1;
const PLAYER_LOCAL_HORIZONTAL_RADIUS_CHUNKS: i32 = 2;
const PLAYER_LOCAL_VERTICAL_RADIUS_CHUNKS: i32 = 2;

pub(super) fn desired_chunk_coords(
    generator: &WorldGenerator,
    center: IVec3,
    horizontal_radius: i32,
) -> HashSet<IVec3> {
    let radius = horizontal_radius.max(1);
    let radius_squared = radius.saturating_mul(radius);
    let chunk_size = CHUNK_SIZE as i32;
    let mut desired = HashSet::default();

    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if dx.saturating_mul(dx).saturating_add(dz.saturating_mul(dz)) > radius_squared {
                continue;
            }

            let horizontal = center.xz() + IVec2::new(dx, dz);
            let Some(origin_x) = horizontal.x.checked_mul(chunk_size) else {
                continue;
            };
            let Some(origin_z) = horizontal.y.checked_mul(chunk_size) else {
                continue;
            };
            let surface = generator.terrain().sample_surface_area(
                origin_x,
                origin_z,
                CHUNK_SIZE as u32,
                CHUNK_SIZE as u32,
            );
            let mut minimum_surface = i32::MAX;
            let mut maximum_surface = i32::MIN;
            for z in 0..CHUNK_SIZE as u32 {
                for x in 0..CHUNK_SIZE as u32 {
                    let column = surface
                        .sample_at(x, z)
                        .expect("matching streaming surface area must contain each chunk column");
                    minimum_surface = minimum_surface.min(column.surface_y());
                    maximum_surface = maximum_surface.max(column.surface_y());
                }
            }

            let minimum_chunk = minimum_surface
                .div_euclid(chunk_size)
                .saturating_sub(SURFACE_PADDING_BELOW_CHUNKS)
                .max(0);
            let maximum_chunk = maximum_surface
                .div_euclid(chunk_size)
                .saturating_add(SURFACE_PADDING_ABOVE_CHUNKS)
                .max(minimum_chunk);
            insert_vertical_range(&mut desired, horizontal, minimum_chunk, maximum_chunk);

            for placement in generator.structures().placements_intersecting(
                origin_x,
                origin_z,
                CHUNK_SIZE as u32,
                CHUNK_SIZE as u32,
            ) {
                let (minimum_y, maximum_y) = placement.vertical_bounds();
                let attachment_rise = placement.structure().restrictions.max_slope.max(0);
                let minimum_y = minimum_y.div_euclid(chunk_size).max(0);
                let maximum_y = maximum_y
                    .saturating_add(attachment_rise)
                    .div_euclid(chunk_size)
                    .max(minimum_y);
                insert_vertical_range(&mut desired, horizontal, minimum_y, maximum_y);
            }

            if dx.abs() <= PLAYER_LOCAL_HORIZONTAL_RADIUS_CHUNKS
                && dz.abs() <= PLAYER_LOCAL_HORIZONTAL_RADIUS_CHUNKS
            {
                let minimum_local = center
                    .y
                    .saturating_sub(PLAYER_LOCAL_VERTICAL_RADIUS_CHUNKS)
                    .max(0);
                let maximum_local = center
                    .y
                    .saturating_add(PLAYER_LOCAL_VERTICAL_RADIUS_CHUNKS);
                insert_vertical_range(&mut desired, horizontal, minimum_local, maximum_local);
            }
        }
    }

    desired
}

fn insert_vertical_range(
    desired: &mut HashSet<IVec3>,
    horizontal: IVec2,
    minimum_y: i32,
    maximum_y: i32,
) {
    for y in minimum_y..=maximum_y {
        desired.insert(IVec3::new(horizontal.x, y, horizontal.y));
    }
}
