use bevy::prelude::*;

use crate::voxel::block_face::BlockFace;

pub(crate) const BLOCK_DISPLAY_FACES: [BlockFace; 3] = [
    BlockFace::Top,
    BlockFace::Front,
    BlockFace::Right,
];

const DISPLAY_LEFT: f32 = 0.10;
const DISPLAY_CENTER_X: f32 = 0.50;
const DISPLAY_TOP_Y: f32 = 0.27;
const DISPLAY_HALF_WIDTH: f32 = 0.40;
const DISPLAY_SLOPE: f32 = 0.20;
const DISPLAY_SIDE_HEIGHT: f32 = 0.50;

#[derive(Clone, Copy)]
struct BlockDisplayFaceGeometry {
    origin: Vec2,
    axis_u: Vec2,
    axis_v: Vec2,
}

impl BlockDisplayFaceGeometry {
    fn points(self) -> [Vec2; 4] {
        [
            self.origin,
            self.origin + self.axis_u,
            self.origin + self.axis_u + self.axis_v,
            self.origin + self.axis_v,
        ]
    }
}

pub(crate) fn block_display_face_points(face: BlockFace) -> [Vec2; 4] {
    block_display_face_points_for_height(face, 1.0)
}

pub(crate) fn block_display_face_points_for_height(
    face: BlockFace,
    height: f32,
) -> [Vec2; 4] {
    block_display_face_geometry(face, height).points()
}

pub(crate) fn block_display_face_basis(face: BlockFace) -> (Vec4, Vec4) {
    block_display_face_basis_for_height(face, 1.0)
}

pub(crate) fn block_display_face_basis_for_height(
    face: BlockFace,
    height: f32,
) -> (Vec4, Vec4) {
    let geometry = block_display_face_geometry(face, height);
    (
        Vec4::new(
            geometry.origin.x,
            geometry.origin.y,
            geometry.axis_u.x,
            geometry.axis_u.y,
        ),
        Vec4::new(geometry.axis_v.x, geometry.axis_v.y, 0.0, 0.0),
    )
}

pub(crate) fn block_display_face_shade(face: BlockFace) -> f32 {
    match face {
        BlockFace::Top => 1.0,
        BlockFace::Front => 0.86,
        BlockFace::Right => 0.74,
        _ => 1.0,
    }
}

fn block_display_face_geometry(face: BlockFace, height: f32) -> BlockDisplayFaceGeometry {
    let height = height.clamp(0.0, 1.0);
    let rising = Vec2::new(DISPLAY_HALF_WIDTH, DISPLAY_SLOPE);
    let falling = Vec2::new(DISPLAY_HALF_WIDTH, -DISPLAY_SLOPE);
    let vertical = Vec2::new(0.0, DISPLAY_SIDE_HEIGHT * height);
    let top_y = DISPLAY_TOP_Y + DISPLAY_SIDE_HEIGHT * (1.0 - height) * 0.5;

    match face {
        BlockFace::Top => BlockDisplayFaceGeometry {
            origin: Vec2::new(DISPLAY_LEFT, top_y),
            axis_u: rising,
            axis_v: falling,
        },
        BlockFace::Front => BlockDisplayFaceGeometry {
            origin: Vec2::new(DISPLAY_LEFT, top_y),
            axis_u: rising,
            axis_v: vertical,
        },
        BlockFace::Right => BlockDisplayFaceGeometry {
            origin: Vec2::new(DISPLAY_CENTER_X, top_y + DISPLAY_SLOPE),
            axis_u: falling,
            axis_v: vertical,
        },
        _ => panic!("{face:?} is not part of the display block model"),
    }
}
