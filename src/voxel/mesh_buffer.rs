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
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl VoxelMeshBuffer {
    pub(crate) fn push_quad(&mut self, quad: VoxelMeshQuad) {
        let base = self.positions.len() as u32;

        self.positions.extend(quad.vertices);
        self.normals.extend([quad.normal; 4]);
        self.uvs.extend(quad.uvs);
        self.colors.extend(std::array::from_fn(|index| {
            encode_vertex_payload(
                quad.tint,
                quad.colors[index],
                quad.light_uvs[index],
            )
        }));
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
            // COLOR packs block-light RGB, biome/layer tint, AO, sky light and
            // the terrain material code. Tangents and UV1 are intentionally
            // omitted: voxel materials do not use normal maps and no secondary
            // texture coordinates are required by the custom shader.
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
            .with_inserted_indices(Indices::U32(self.indices)),
        )
    }
}

fn encode_vertex_payload(
    tint: [f32; 3],
    lighting: [f32; 4],
    light_uv: [f32; 2],
) -> [f32; 4] {
    let block = [
        (lighting[0].clamp(0.0, 1.0) * 15.0).round() as u32,
        (lighting[1].clamp(0.0, 1.0) * 15.0).round() as u32,
        (lighting[2].clamp(0.0, 1.0) * 15.0).round() as u32,
    ];
    let packed_block = block[0] | (block[1] << 4) | (block[2] << 8);

    let tint = tint.map(|channel| {
        (channel.clamp(0.0, 1.0) * 255.0).round() as u32
    });
    let packed_tint = tint[0] | (tint[1] << 8) | (tint[2] << 16);

    let material_code = light_uv[1].round().max(0.0) as u32;
    debug_assert!(material_code < (1 << 20));
    let sky_level =
        (light_uv[0].clamp(0.0, 1.0) * 15.0).round() as u32;
    let packed_sky_material = material_code | (sky_level << 20);

    [
        packed_block as f32,
        packed_tint as f32,
        lighting[3].clamp(0.0, 1.0),
        packed_sky_material as f32,
    ]
}
