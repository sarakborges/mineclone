use bevy::prelude::*;

pub const RENDER_DISTANCE_RADIUS: i32 = 16;

pub fn chunk_coords_in_radius(center: IVec2, radius: i32) -> Vec<IVec2> {
    let mut coords = Vec::new();
    let radius_squared = radius * radius;

    for z in -radius..=radius {
        for x in -radius..=radius {
            if x * x + z * z <= radius_squared {
                coords.push(center + IVec2::new(x, z));
            }
        }
    }

    coords
}
