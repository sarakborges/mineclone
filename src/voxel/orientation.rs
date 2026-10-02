use bevy::prelude::*;

use crate::content::{block::BlockDefinition, block_orientation::BlockOrientation};

use super::{block_face::BlockFace, cell::VoxelCell};

pub(crate) const HORIZONTAL_FACING_BLOCK_TAG: &str = "horizontal_facing";
pub(crate) const HORIZONTAL_FACING_STATE_KEY: &str = "facing";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum HorizontalFacing {
    North,
    East,
    #[default]
    South,
    West,
}

impl HorizontalFacing {
    pub(crate) fn state_value(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::East => "east",
            Self::South => "south",
            Self::West => "west",
        }
    }

    fn from_state_value(value: Option<&str>) -> Self {
        match value {
            Some("north") => Self::North,
            Some("east") => Self::East,
            Some("west") => Self::West,
            Some("south") | None | Some(_) => Self::South,
        }
    }

    fn quarter_turns_from_south(self) -> u8 {
        match self {
            Self::South => 0,
            Self::East => 1,
            Self::North => 2,
            Self::West => 3,
        }
    }
}

pub(crate) fn block_uses_horizontal_facing(block: &BlockDefinition) -> bool {
    block
        .tags
        .iter()
        .any(|tag| tag == HORIZONTAL_FACING_BLOCK_TAG)
}

pub(crate) fn horizontal_facing_toward_player(
    voxel: IVec3,
    player_position: Vec3,
) -> HorizontalFacing {
    let center = voxel.as_vec3() + Vec3::splat(0.5);
    let offset = player_position - center;
    if offset.x.abs() > offset.z.abs() {
        if offset.x >= 0.0 {
            HorizontalFacing::East
        } else {
            HorizontalFacing::West
        }
    } else if offset.z >= 0.0 {
        HorizontalFacing::South
    } else {
        HorizontalFacing::North
    }
}

pub(crate) fn horizontal_facing_for_cell(cell: VoxelCell) -> HorizontalFacing {
    HorizontalFacing::from_state_value(cell.state(HORIZONTAL_FACING_STATE_KEY))
}

pub(crate) fn orient_vector(vector: Vec3, orientation: BlockOrientation) -> Vec3 {
    orientation_matrix(orientation) * vector
}

pub(crate) fn orient_face(face: BlockFace, orientation: BlockOrientation) -> BlockFace {
    let offset = orient_vector(face.offset().as_vec3(), orientation).as_ivec3();
    BlockFace::from_offset(offset)
        .unwrap_or_else(|| panic!("block orientation produced invalid face offset: {offset}"))
}

pub(crate) fn source_face_for_oriented_face(
    oriented_face: BlockFace,
    orientation: BlockOrientation,
) -> BlockFace {
    BlockFace::ALL
        .into_iter()
        .find(|source_face| orient_face(*source_face, orientation) == oriented_face)
        .unwrap_or_else(|| {
            panic!("block orientation has no source face for oriented face: {oriented_face:?}")
        })
}

pub(crate) fn source_face_for_horizontal_facing(
    oriented_face: BlockFace,
    facing: HorizontalFacing,
) -> BlockFace {
    BlockFace::ALL
        .into_iter()
        .find(|source_face| orient_horizontal_face(*source_face, facing) == oriented_face)
        .unwrap_or_else(|| {
            panic!("horizontal facing has no source face for oriented face: {oriented_face:?}")
        })
}

/// Returns the block-local visual face that must be rendered on a world face.
/// Axis orientation is applied first, then optional horizontal facing.
pub(crate) fn source_face_for_cell_visual(
    world_face: BlockFace,
    cell: VoxelCell,
    block: &BlockDefinition,
) -> BlockFace {
    let before_horizontal_facing = if block_uses_horizontal_facing(block) {
        source_face_for_horizontal_facing(world_face, horizontal_facing_for_cell(cell))
    } else {
        world_face
    };

    if cell.orientation == BlockOrientation::Y {
        before_horizontal_facing
    } else {
        source_face_for_oriented_face(before_horizontal_facing, cell.orientation)
    }
}

pub(crate) fn orientation_rotation(orientation: BlockOrientation) -> Quat {
    Quat::from_mat3(&orientation_matrix(orientation))
}

pub(crate) fn horizontal_facing_rotation(facing: HorizontalFacing) -> Quat {
    Quat::from_rotation_y(
        facing.quarter_turns_from_south() as f32 * std::f32::consts::FRAC_PI_2,
    )
}

pub(crate) fn block_rotation(
    orientation: BlockOrientation,
    facing: Option<HorizontalFacing>,
) -> Quat {
    facing.map_or(Quat::IDENTITY, horizontal_facing_rotation) * orientation_rotation(orientation)
}

fn orient_horizontal_face(mut face: BlockFace, facing: HorizontalFacing) -> BlockFace {
    for _ in 0..facing.quarter_turns_from_south() {
        face = match face {
            BlockFace::Front => BlockFace::Right,
            BlockFace::Right => BlockFace::Back,
            BlockFace::Back => BlockFace::Left,
            BlockFace::Left => BlockFace::Front,
            BlockFace::Top => BlockFace::Top,
            BlockFace::Bottom => BlockFace::Bottom,
        };
    }
    face
}

fn orientation_matrix(orientation: BlockOrientation) -> Mat3 {
    match orientation {
        BlockOrientation::Y => Mat3::IDENTITY,
        BlockOrientation::Z => Mat3::from_cols(Vec3::X, Vec3::Z, Vec3::NEG_Y),
        BlockOrientation::X => Mat3::from_cols(Vec3::NEG_Y, Vec3::X, Vec3::Z),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_maps_top_face_to_selected_axis() {
        assert_eq!(orient_face(BlockFace::Top, BlockOrientation::Y), BlockFace::Top);
        assert_eq!(orient_face(BlockFace::Top, BlockOrientation::Z), BlockFace::Front);
        assert_eq!(orient_face(BlockFace::Top, BlockOrientation::X), BlockFace::Right);
    }

    #[test]
    fn source_face_inverts_oriented_face_mapping() {
        assert_eq!(
            source_face_for_oriented_face(BlockFace::Front, BlockOrientation::Z),
            BlockFace::Top,
        );
        assert_eq!(
            source_face_for_oriented_face(BlockFace::Right, BlockOrientation::X),
            BlockFace::Top,
        );
    }

    #[test]
    fn horizontal_facing_moves_front_to_cardinal_world_face() {
        assert_eq!(
            source_face_for_horizontal_facing(BlockFace::Right, HorizontalFacing::East),
            BlockFace::Front,
        );
        assert_eq!(
            source_face_for_horizontal_facing(BlockFace::Back, HorizontalFacing::North),
            BlockFace::Front,
        );
        assert_eq!(
            source_face_for_horizontal_facing(BlockFace::Left, HorizontalFacing::West),
            BlockFace::Front,
        );
        assert_eq!(
            source_face_for_horizontal_facing(BlockFace::Front, HorizontalFacing::South),
            BlockFace::Front,
        );
    }

    #[test]
    fn player_position_selects_nearest_cardinal_facing() {
        let voxel = IVec3::ZERO;
        assert_eq!(
            horizontal_facing_toward_player(voxel, Vec3::new(3.0, 0.0, 0.5)),
            HorizontalFacing::East,
        );
        assert_eq!(
            horizontal_facing_toward_player(voxel, Vec3::new(0.5, 0.0, -3.0)),
            HorizontalFacing::North,
        );
    }
}
