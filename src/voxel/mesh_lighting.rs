use bevy::prelude::*;

use super::{
    block_face::BlockFace, light::VoxelLight, mesh_buffer::VoxelMeshBuffer, world::VoxelWorld,
};

const AO_BRIGHTNESS: [f32; 4] = [1.0, 0.86, 0.72, 0.58];

pub(super) struct FaceLighting {
    pub(super) channels: [[f32; 2]; 4],
    pub(super) ambient_occlusion: [f32; 4],
}

pub(super) fn face_lighting(
    world: &VoxelWorld,
    voxel: IVec3,
    face: BlockFace,
    _neutralize_emissive_surface_light: bool,
) -> FaceLighting {
    let (normal, tangent_a, tangent_b, signs) = face_basis(face);
    let base = voxel + normal;
    let emitted_block_level = world.light_at(voxel).block() as f32;
    let mut channels = [[0.0; 2]; 4];
    let mut ambient_occlusion = [1.0; 4];

    for (index, (sign_a, sign_b)) in signs.into_iter().enumerate() {
        let offset_a = tangent_a * sign_a;
        let offset_b = tangent_b * sign_b;
        let side_a = base + offset_a;
        let side_b = base + offset_b;
        let corner = base + offset_a + offset_b;
        let side_a_solid = world.is_solid(side_a);
        let side_b_solid = world.is_solid(side_b);
        let corner_solid = world.is_solid(corner);
        let occlusion = if side_a_solid && side_b_solid {
            3
        } else {
            side_a_solid as usize + side_b_solid as usize + corner_solid as usize
        };
        let (sky_level, block_level) =
            average_light_levels(world, [base, side_a, side_b, corner]);

        channels[index] = [
            normalize_level(sky_level),
            normalize_level(block_level.max(emitted_block_level)),
        ];
        ambient_occlusion[index] = AO_BRIGHTNESS[occlusion];
    }

    FaceLighting {
        channels,
        ambient_occlusion,
    }
}

pub(super) fn push_lit_quad(
    buffer: &mut VoxelMeshBuffer,
    vertices: [[f32; 3]; 4],
    normal: [f32; 3],
    uvs: [[f32; 2]; 4],
    tint: [f32; 3],
    lighting: FaceLighting,
) {
    let colors = std::array::from_fn(|index| {
        [
            tint[0],
            tint[1],
            tint[2],
            lighting.ambient_occlusion[index],
        ]
    });

    buffer.push_quad(
        vertices,
        normal,
        uvs,
        lighting.channels,
        colors,
        should_flip_diagonal(lighting.ambient_occlusion),
    );
}

fn should_flip_diagonal(ambient_occlusion: [f32; 4]) -> bool {
    ambient_occlusion[0] + ambient_occlusion[2] > ambient_occlusion[1] + ambient_occlusion[3]
}

fn average_light_levels(world: &VoxelWorld, samples: [IVec3; 4]) -> (f32, f32) {
    let mut sky_total = 0.0;
    let mut block_total = 0.0;
    let mut count = 0_u32;

    for position in samples {
        if !world.is_loaded_at(position) || world.is_solid(position) {
            continue;
        }

        let light = world.light_at(position);
        sky_total += light.sky() as f32;
        block_total += light.block() as f32;
        count += 1;
    }

    if count == 0 {
        (VoxelLight::MAX_LEVEL as f32, 0.0)
    } else {
        (sky_total / count as f32, block_total / count as f32)
    }
}

fn normalize_level(level: f32) -> f32 {
    (level / VoxelLight::MAX_LEVEL as f32).clamp(0.0, 1.0)
}

fn face_basis(face: BlockFace) -> (IVec3, IVec3, IVec3, [(i32, i32); 4]) {
    match face {
        BlockFace::Right => (
            IVec3::X,
            IVec3::Y,
            IVec3::Z,
            [(-1, 1), (-1, -1), (1, -1), (1, 1)],
        ),
        BlockFace::Left => (
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::Z,
            [(-1, -1), (-1, 1), (1, 1), (1, -1)],
        ),
        BlockFace::Top => (
            IVec3::Y,
            IVec3::X,
            IVec3::Z,
            [(-1, 1), (1, 1), (1, -1), (-1, -1)],
        ),
        BlockFace::Bottom => (
            IVec3::NEG_Y,
            IVec3::X,
            IVec3::Z,
            [(-1, -1), (1, -1), (1, 1), (-1, 1)],
        ),
        BlockFace::Front => (
            IVec3::Z,
            IVec3::X,
            IVec3::Y,
            [(-1, -1), (1, -1), (1, 1), (-1, 1)],
        ),
        BlockFace::Back => (
            IVec3::NEG_Z,
            IVec3::X,
            IVec3::Y,
            [(1, -1), (-1, -1), (-1, 1), (1, 1)],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::should_flip_diagonal;

    #[test]
    fn chooses_the_lower_error_ao_diagonal() {
        assert!(should_flip_diagonal([1.0, 0.6, 1.0, 0.6]));
        assert!(!should_flip_diagonal([0.6, 1.0, 0.6, 1.0]));
    }
}
