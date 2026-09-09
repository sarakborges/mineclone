use bevy::prelude::*;

use super::{
    cave_connectivity::CaveConnectivityRegion,
    geology::GeologyRegion,
    hydrology::HydrologyRegion,
};

pub const GENERATION_REGION_SIZE_CHUNKS: i32 = 8;

#[derive(Clone, Debug)]
pub struct GenerationRegion {
    pub coord: IVec3,
    pub hydrology: HydrologyRegion,
    pub cave_connectivity: CaveConnectivityRegion,
    pub geology: GeologyRegion,
}

pub fn generation_region_coord(chunk_coord: IVec3) -> IVec3 {
    IVec3::new(
        chunk_coord.x.div_euclid(GENERATION_REGION_SIZE_CHUNKS),
        chunk_coord.y.div_euclid(GENERATION_REGION_SIZE_CHUNKS).max(0),
        chunk_coord.z.div_euclid(GENERATION_REGION_SIZE_CHUNKS),
    )
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
}
