use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use super::{face::push_face, lighting::FaceLighting};
use crate::voxel::texture_rotation::TextureRotation;

#[derive(Default)]
pub(super) struct MeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    light_uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuffers {
    pub fn push(
        &mut self,
        vertices: [[f32; 3]; 4],
        normal: [f32; 3],
        texture_rotation: TextureRotation,
        tint: [f32; 3],
        lighting: FaceLighting,
    ) {
        push_face(
            &mut self.positions,
            &mut self.normals,
            &mut self.uvs,
            &mut self.light_uvs,
            &mut self.colors,
            &mut self.indices,
            vertices,
            normal,
            texture_rotation,
            tint,
            lighting.channels,
            lighting.ambient_occlusion,
        );
    }

    pub fn into_mesh(self) -> Option<Mesh> {
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
