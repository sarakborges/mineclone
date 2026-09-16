pub(crate) const VOXEL_FACE_UVS: [[f32; 2]; 4] =
    [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

pub(crate) const QUAD_TRIANGLE_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];
const FLIPPED_QUAD_TRIANGLE_INDICES: [u32; 6] = [0, 1, 3, 1, 2, 3];

pub(crate) fn quad_triangle_indices(base: u32, flipped: bool) -> [u32; 6] {
    let indices = if flipped {
        FLIPPED_QUAD_TRIANGLE_INDICES
    } else {
        QUAD_TRIANGLE_INDICES
    };

    indices.map(|index| base + index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_indices_preserve_vertex_offset() {
        assert_eq!(quad_triangle_indices(4, false), [4, 5, 6, 4, 6, 7]);
        assert_eq!(quad_triangle_indices(4, true), [4, 5, 7, 5, 6, 7]);
    }
}
