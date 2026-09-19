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
    tangents: Vec<[f32; 4]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl VoxelMeshBuffer {
    pub(crate) fn push_quad(&mut self, quad: VoxelMeshQuad) {
        let base = self.positions.len() as u32;
        let encoded_tint = encode_tint_tangent(quad.tint);

        self.positions.extend(quad.vertices);
        self.normals.extend([quad.normal; 4]);
        self.uvs.extend(quad.uvs);
        self.light_uvs.extend(quad.light_uvs);
        self.tangents.extend([encoded_tint; 4]);
        self.colors.extend(quad.colors);
        self.indices
            .extend(quad_triangle_indices(base, quad.flip_diagonal));
    }

    pub(crate) fn into_mesh(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }

        // Chunk remeshes mutate these Mesh assets in place. Keep CPU-side mesh data
        // resident so Assets<Mesh>::get_mut remains valid after render extraction.
        Some(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.light_uvs)
            // Terrain does not use normal maps. Reuse the built-in tangent varying
            // for the per-quad tint. Tint is constant across each quad, so the
            // direction+magnitude encoding cannot distort an interpolated gradient.
            .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, self.tangents)
            // RGB carries block-light color directly so rasterization interpolates
            // colored light linearly. Alpha remains ambient occlusion.
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_indices(Indices::U32(self.indices)),
        )
    }
}

fn encode_tint_tangent(tint: [f32; 3]) -> [f32; 4] {
    let color = Vec3::from_array(tint);
    let magnitude = color.length();
    if magnitude <= f32::EPSILON {
        return [0.0; 4];
    }

    let direction = color / magnitude;
    [direction.x, direction.y, direction.z, magnitude]
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remeshable_meshes_stay_cpu_accessible() {
        let mut buffer = VoxelMeshBuffer::default();
        buffer.push_quad(VoxelMeshQuad {
            vertices: [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            normal: [0.0, 0.0, 1.0],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            light_uvs: [[1.0, 0.0]; 4],
            tint: [1.0; 3],
            colors: [[1.0; 4]; 4],
            flip_diagonal: false,
        });

        let mesh = buffer.into_mesh().expect("quad should produce a mesh");

        assert!(mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD));
        assert!(mesh.asset_usage.contains(RenderAssetUsages::RENDER_WORLD));
    }
}
