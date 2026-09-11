use crate::voxel::{
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{FaceLighting, should_flip_diagonal},
    quad::VOXEL_FACE_UVS,
    texture_rotation::TextureRotation,
};

#[derive(Default)]
pub(super) struct MeshBuffers {
    inner: VoxelMeshBuffer,
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
        let colors = lighting
            .ambient_occlusion
            .map(|ao| [tint[0], tint[1], tint[2], ao]);

        self.inner.push_quad(
            vertices,
            normal,
            texture_rotation.rotate_uvs(VOXEL_FACE_UVS),
            lighting.channels,
            colors,
            should_flip_diagonal(lighting.ambient_occlusion),
        );
    }

    pub fn into_mesh(self) -> Option<bevy::prelude::Mesh> {
        self.inner.into_mesh()
    }
}
