use bevy::prelude::*;

use super::BlockFace;
use crate::{
    content::{block::BlockRegistry, block_orientation::BlockOrientation},
    voxel::{texture_rotation::TextureRotation, world::VoxelWorld},
};

const FACE_OVERDRAW: f32 = 0.002;

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

    let neighbor_position = world_voxel + face_offset(face);
    let Some(neighbor_id) = world.block_id_at(neighbor_position) else {
        return true;
    };
    let block = blocks
        .get(block_id)
        .unwrap_or_else(|| panic!("missing block definition: {block_id}"));
    let neighbor = blocks
        .get(neighbor_id)
        .unwrap_or_else(|| panic!("missing block definition: {neighbor_id}"));
    let neighbor_occludes = !neighbor.alpha_blend && neighbor.alpha_cutoff.is_none();

    if block_id == neighbor_id && block.alpha_blend {
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
    let x0 = x as f32;
    let y0 = y as f32;
    let z0 = z as f32;
    let x1 = x0 + 1.0;
    let y1 = y0 + 1.0;
    let z1 = z0 + 1.0;
    let e = FACE_OVERDRAW;

    match face {
        BlockFace::Right => FaceGeometry {
            vertices: [
                [x1, y0 - e, z1 + e],
                [x1, y0 - e, z0 - e],
                [x1, y1 + e, z0 - e],
                [x1, y1 + e, z1 + e],
            ],
            normal: [1.0, 0.0, 0.0],
            texture_rotation,
        },
        BlockFace::Left => FaceGeometry {
            vertices: [
                [x0, y0 - e, z0 - e],
                [x0, y0 - e, z1 + e],
                [x0, y1 + e, z1 + e],
                [x0, y1 + e, z0 - e],
            ],
            normal: [-1.0, 0.0, 0.0],
            texture_rotation,
        },
        BlockFace::Top => FaceGeometry {
            vertices: [
                [x0 - e, y1, z1 + e],
                [x1 + e, y1, z1 + e],
                [x1 + e, y1, z0 - e],
                [x0 - e, y1, z0 - e],
            ],
            normal: [0.0, 1.0, 0.0],
            texture_rotation,
        },
        BlockFace::Bottom => FaceGeometry {
            vertices: [
                [x0 - e, y0, z0 - e],
                [x1 + e, y0, z0 - e],
                [x1 + e, y0, z1 + e],
                [x0 - e, y0, z1 + e],
            ],
            normal: [0.0, -1.0, 0.0],
            texture_rotation,
        },
        BlockFace::Front => FaceGeometry {
            vertices: [
                [x0 - e, y0 - e, z1],
                [x1 + e, y0 - e, z1],
                [x1 + e, y1 + e, z1],
                [x0 - e, y1 + e, z1],
            ],
            normal: [0.0, 0.0, 1.0],
            texture_rotation,
        },
        BlockFace::Back => FaceGeometry {
            vertices: [
                [x1 + e, y0 - e, z0],
                [x0 - e, y0 - e, z0],
                [x0 - e, y1 + e, z0],
                [x1 + e, y1 + e, z0],
            ],
            normal: [0.0, 0.0, -1.0],
            texture_rotation,
        },
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

fn orient_vector(vector: Vec3, orientation: BlockOrientation) -> Vec3 {
    match orientation {
        BlockOrientation::Y => vector,
        BlockOrientation::Z => Vec3::new(vector.x, -vector.z, vector.y),
        BlockOrientation::X => Vec3::new(vector.y, -vector.x, vector.z),
    }
}

fn face_offset(face: BlockFace) -> IVec3 {
    match face {
        BlockFace::Right => IVec3::X,
        BlockFace::Left => IVec3::NEG_X,
        BlockFace::Top => IVec3::Y,
        BlockFace::Bottom => IVec3::NEG_Y,
        BlockFace::Front => IVec3::Z,
        BlockFace::Back => IVec3::NEG_Z,
    }
}
