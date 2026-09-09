use crate::voxel::texture_rotation::TextureRotation;

use super::lighting::should_flip_diagonal;

const FACE_UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub(super) fn push_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    light_uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertices: [[f32; 3]; 4],
    normal: [f32; 3],
    texture_rotation: TextureRotation,
    tint: [f32; 3],
    light: [[f32; 2]; 4],
    ambient_occlusion: [f32; 4],
) {
    let start = positions.len() as u32;
    let vertex_colors = ambient_occlusion.map(|ao| [tint[0], tint[1], tint[2], ao]);

    positions.extend(vertices);
    normals.extend([normal; 4]);
    uvs.extend(texture_rotation.rotate_uvs(FACE_UVS));
    light_uvs.extend(light);
    colors.extend(vertex_colors);

    if should_flip_diagonal(ambient_occlusion) {
        indices.extend([start, start + 1, start + 3, start + 1, start + 2, start + 3]);
    } else {
        indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }
}
