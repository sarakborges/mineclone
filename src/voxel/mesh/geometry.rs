use bevy::prelude::*;

use super::BlockFace;
use crate::voxel::{texture_rotation::TextureRotation, world::VoxelWorld};

const FACE_OVERDRAW: f32 = 0.002;

pub(super) struct FaceGeometry {
    pub vertices: [[f32; 3]; 4],
    pub normal: [f32; 3],
    pub texture_rotation: TextureRotation,
}

pub(super) fn is_face_exposed(
    world: &VoxelWorld,
    world_voxel: IVec3,
    face: BlockFace,
) -> bool {
    if face == BlockFace::Bottom && world_voxel.y <= 0 {
        return false;
    }

    !world.is_solid(world_voxel + face_offset(face))
}

pub(super) fn face_geometry(
    face: BlockFace,
    x: usize,
    y: usize,
    z: usize,
    block_rotation: TextureRotation,
) -> FaceGeometry {
    let x0 = x as f32;
    let y0 = y as f32;
    let z0 = z as f32;
    let x1 = x0 + 1.0;
    let y1 = y0 + 1.0;
    let z1 = z0 + 1.0;
    let e = FACE_OVERDRAW;
    let default_rotation = TextureRotation::default();

    match face {
        BlockFace::Right => FaceGeometry {
            vertices: [
                [x1, y0 - e, z1 + e],
                [x1, y0 - e, z0 - e],
                [x1, y1 + e, z0 - e],
                [x1, y1 + e, z1 + e],
            ],
            normal: [1.0, 0.0, 0.0],
            texture_rotation: default_rotation,
        },
        BlockFace::Left => FaceGeometry {
            vertices: [
                [x0, y0 - e, z0 - e],
                [x0, y0 - e, z1 + e],
                [x0, y1 + e, z1 + e],
                [x0, y1 + e, z0 - e],
            ],
            normal: [-1.0, 0.0, 0.0],
            texture_rotation: default_rotation,
        },
        BlockFace::Top => FaceGeometry {
            vertices: [
                [x0 - e, y1, z1 + e],
                [x1 + e, y1, z1 + e],
                [x1 + e, y1, z0 - e],
                [x0 - e, y1, z0 - e],
            ],
            normal: [0.0, 1.0, 0.0],
            texture_rotation: block_rotation,
        },
        BlockFace::Bottom => FaceGeometry {
            vertices: [
                [x0 - e, y0, z0 - e],
                [x1 + e, y0, z0 - e],
                [x1 + e, y0, z1 + e],
                [x0 - e, y0, z1 + e],
            ],
            normal: [0.0, -1.0, 0.0],
            texture_rotation: block_rotation,
        },
        BlockFace::Front => FaceGeometry {
            vertices: [
                [x0 - e, y0 - e, z1],
                [x1 + e, y0 - e, z1],
                [x1 + e, y1 + e, z1],
                [x0 - e, y1 + e, z1],
            ],
            normal: [0.0, 0.0, 1.0],
            texture_rotation: default_rotation,
        },
        BlockFace::Back => FaceGeometry {
            vertices: [
                [x1 + e, y0 - e, z0],
                [x0 - e, y0 - e, z0],
                [x0 - e, y1 + e, z0],
                [x1 + e, y1 + e, z0],
            ],
            normal: [0.0, 0.0, -1.0],
            texture_rotation: default_rotation,
        },
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
