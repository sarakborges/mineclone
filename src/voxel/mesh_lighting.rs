use bevy::prelude::*;

use super::{
    block_face::BlockFace, light::VoxelLight, mesh_buffer::VoxelMeshBuffer, world::VoxelWorld,
};

const AO_BRIGHTNESS: [f32; 4] = [1.0, 0.92, 0.84, 0.76];
const PACKED_RGB_MAX: f32 = 16_777_215.0;

pub(super) struct FaceLighting {
    pub(super) sky: [[f32; 3]; 4],
    pub(super) block: [[f32; 3]; 4],
    pub(super) ambient_occlusion: [f32; 4],
}

pub(super) fn face_lighting(world: &VoxelWorld, voxel: IVec3, face: BlockFace) -> FaceLighting {
    let (normal, tangent_a, tangent_b, signs) = face_basis(face);
    let base = voxel + normal;
    let emitted_block = normalize_rgb(world.light_at(voxel).block_rgb());
    let mut sky = [[0.0; 3]; 4];
    let mut block = [[0.0; 3]; 4];
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
        let (sky_levels, block_levels) = average_light_levels(
            world,
            voxel,
            [base, side_a, side_b, corner],
        );
        let normalized_block = normalize_rgb(block_levels);

        sky[index] = normalize_rgb(sky_levels);
        block[index] = [
            normalized_block[0].max(emitted_block[0]),
            normalized_block[1].max(emitted_block[1]),
            normalized_block[2].max(emitted_block[2]),
        ];
        ambient_occlusion[index] = AO_BRIGHTNESS[occlusion];
    }

    FaceLighting {
        sky,
        block,
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
    let packed_tint = pack_rgb(tint);
    let light_uvs = lighting.sky.map(|sky| [pack_rgb(sky), packed_tint]);
    let colors = std::array::from_fn(|index| {
        let block = lighting.block[index];
        [
            block[0],
            block[1],
            block[2],
            lighting.ambient_occlusion[index],
        ]
    });

    buffer.push_quad(
        vertices,
        normal,
        uvs,
        light_uvs,
        colors,
        should_flip_diagonal(lighting.ambient_occlusion),
    );
}

fn should_flip_diagonal(ambient_occlusion: [f32; 4]) -> bool {
    ambient_occlusion[0] + ambient_occlusion[2] > ambient_occlusion[1] + ambient_occlusion[3]
}

fn average_light_levels(
    world: &VoxelWorld,
    source: IVec3,
    samples: [IVec3; 4],
) -> ([f32; 3], [f32; 3]) {
    let mut sky_total = [0.0; 3];
    let mut block_total = [0.0; 3];
    let mut count = 0_u32;

    for position in samples {
        if !world.is_loaded_at(position) || world.is_solid(position) {
            continue;
        }

        let light = world.light_at(position);
        let sky = light.sky_rgb();
        let block = light.block_rgb();
        for channel in 0..3 {
            sky_total[channel] += sky[channel] as f32;
            block_total[channel] += block[channel] as f32;
        }
        count += 1;
    }

    if count == 0 {
        if !world.is_loaded_at(source) {
            return ([0.0; 3], [0.0; 3]);
        }

        let light = world.light_at(source);
        return (
            light.sky_rgb().map(|level| level as f32),
            light.block_rgb().map(|level| level as f32),
        );
    }

    let count = count as f32;
    (
        sky_total.map(|level| level / count),
        block_total.map(|level| level / count),
    )
}

fn normalize_rgb(levels: [u8; 3]) -> [f32; 3] {
    levels.map(|level| normalize_level(level as f32))
}

fn normalize_level(level: f32) -> f32 {
    (level / VoxelLight::MAX_LEVEL as f32).clamp(0.0, 1.0)
}

fn pack_rgb(rgb: [f32; 3]) -> f32 {
    let r = (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u32;
    let g = (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u32;
    let b = (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u32;
    let packed = (r << 16) | (g << 8) | b;

    packed as f32 / PACKED_RGB_MAX
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
    use super::{average_light_levels, should_flip_diagonal};
    use crate::voxel::{chunk::VoxelChunk, light::VoxelLight, world::VoxelWorld};
    use bevy::prelude::*;

    #[test]
    fn chooses_the_lower_error_ao_diagonal() {
        assert!(should_flip_diagonal([1.0, 0.6, 1.0, 0.6]));
        assert!(!should_flip_diagonal([0.6, 1.0, 0.6, 1.0]));
    }

    #[test]
    fn unloaded_face_samples_fall_back_to_source_light() {
        let mut world = VoxelWorld::default();
        let mut chunk = VoxelChunk::empty();
        chunk.set_light(15, 8, 8, VoxelLight::new_colored([12, 12, 12], [8, 2, 1]));
        world.insert_chunk(IVec3::ZERO, chunk);
        let source = IVec3::new(15, 8, 8);
        let base = source + IVec3::X;
        let samples = [base, base + IVec3::Y, base + IVec3::Z, base + IVec3::Y + IVec3::Z];

        assert_eq!(
            average_light_levels(&world, source, samples),
            ([12.0; 3], [8.0, 2.0, 1.0]),
        );
    }
}
