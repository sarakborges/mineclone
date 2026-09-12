use bevy::prelude::*;

use super::BlockFace;
use crate::{
    content::{block::BlockRegistry, block_orientation::BlockOrientation},
    voxel::{
        orientation::orient_vector, texture_rotation::TextureRotation, world::VoxelWorld,
    },
};

pub(super) struct FaceGeometry {
    pub vertices: [[f32; 3]; 4],
    pub normal: [f32; 3],
    pub texture_rotation: TextureRotation,
}

pub(super) fn is_face_exposed(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    block_id: &str,
    world_voxel: IVec3,
    face: BlockFace,
) -> bool {
    if face == BlockFace::Bottom && world_voxel.y <= 0 {
        return false;
    }

    let neighbor_position = world_voxel + face.offset();
    let Some(neighbor_id) = world.block_id_at(neighbor_position) else {
        return true;
    };
    let block = blocks
        .get(block_id)
        .unwrap_or_else(|| panic!("missing block definition: {block_id}"));
    let neighbor = blocks
        .get(neighbor_id)
        .unwrap_or_else(|| panic!("missing block definition: {neighbor_id}"));
    let block_is_transparent = block.alpha_blend || block.alpha_cutoff.is_some();
    let neighbor_occludes = !neighbor.alpha_blend && neighbor.alpha_cutoff.is_none();

    if block_id == neighbor_id && block_is_transparent {
        return false;
    }

    !neighbor_occludes
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
