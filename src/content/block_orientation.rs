use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BlockOrientation {
    X,
    #[default]
    Y,
    Z,
}

impl BlockOrientation {
    pub fn from_index(index: u8) -> Self {
        match index {
            1 => Self::Z,
            2 => Self::X,
            _ => Self::Y,
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Self::Y => 0,
            Self::Z => 1,
            Self::X => 2,
        }
    }
}
