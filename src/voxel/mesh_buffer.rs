use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use super::quad::quad_triangle_indices;

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
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    light_uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl VoxelMeshBuffer {
    pub(crate) fn push_quad(&mut self, quad: VoxelMeshQuad) {
        let base = self.positions.len() as u32;

        self.positions.extend(quad.vertices);
        self.normals.extend([quad.normal; 4]);
        self.uvs.extend(quad.uvs);
        self.light_uvs.extend(quad.light_uvs);
        self.colors.extend(
            quad.colors
                .map(|color| encode_vertex_payload(quad.tint, color)),
        );
        self.indices
            .extend(quad_triangle_indices(base, quad.flip_diagonal));
    }

    pub(crate) fn into_mesh(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }

        Some(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.light_uvs)
            // COLOR packs block-light RGB, biome/layer tint and AO. Tangents are
            // intentionally omitted: voxel materials do not use normal maps.
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_indices(Indices::U32(self.indices)),
        )
    }
}

fn encode_vertex_payload(tint: [f32; 3], lighting: [f32; 4]) -> [f32; 4] {
    let block = lighting[..3].map(|channel| {
        (channel.clamp(0.0, 1.0) * 15.0).round() as u32
    });
    let packed_block = block[0] | (block[1] << 4) | (block[2] << 8);

    let tint = tint.map(|channel| {
        (channel.clamp(0.0, 1.0) * 255.0).round() as u32
    });
    let packed_tint = tint[0] | (tint[1] << 8) | (tint[2] << 16);

    [
        packed_block as f32,
        packed_tint as f32,
        lighting[3].clamp(0.0, 1.0),
        1.0,
    ]
}
