use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum BlockFace {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}

impl BlockFace {
    pub(crate) const ALL: [Self; 6] = [
        Self::Right,
        Self::Left,
        Self::Top,
        Self::Bottom,
        Self::Front,
        Self::Back,
    ];

    pub(crate) fn unit_vertices(self) -> [[f32; 3]; 4] {
        match self {
            Self::Right => [
                [1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
            ],
            Self::Left => [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
            ],
            Self::Top => [
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            Self::Bottom => [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            Self::Front => [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
            Self::Back => [
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
        }
    }

    pub(crate) fn normal(self) -> [f32; 3] {
        match self {
            Self::Right => [1.0, 0.0, 0.0],
            Self::Left => [-1.0, 0.0, 0.0],
            Self::Top => [0.0, 1.0, 0.0],
            Self::Bottom => [0.0, -1.0, 0.0],
            Self::Front => [0.0, 0.0, 1.0],
            Self::Back => [0.0, 0.0, -1.0],
        }
    }

    pub(crate) fn offset(self) -> IVec3 {
        match self {
            Self::Right => IVec3::X,
            Self::Left => IVec3::NEG_X,
            Self::Top => IVec3::Y,
            Self::Bottom => IVec3::NEG_Y,
            Self::Front => IVec3::Z,
            Self::Back => IVec3::NEG_Z,
        }
    }
}

#[derive(Clone)]
pub(crate) struct BlockFaces<T> {
    right: T,
    left: T,
    top: T,
    bottom: T,
    front: T,
    back: T,
}

impl<T> BlockFaces<T> {
    pub(crate) fn from_fn(mut value_for: impl FnMut(BlockFace) -> T) -> Self {
        Self {
            right: value_for(BlockFace::Right),
            left: value_for(BlockFace::Left),
            top: value_for(BlockFace::Top),
            bottom: value_for(BlockFace::Bottom),
            front: value_for(BlockFace::Front),
            back: value_for(BlockFace::Back),
        }
    }

    pub(crate) fn get(&self, face: BlockFace) -> &T {
        match face {
            BlockFace::Right => &self.right,
            BlockFace::Left => &self.left,
            BlockFace::Top => &self.top,
            BlockFace::Bottom => &self.bottom,
            BlockFace::Front => &self.front,
            BlockFace::Back => &self.back,
        }
    }
}
