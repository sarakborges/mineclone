use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::voxel::{
    block_face::{BlockFace, BlockFaces},
    quad::{QUAD_TRIANGLE_INDICES, VOXEL_FACE_UVS},
};

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

fn block_display_face_geometry(face: BlockFace) -> BlockDisplayFaceGeometry {
    match face {
        BlockFace::Top => BlockDisplayFaceGeometry {
            origin: Vec2::new(0.10, 0.28),
            axis_u: Vec2::new(0.40, 0.20),
            axis_v: Vec2::new(0.40, -0.20),
        },
        BlockFace::Front => BlockDisplayFaceGeometry {
            origin: Vec2::new(0.10, 0.28),
            axis_u: Vec2::new(0.40, 0.20),
            axis_v: Vec2::new(0.00, 0.42),
        },
        BlockFace::Right => BlockDisplayFaceGeometry {
            origin: Vec2::new(0.50, 0.48),
            axis_u: Vec2::new(0.40, -0.20),
            axis_v: Vec2::new(0.00, 0.42),
        },
        _ => panic!("{face:?} is not part of the display block model"),
    }
}

pub(crate) fn block_display_face_basis(face: BlockFace) -> (Vec4, Vec4) {
    let geometry = block_display_face_geometry(face);
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

#[derive(Resource)]
pub(crate) struct BlockModelMeshes {
    world: BlockFaces<Handle<Mesh>>,
    display_top: Handle<Mesh>,
    display_front: Handle<Mesh>,
    display_right: Handle<Mesh>,
}

impl BlockModelMeshes {
    pub(super) fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            world: BlockFaces::from_fn(|face| meshes.add(block_face_mesh(face))),
            display_top: meshes.add(block_display_face_mesh(BlockFace::Top)),
            display_front: meshes.add(block_display_face_mesh(BlockFace::Front)),
            display_right: meshes.add(block_display_face_mesh(BlockFace::Right)),
        }
    }

    pub(crate) fn world_face(&self, face: BlockFace) -> Handle<Mesh> {
        self.world.get(face).clone()
    }

    pub(crate) fn display_face(&self, face: BlockFace) -> Handle<Mesh> {
        match face {
            BlockFace::Top => self.display_top.clone(),
            BlockFace::Front => self.display_front.clone(),
            BlockFace::Right => self.display_right.clone(),
            _ => panic!("{face:?} is not part of the display block model"),
        }
    }
}

fn display_position(point: Vec2) -> [f32; 3] {
    [(point.x - 0.5) * 2.0, (0.5 - point.y) * 2.0, 0.0]
}

fn block_display_face_mesh(face: BlockFace) -> Mesh {
    let geometry = block_display_face_geometry(face);
    let points = geometry.points();
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        points.into_iter().map(display_position).collect::<Vec<_>>(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec())
    .with_inserted_indices(Indices::U32(QUAD_TRIANGLE_INDICES.to_vec()))
}

pub(crate) fn block_face_mesh(face: BlockFace) -> Mesh {
    let center = Vec3::splat(0.5);
    let vertices = face
        .unit_vertices()
        .map(|vertex| (Vec3::from_array(vertex) - center).to_array());

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![face.normal(); 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, VOXEL_FACE_UVS.to_vec())
    .with_inserted_indices(Indices::U32(QUAD_TRIANGLE_INDICES.to_vec()))
}
