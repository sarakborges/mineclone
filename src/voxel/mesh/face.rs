use crate::voxel::texture_rotation::TextureRotation;

use super::{QUAD_TRIANGLE_INDICES, WORLD_FACE_UVS, lighting::should_flip_diagonal};

pub(super) struct FaceData {
    pub vertices: [[f32; 3]; 4],
    pub normal: [f32; 3],
    pub texture_rotation: TextureRotation,
    pub tint: [f32; 3],
    pub light: [[f32; 2]; 4],
    pub ambient_occlusion: [f32; 4],
}

pub(super) fn push_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    light_uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    face: FaceData,
) {
    let start = positions.len() as u32;
    let vertex_colors = face
        .ambient_occlusion
        .map(|ao| [face.tint[0], face.tint[1], face.tint[2], ao]);

    positions.extend(face.vertices);
    normals.extend([face.normal; 4]);
    uvs.extend(face.texture_rotation.rotate_uvs(WORLD_FACE_UVS));
    light_uvs.extend(face.light);
    colors.extend(vertex_colors);

    if should_flip_diagonal(face.ambient_occlusion) {
        indices.extend([start, start + 1, start + 3, start + 1, start + 2, start + 3]);
    } else {
        indices.extend(QUAD_TRIANGLE_INDICES.map(|index| start + index));
    }
}
