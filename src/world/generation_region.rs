use bevy::prelude::*;

use crate::voxel::chunk::CHUNK_SIZE;

pub const GENERATION_REGION_SIZE_CHUNKS: i32 = 8;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GenerationRegionCoord(IVec3);

impl GenerationRegionCoord {
    pub(crate) fn from_chunk_coord(chunk_coord: IVec3) -> Self {
        Self(IVec3::new(
            chunk_coord.x.div_euclid(GENERATION_REGION_SIZE_CHUNKS),
            chunk_coord
                .y
                .div_euclid(GENERATION_REGION_SIZE_CHUNKS)
                .max(0),
            chunk_coord.z.div_euclid(GENERATION_REGION_SIZE_CHUNKS),
        ))
    }

    pub(crate) fn from_region_coord(coord: IVec3) -> Self {
        debug_assert!(
            coord.y >= 0,
            "generation region Y cannot be negative: {}",
            coord.y
        );
        Self(IVec3::new(coord.x, coord.y.max(0), coord.z))
    }

    pub(crate) fn as_ivec3(self) -> IVec3 {
        self.0
    }
}

pub fn generation_region_coord(chunk_coord: IVec3) -> IVec3 {
    GenerationRegionCoord::from_chunk_coord(chunk_coord).as_ivec3()
}

pub fn generation_region_world_bounds(coord: IVec3) -> (Vec3, Vec3) {
    let coord = GenerationRegionCoord::from_region_coord(coord).as_ivec3();
    let size = (GENERATION_REGION_SIZE_CHUNKS * CHUNK_SIZE as i32) as f32;
    let minimum = Vec3::new(
        coord.x as f32 * size,
        coord.y as f32 * size,
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
    fn typed_generation_region_preserves_region_identity() {
        let region = GenerationRegionCoord::from_chunk_coord(IVec3::new(16, 16, -1));
        assert_eq!(region.as_ivec3(), IVec3::new(2, 2, -1));
    }

    #[test]
    fn adjacent_region_bounds_share_the_same_boundary() {
        let (_, left_max) = generation_region_world_bounds(IVec3::ZERO);
        let (right_min, _) = generation_region_world_bounds(IVec3::X);

        assert_eq!(left_max.x, right_min.x);
        assert_eq!(right_min.y, 0.0);
    }
}
