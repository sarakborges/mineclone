use bevy::prelude::*;

use crate::voxel::chunk::CHUNK_SIZE;

use super::{geology::GeologyRegion, hydrology::HydrologyRegion};

pub const GENERATION_REGION_SIZE_CHUNKS: i32 = 8;

#[derive(Clone, Debug)]
pub struct GenerationRegion {
    pub coord: IVec3,
    pub hydrology: HydrologyRegion,
    pub geology: GeologyRegion,
}

pub fn generation_region_coord(chunk_coord: IVec3) -> IVec3 {
    IVec3::new(
        chunk_coord.x.div_euclid(GENERATION_REGION_SIZE_CHUNKS),
        chunk_coord
            .y
            .div_euclid(GENERATION_REGION_SIZE_CHUNKS)
            .max(0),
        chunk_coord.z.div_euclid(GENERATION_REGION_SIZE_CHUNKS),
    )
}

pub fn generation_region_world_bounds(coord: IVec3) -> (Vec3, Vec3) {
    let size = (GENERATION_REGION_SIZE_CHUNKS * CHUNK_SIZE as i32) as f32;
    let minimum = Vec3::new(
        coord.x as f32 * size,
        coord.y.max(0) as f32 * size,
        coord.z as f32 * size,
    );

    (minimum, minimum + Vec3::splat(size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_regions_keep_y_non_negative() {
        assert_eq!(
            generation_region_coord(IVec3::new(16, 16, -1)),
            IVec3::new(2, 2, -1)
        );
        assert_eq!(generation_region_coord(IVec3::new(0, -10, 0)).y, 0);
    }

    #[test]
    fn adjacent_region_bounds_share_the_same_boundary() {
        let (_, left_max) = generation_region_world_bounds(IVec3::ZERO);
        let (right_min, _) = generation_region_world_bounds(IVec3::X);

        assert_eq!(left_max.x, right_min.x);
        assert_eq!(right_min.y, 0.0);
    }
}
