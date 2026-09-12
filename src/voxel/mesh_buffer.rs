use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use super::quad::quad_triangle_indices;

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
    pub(crate) fn push_quad(
        &mut self,
        vertices: [[f32; 3]; 4],
        normal: [f32; 3],
        uvs: [[f32; 2]; 4],
        light_uvs: [[f32; 2]; 4],
        colors: [[f32; 4]; 4],
        flip_diagonal: bool,
    ) {
        let base = self.positions.len() as u32;

        self.positions.extend(vertices);
        self.normals.extend([normal; 4]);
        self.uvs.extend(uvs);
        self.light_uvs.extend(light_uvs);
        self.colors.extend(colors);
        self.indices
            .extend(quad_triangle_indices(base, flip_diagonal));
    }

    pub(crate) fn into_mesh(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }

        Some(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.light_uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_indices(Indices::U32(self.indices)),
        )
    }
}
