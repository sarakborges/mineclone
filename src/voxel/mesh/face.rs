use crate::voxel::texture_rotation::TextureRotation;

const FACE_UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub(super) fn push_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertices: [[f32; 3]; 4],
    normal: [f32; 3],
    texture_rotation: TextureRotation,
    tint: [f32; 3],
    shade: f32,
    skylight: f32,
) {
    let start = positions.len() as u32;
    let light = (shade * skylight).clamp(0.0, 1.0);

    positions.extend(vertices);
    normals.extend([normal; 4]);
    uvs.extend(texture_rotation.rotate_uvs(FACE_UVS));
    colors.extend([[tint[0], tint[1], tint[2], light]; 4]);
    indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
}
