use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::{
    content::{block::BlockDefinition, block_shape::{STACKABLE_LAYER_HEIGHT, is_stackable_layer}},
    rendering::block_display::{block_display_face_points, block_display_face_points_for_height},
    voxel::{
        block_face::{BlockFace, BlockFaces},
        quad::{QUAD_TRIANGLE_INDICES, VOXEL_FACE_UVS},
    },
};

#[derive(Resource)]
pub(crate) struct BlockModelMeshes {
    world: BlockFaces<Handle<Mesh>>,
    layer_world: BlockFaces<Handle<Mesh>>,
    hollow_world: BlockFaces<Handle<Mesh>>,
    display_top: Handle<Mesh>,
    display_front: Handle<Mesh>,
    display_right: Handle<Mesh>,
    layer_display_top: Handle<Mesh>,
    layer_display_front: Handle<Mesh>,
    layer_display_right: Handle<Mesh>,
}

impl BlockModelMeshes {
    pub(super) fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            world: BlockFaces::from_fn(|face| meshes.add(block_face_mesh(face))),
            layer_world: BlockFaces::from_fn(|face| {
                meshes.add(block_face_mesh_for_height(face, STACKABLE_LAYER_HEIGHT))
            }),
            hollow_world: BlockFaces::from_fn(|face| meshes.add(hollow_log_face_mesh(face))),
            display_top: meshes.add(block_display_face_mesh(BlockFace::Top)),
            display_front: meshes.add(block_display_face_mesh(BlockFace::Front)),
            display_right: meshes.add(block_display_face_mesh(BlockFace::Right)),
            layer_display_top: meshes.add(block_display_face_mesh_for_height(
                BlockFace::Top,
                STACKABLE_LAYER_HEIGHT,
            )),
            layer_display_front: meshes.add(block_display_face_mesh_for_height(
                BlockFace::Front,
                STACKABLE_LAYER_HEIGHT,
            )),
            layer_display_right: meshes.add(block_display_face_mesh_for_height(
                BlockFace::Right,
                STACKABLE_LAYER_HEIGHT,
            )),
        }
    }

    pub(crate) fn world_face(&self, face: BlockFace) -> Handle<Mesh> {
        self.world.get(face).clone()
    }

    pub(crate) fn world_face_for_block(
        &self,
        face: BlockFace,
        block: &BlockDefinition,
    ) -> Handle<Mesh> {
        if is_stackable_layer(block) {
            self.layer_world.get(face).clone()
        } else {
            self.world_face(face)
        }
    }

    pub(crate) fn hollow_world_face(&self, face: BlockFace) -> Handle<Mesh> {
        self.hollow_world.get(face).clone()
    }

    pub(crate) fn display_face(&self, face: BlockFace) -> Handle<Mesh> {
        display_face_handle(face, &self.display_top, &self.display_front, &self.display_right)
    }

    pub(crate) fn display_face_for_block(
        &self,
        face: BlockFace,
        block: &BlockDefinition,
    ) -> Handle<Mesh> {
        if is_stackable_layer(block) {
            display_face_handle(
                face,
                &self.layer_display_top,
                &self.layer_display_front,
                &self.layer_display_right,
            )
        } else {
            self.display_face(face)
        }
    }

    pub(crate) fn face_for_mesh(&self, mesh: &Handle<Mesh>) -> Option<BlockFace> {
        for face in BlockFace::ALL {
            if self.world.get(face) == mesh
                || self.layer_world.get(face) == mesh
                || self.hollow_world.get(face) == mesh
            {
                return Some(face);
            }
        }

        for face in [BlockFace::Top, BlockFace::Front, BlockFace::Right] {
            if self.display_face(face) == *mesh
                || display_face_handle(
                    face,
                    &self.layer_display_top,
                    &self.layer_display_front,
                    &self.layer_display_right,
                ) == *mesh
            {
                return Some(face);
            }
        }
        None
    }
}

fn display_face_handle(
    face: BlockFace,
    top: &Handle<Mesh>,
    front: &Handle<Mesh>,
    right: &Handle<Mesh>,
) -> Handle<Mesh> {
    match face {
        BlockFace::Top => top.clone(),
        BlockFace::Front => front.clone(),
        BlockFace::Right => right.clone(),
        _ => panic!("{face:?} is not part of the display block model"),
    }
}

fn display_position(point: Vec2) -> [f32; 3] {
    [(point.x - 0.5) * 2.0, (0.5 - point.y) * 2.0, 0.0]
}

fn block_display_face_mesh(face: BlockFace) -> Mesh {
    let points = block_display_face_points(face);
    block_display_mesh_from_points(points)
}

fn block_display_face_mesh_for_height(face: BlockFace, height: f32) -> Mesh {
    let points = block_display_face_points_for_height(face, height);
    block_display_mesh_from_points(points)
}

fn block_display_mesh_from_points(points: [Vec2; 4]) -> Mesh {
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
    block_face_mesh_for_height(face, 1.0)
}

fn block_face_mesh_for_height(face: BlockFace, height: f32) -> Mesh {
    let center = Vec3::splat(0.5);
    let height = height.clamp(0.0, 1.0);
    let vertices = face.unit_vertices().map(|vertex| {
        let mut centered = Vec3::from_array(vertex) - center;
        centered.y *= height;
        centered.to_array()
    });

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![face.normal(); 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, VOXEL_FACE_UVS.to_vec())
    .with_inserted_indices(Indices::U32(QUAD_TRIANGLE_INDICES.to_vec()))
}

const HOLLOW_PREVIEW_WALL: f32 = 1.0 / 16.0;

fn hollow_log_face_mesh(face: BlockFace) -> Mesh {
    let inner = 0.5 - HOLLOW_PREVIEW_WALL;
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<u32>::new();

    match face {
        BlockFace::Right | BlockFace::Left | BlockFace::Front | BlockFace::Back => {
            let outer = face
                .unit_vertices()
                .map(|vertex| (Vec3::from_array(vertex) - Vec3::splat(0.5)).to_array());
            push_preview_quad(&mut positions, &mut normals, &mut uvs, &mut indices, outer, face.normal());

            let mut inner_quad = outer;
            let (normal_axis, radial_axis) = match face {
                BlockFace::Right | BlockFace::Left => (0, 2),
                BlockFace::Front | BlockFace::Back => (2, 0),
                _ => unreachable!(),
            };
            for vertex in &mut inner_quad {
                vertex[normal_axis] = vertex[normal_axis].signum() * inner;
                vertex[radial_axis] = vertex[radial_axis].signum() * inner;
            }
            push_preview_quad(&mut positions, &mut normals, &mut uvs, &mut indices, inner_quad, face.normal());
        }
        BlockFace::Top | BlockFace::Bottom => {
            let y = if face == BlockFace::Top { 0.5 } else { -0.5 };
            for (x0, x1, z0, z1) in [
                (-0.5, 0.5, inner, 0.5),
                (-0.5, 0.5, -0.5, -inner),
                (-0.5, -inner, -inner, inner),
                (inner, 0.5, -inner, inner),
            ] {
                push_preview_quad(
                    &mut positions, &mut normals, &mut uvs, &mut indices,
                    [[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]],
                    face.normal(),
                );
            }
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

fn push_preview_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    vertices: [[f32; 3]; 4],
    normal: [f32; 3],
) {
    let base = positions.len() as u32;
    positions.extend(vertices);
    normals.extend([normal; 4]);
    uvs.extend(VOXEL_FACE_UVS);
    indices.extend(QUAD_TRIANGLE_INDICES.map(|index| base + index));
}
