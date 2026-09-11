use bevy::prelude::*;

use crate::content::block_orientation::BlockOrientation;

use super::block_face::BlockFace;

pub(crate) fn orient_vector(vector: Vec3, orientation: BlockOrientation) -> Vec3 {
    orientation_matrix(orientation) * vector
}

pub(crate) fn orient_face(face: BlockFace, orientation: BlockOrientation) -> BlockFace {
    let offset = orient_vector(face.offset().as_vec3(), orientation).as_ivec3();
    BlockFace::from_offset(offset)
        .unwrap_or_else(|| panic!("block orientation produced invalid face offset: {offset}"))
}

pub(crate) fn orientation_rotation(orientation: BlockOrientation) -> Quat {
    Quat::from_mat3(&orientation_matrix(orientation))
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
}
