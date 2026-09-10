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

        let mut hash = position.x as u32;
        hash ^= (position.y as u32).wrapping_mul(0x9e37_79b9);
        hash = hash.rotate_left(13);
        hash ^= (position.z as u32).wrapping_mul(0x85eb_ca6b);
        hash ^= hash >> 16;

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
}
