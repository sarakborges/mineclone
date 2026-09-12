use bevy::prelude::IVec3;

#[derive(Clone, Copy, Default)]
pub enum TextureRotation {
    #[default]
    Degrees0,
    Degrees90,
    Degrees180,
    Degrees270,
}

impl TextureRotation {
    pub fn from_quarter_turn(turn: u8) -> Self {
        match turn % 4 {
            0 => Self::Degrees0,
            1 => Self::Degrees90,
            2 => Self::Degrees180,
            _ => Self::Degrees270,
        }
    }

    pub fn for_position(position: IVec3, enabled: bool) -> Self {
        if !enabled {
            return Self::default();
        }

        let mut hash = (position.x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= (position.y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
        hash ^= (position.z as i64 as u64).wrapping_mul(0x1656_67b1_9e37_79f9);

        hash ^= hash >> 30;
        hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        hash ^= hash >> 27;
        hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
        hash ^= hash >> 31;

        Self::from_quarter_turn((hash & 3) as u8)
    }

    pub fn rotate_uvs(self, uvs: [[f32; 2]; 4]) -> [[f32; 2]; 4] {
        match self {
            Self::Degrees0 => uvs,
            Self::Degrees90 => [uvs[3], uvs[0], uvs[1], uvs[2]],
            Self::Degrees180 => [uvs[2], uvs[3], uvs[0], uvs[1]],
            Self::Degrees270 => [uvs[1], uvs[2], uvs[3], uvs[0]],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_rotation_is_always_zero() {
        assert!(matches!(
            TextureRotation::for_position(IVec3::new(12, 34, 56), false),
            TextureRotation::Degrees0
        ));
    }

    #[test]
    fn position_rotation_is_deterministic() {
        let position = IVec3::new(12, 34, 56);
        let first = TextureRotation::for_position(position, true) as u8;
        let second = TextureRotation::for_position(position, true) as u8;

        assert_eq!(first, second);
    }

    #[test]
    fn neighboring_surface_voxels_do_not_form_large_rotation_bands() {
        const SIZE: i32 = 32;
        let mut matching_neighbors = 0;
        let mut neighbor_pairs = 0;
        let mut counts = [0_usize; 4];

        for z in 0..SIZE {
            for x in 0..SIZE {
                let position = IVec3::new(x, 64, z);
                let rotation = TextureRotation::for_position(position, true) as usize;
                counts[rotation] += 1;

                if x + 1 < SIZE {
                    neighbor_pairs += 1;
                    matching_neighbors += usize::from(
                        TextureRotation::for_position(IVec3::new(x + 1, 64, z), true) as usize
                            == rotation,
                    );
                }
                if z + 1 < SIZE {
                    neighbor_pairs += 1;
                    matching_neighbors += usize::from(
                        TextureRotation::for_position(IVec3::new(x, 64, z + 1), true) as usize
                            == rotation,
                    );
                }
            }
        }

        assert!(counts.into_iter().all(|count| count > 150));
        assert!(matching_neighbors * 100 < neighbor_pairs * 40);
    }
}
