use bevy::prelude::*;

use super::BlockFace;
use crate::{
    content::{block::BlockLookup, block_orientation::BlockOrientation},
    voxel::{
        cell::VoxelCell, orientation::orient_vector, read::VoxelRead,
        texture_rotation::TextureRotation,
    },
};

pub(super) struct FaceGeometry {
    pub vertices: [[f32; 3]; 4],
    pub normal: [f32; 3],
    pub texture_rotation: TextureRotation,
}

pub(super) fn is_face_exposed_against_neighbor(
    blocks: &mut BlockLookup<'_>,
    block_id: &str,
    block_is_transparent: bool,
    world_voxel: IVec3,
    face: BlockFace,
    neighbor: Option<VoxelCell>,
) -> bool {
    if face == BlockFace::Bottom && world_voxel.y <= 0 {
        return false;
    }

    let Some(neighbor) = neighbor else {
        return true;
    };
    let definition = blocks.get(neighbor.block_id);
    let neighbor_occludes = !definition.uses_custom_model()
        && !definition.alpha_blend
        && definition.alpha_cutoff.is_none();

    !face_is_occluded(
        block_id,
        neighbor.block_id,
        block_is_transparent,
        neighbor_occludes,
    )
}

pub(crate) fn is_face_exposed<W: VoxelRead + ?Sized>(
    world: &W,
    blocks: &mut BlockLookup<'_>,
    block_id: &str,
    block_is_transparent: bool,
    world_voxel: IVec3,
    face: BlockFace,
) -> bool {
    is_face_exposed_against_neighbor(
        blocks,
        block_id,
        block_is_transparent,
        world_voxel,
        face,
        world.cell_at(world_voxel + face.offset()),
    )
}

fn face_is_occluded(
    block_id: &str,
    neighbor_id: &str,
    block_is_transparent: bool,
    neighbor_occludes: bool,
) -> bool {
    (block_id == neighbor_id && block_is_transparent) || neighbor_occludes
}

pub(super) fn face_geometry(
    face: BlockFace,
    x: usize,
    y: usize,
    z: usize,
    texture_rotation: TextureRotation,
) -> FaceGeometry {
    let origin = Vec3::new(x as f32, y as f32, z as f32);

    FaceGeometry {
        vertices: face
            .unit_vertices()
            .map(|vertex| (Vec3::from_array(vertex) + origin).to_array()),
        normal: face.normal(),
        texture_rotation,
    }
}

pub(super) fn orient_face_geometry(
    mut geometry: FaceGeometry,
    orientation: BlockOrientation,
    x: usize,
    y: usize,
    z: usize,
) -> FaceGeometry {
    if orientation == BlockOrientation::Y {
        return geometry;
    }

    let center = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
    geometry.vertices = geometry.vertices.map(|vertex| {
        let vertex = Vec3::from_array(vertex);
        (center + orient_vector(vertex - center, orientation)).to_array()
    });
    geometry.normal = orient_vector(Vec3::from_array(geometry.normal), orientation).to_array();
    geometry
}

#[cfg(test)]
mod tests {
    use super::face_is_occluded;

    #[test]
    fn face_occlusion_preserves_opaque_and_transparent_neighbor_rules() {
        assert!(face_is_occluded("stone", "dirt", false, true));
        assert!(face_is_occluded("glass", "glass", true, false));
        assert!(!face_is_occluded("glass", "other_glass", true, false));
        assert!(!face_is_occluded("stone", "glass", false, false));
        assert!(face_is_occluded("glass", "stone", true, true));
    }
}
