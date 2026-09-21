use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, MeshVertexAttribute, VertexAttributeValues},
    prelude::*,
    render::render_resource::{PrimitiveTopology, VertexFormat},
};

use super::{
    chunk::CHUNK_SIZE,
    meshlet::CHUNK_MESHLET_EDGE,
    quad::quad_triangle_indices,
};

/// Keep Bevy's built-in color attribute ID so the standard material pipeline
/// still defines VERTEX_COLORS, but store voxel lighting in normalized bytes.
pub(crate) const ATTRIBUTE_VOXEL_LIGHT: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color", 5, VertexFormat::Unorm8x4);
pub(crate) const ATTRIBUTE_VOXEL_PAYLOAD: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv_1", 3, VertexFormat::Uint32);

pub(crate) struct VoxelMeshQuad {
    pub(crate) vertices: [[f32; 3]; 4],
    pub(crate) normal: [f32; 3],
    pub(crate) uvs: [[f32; 2]; 4],
    pub(crate) light_uvs: [[f32; 2]; 4],
    pub(crate) tint: [f32; 3],
    pub(crate) colors: [[f32; 4]; 4],
    pub(crate) flip_diagonal: bool,
}

#[derive(Default)]
pub(crate) struct VoxelMeshBuffer {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    payloads: Vec<u32>,
    colors: Vec<[u8; 4]>,
    indices: Vec<u32>,
}

impl VoxelMeshBuffer {
    pub(crate) fn push_quad(&mut self, quad: VoxelMeshQuad) {
        let base = self.positions.len() as u32;

        let packed_tint_normal = encode_tint_and_normal(quad.tint, quad.normal);
        let meshlet_index = quad_meshlet_index(quad.vertices, quad.normal);
        self.positions.extend(quad.vertices);
        self.uvs.extend(std::array::from_fn(|index| {
            encode_material_uv(quad.uvs[index], quad.light_uvs[index][1])
        }));
        self.payloads.extend(std::array::from_fn(|index| {
            encode_vertex_payload(
                packed_tint_normal,
                quad.light_uvs[index][0],
                meshlet_index,
            )
        }));
        self.colors.extend(quad.colors.map(encode_voxel_light));
        self.indices
            .extend(quad_triangle_indices(base, quad.flip_diagonal));
    }

    pub(crate) fn into_mesh(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }

        let indices = compact_indices(self.positions.len(), self.indices);

        Some(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_attribute(
                ATTRIBUTE_VOXEL_PAYLOAD,
                VertexAttributeValues::Uint32(self.payloads),
            )
            // Tangents are omitted: voxel materials do not use normal maps.
            // RGB block light and AO remain independent interpolated channels.
            .with_inserted_attribute(
                ATTRIBUTE_VOXEL_LIGHT,
                VertexAttributeValues::Unorm8x4(self.colors),
            )
            .with_inserted_indices(indices),
        )
    }
}

const MATERIAL_UV_STRIDE: f32 = 16.0;
const _: () = assert!(CHUNK_MESHLET_EDGE < MATERIAL_UV_STRIDE as usize);

fn encode_material_uv(uv: [f32; 2], material_code: f32) -> [f32; 2] {
    [
        uv[0] + material_code.round().max(0.0) * MATERIAL_UV_STRIDE,
        uv[1],
    ]
}

fn encode_voxel_light(color: [f32; 4]) -> [u8; 4] {
    color.map(|channel| {
        (channel.clamp(0.0, 1.0) * 255.0).round() as u8
    })
}

fn encode_tint_and_normal(tint: [f32; 3], normal: [f32; 3]) -> u32 {
    let tint = tint.map(|channel| {
        (channel.clamp(0.0, 1.0) * 127.0).round() as u32
    });
    let normal_code = axis_normal_code(normal);
    let packed =
        tint[0] | (tint[1] << 7) | (tint[2] << 14) | (normal_code << 21);
    debug_assert!(packed <= 0x00ff_ffff);
    packed
}

fn encode_vertex_payload(
    packed_tint_normal: u32,
    sky_light: f32,
    meshlet_index: u32,
) -> u32 {
    let sky = (sky_light.clamp(0.0, 1.0) * 15.0).round() as u32;
    debug_assert!(meshlet_index < 8);
    packed_tint_normal | (sky << 24) | (meshlet_index << 28)
}

fn quad_meshlet_index(vertices: [[f32; 3]; 4], normal: [f32; 3]) -> u32 {
    let mut center = Vec3::ZERO;
    for vertex in vertices {
        center += Vec3::from_array(vertex);
    }
    center *= 0.25;
    let source = (center - Vec3::from_array(normal) * 0.01)
        .floor()
        .as_ivec3();
    debug_assert!(
        source.x >= 0
            && source.y >= 0
            && source.z >= 0
            && source.x < CHUNK_SIZE as i32
            && source.y < CHUNK_SIZE as i32
            && source.z < CHUNK_SIZE as i32,
        "voxel quad source must remain inside its chunk: {source:?}"
    );

    let x = source.x.clamp(0, CHUNK_SIZE as i32 - 1) as usize / CHUNK_MESHLET_EDGE;
    let y = source.y.clamp(0, CHUNK_SIZE as i32 - 1) as usize / CHUNK_MESHLET_EDGE;
    let z = source.z.clamp(0, CHUNK_SIZE as i32 - 1) as usize / CHUNK_MESHLET_EDGE;
    let meshlets_per_axis = CHUNK_SIZE / CHUNK_MESHLET_EDGE;
    (x + z * meshlets_per_axis + y * meshlets_per_axis * meshlets_per_axis) as u32
}

fn axis_normal_code(normal: [f32; 3]) -> u32 {
    match normal {
        [1.0, 0.0, 0.0] => 0,
        [-1.0, 0.0, 0.0] => 1,
        [0.0, 1.0, 0.0] => 2,
        [0.0, -1.0, 0.0] => 3,
        [0.0, 0.0, 1.0] => 4,
        [0.0, 0.0, -1.0] => 5,
        _ => panic!("voxel mesh normal must be axis-aligned: {normal:?}"),
    }
}

fn compact_indices(vertex_count: usize, indices: Vec<u32>) -> Indices {
    if vertex_count <= usize::from(u16::MAX) + 1 {
        Indices::U16(indices.into_iter().map(|index| index as u16).collect())
    } else {
        Indices::U32(indices)
    }
}
