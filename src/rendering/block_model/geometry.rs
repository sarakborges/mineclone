use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::voxel::mesh::{BlockFace, QUAD_TRIANGLE_INDICES, WORLD_FACE_UVS};

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
    world_right: Handle<Mesh>,
    world_left: Handle<Mesh>,
    world_top: Handle<Mesh>,
    world_bottom: Handle<Mesh>,
    world_front: Handle<Mesh>,
    world_back: Handle<Mesh>,
    display_top: Handle<Mesh>,
    display_front: Handle<Mesh>,
    display_right: Handle<Mesh>,
}

impl BlockModelMeshes {
    pub(super) fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            world_right: meshes.add(block_face_mesh(BlockFace::Right)),
            world_left: meshes.add(block_face_mesh(BlockFace::Left)),
            world_top: meshes.add(block_face_mesh(BlockFace::Top)),
            world_bottom: meshes.add(block_face_mesh(BlockFace::Bottom)),
            world_front: meshes.add(block_face_mesh(BlockFace::Front)),
            world_back: meshes.add(block_face_mesh(BlockFace::Back)),
            display_top: meshes.add(block_display_face_mesh(BlockFace::Top)),
            display_front: meshes.add(block_display_face_mesh(BlockFace::Front)),
            display_right: meshes.add(block_display_face_mesh(BlockFace::Right)),
        }
    }

    pub(crate) fn world_face(&self, face: BlockFace) -> Handle<Mesh> {
        match face {
            BlockFace::Right => self.world_right.clone(),
            BlockFace::Left => self.world_left.clone(),
            BlockFace::Top => self.world_top.clone(),
            BlockFace::Bottom => self.world_bottom.clone(),
            BlockFace::Front => self.world_front.clone(),
            BlockFace::Back => self.world_back.clone(),
        }
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
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, WORLD_FACE_UVS.to_vec())
    .with_inserted_indices(Indices::U32(QUAD_TRIANGLE_INDICES.to_vec()))
}
