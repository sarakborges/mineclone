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

    pub fn rotate_uvs(self, uvs: [[f32; 2]; 4]) -> [[f32; 2]; 4] {
        match self {
            Self::Degrees0 => uvs,
            Self::Degrees90 => [uvs[3], uvs[0], uvs[1], uvs[2]],
            Self::Degrees180 => [uvs[2], uvs[3], uvs[0], uvs[1]],
            Self::Degrees270 => [uvs[1], uvs[2], uvs[3], uvs[0]],
        }
    }
}
