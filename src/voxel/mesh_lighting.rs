use bevy::prelude::*;

use super::{
    block_face::BlockFace,
    cell::VoxelCell,
    fluid::FluidCell,
    light::{BlockLight, VoxelLight},
    mesh_buffer::{VoxelMeshBuffer, VoxelMeshQuad},
    read::VoxelRead,
};

const AO_BRIGHTNESS: [f32; 4] = [1.0, 0.86, 0.72, 0.58];
const AO_DIAGONAL_EPSILON: f32 = 0.001;

type VoxelSample = Option<(Option<VoxelCell>, Option<FluidCell>, VoxelLight)>;

pub(super) struct FaceLighting {
    pub(super) channels: [[f32; 2]; 4],
    pub(super) block_srgb: [[f32; 3]; 4],
    pub(super) ambient_occlusion: [f32; 4],
}

pub(super) fn surface_block_srgb(
    light: VoxelLight,
    neutralize_emissive_surface_light: bool,
) -> [f32; 3] {
    let block_srgb = light.block_srgb_levels().map(|level| level as f32);
    if neutralize_emissive_surface_light {
        [max_component(block_srgb); 3]
    } else {
        block_srgb
    }
}

pub(super) fn face_lighting<W: VoxelRead + ?Sized>(
    world: &W,
    voxel: IVec3,
    face: BlockFace,
    surface_block_srgb: [f32; 3],
) -> FaceLighting {
    let (normal, tangent_a, tangent_b, signs) = face_basis(face);
    let base = voxel + normal;
    let base_sample = provisional_top_sky_sample(face, world.sample_at(base));
    let side_a_samples = [
        world.sample_at(base - tangent_a),
        world.sample_at(base + tangent_a),
    ];
    let side_b_samples = [
        world.sample_at(base - tangent_b),
        world.sample_at(base + tangent_b),
    ];
    let corner_samples = [
        [
            world.sample_at(base - tangent_a - tangent_b),
            world.sample_at(base - tangent_a + tangent_b),
        ],
        [
            world.sample_at(base + tangent_a - tangent_b),
            world.sample_at(base + tangent_a + tangent_b),
        ],
    ];
    let mut channels = [[0.0; 2]; 4];
    let mut block_srgb = [[0.0; 3]; 4];
    let mut ambient_occlusion = [1.0; 4];

    for (index, (sign_a, sign_b)) in signs.into_iter().enumerate() {
        let side_a_index = sign_index(sign_a);
        let side_b_index = sign_index(sign_b);
        let side_a_sample = side_a_samples[side_a_index];
        let side_b_sample = side_b_samples[side_b_index];
        let corner_sample = corner_samples[side_a_index][side_b_index];
        let side_a_occlusion = sample_occlusion(side_a_sample);
        let side_b_occlusion = sample_occlusion(side_b_sample);
        let corner_occlusion = sample_occlusion(corner_sample);
        let occlusion = if side_a_occlusion >= 1.0 && side_b_occlusion >= 1.0 {
            3.0
        } else {
            (side_a_occlusion + side_b_occlusion + corner_occlusion).min(3.0)
        };
        let (sky_level, sampled_block_srgb) = average_shader_light_levels([
            base_sample,
            side_a_sample,
            side_b_sample,
            corner_sample,
        ]);
        let sampled_block_srgb = component_max(sampled_block_srgb, surface_block_srgb);

        channels[index] = [
            normalize_level(sky_level),
            normalize_level(max_component(sampled_block_srgb)),
        ];
        block_srgb[index] = sampled_block_srgb.map(normalize_level);
        ambient_occlusion[index] = ao_brightness(occlusion);
    }

    FaceLighting {
        channels,
        block_srgb,
        ambient_occlusion,
    }
}

// A top face is provisionally exposed when the chunk above has not loaded.
// Absence is not measured darkness: sample the open sky until a real neighbor
// arrives and the existing halo remesh replaces the provisional vertex data.
// Do not brighten lateral faces or replace loaded cave measurements.
fn provisional_top_sky_sample(face: BlockFace, sample: VoxelSample) -> VoxelSample {
    if face == BlockFace::Top && sample.is_none() {
        Some((
            None,
            None,
            VoxelLight::new_hsi(VoxelLight::MAX_LEVEL, BlockLight::DARK),
        ))
    } else {
        sample
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
            lighting.block_srgb[index][0],
            lighting.block_srgb[index][1],
            lighting.block_srgb[index][2],
            lighting.ambient_occlusion[index],
        ]
    });
    let flip_diagonal = should_flip_diagonal(lighting.ambient_occlusion, lighting.block_srgb);

    buffer.push_quad(VoxelMeshQuad {
        vertices,
        normal,
        uvs,
        light_uvs: lighting.channels,
        tint,
        colors,
        flip_diagonal,
    });
}

fn should_flip_diagonal(ambient_occlusion: [f32; 4], block_srgb: [[f32; 3]; 4]) -> bool {
    let ao_balance =
        ambient_occlusion[0] + ambient_occlusion[2] - ambient_occlusion[1] - ambient_occlusion[3];

    if ao_balance.abs() > AO_DIAGONAL_EPSILON {
        return ao_balance > 0.0;
    }

    let diagonal_02 = srgb_distance_squared(block_srgb[0], block_srgb[2]);
    let diagonal_13 = srgb_distance_squared(block_srgb[1], block_srgb[3]);
    diagonal_13 < diagonal_02
}

fn srgb_distance_squared(left: [f32; 3], right: [f32; 3]) -> f32 {
    let red = left[0] - right[0];
    let green = left[1] - right[1];
    let blue = left[2] - right[2];
    red * red + green * green + blue * blue
}

fn sign_index(sign: i32) -> usize {
    if sign < 0 { 0 } else { 1 }
}

fn sample_occlusion(sample: VoxelSample) -> f32 {
    sample.map_or(0.0, |(cell, _, _)| {
        cell.map_or(0.0, |cell| {
            crate::voxel::microblock::MicroblockMask::from_cell(cell).occupied_fraction()
        })
    })
}

fn ao_brightness(occlusion: f32) -> f32 {
    let clamped = occlusion.clamp(0.0, 3.0);
    let lower = clamped.floor() as usize;
    if lower >= 3 {
        return AO_BRIGHTNESS[3];
    }
    let fraction = clamped - lower as f32;
    AO_BRIGHTNESS[lower] * (1.0 - fraction) + AO_BRIGHTNESS[lower + 1] * fraction
}

fn average_shader_light_levels(samples: [VoxelSample; 4]) -> (f32, [f32; 3]) {
    let mut sky_total = 0.0;
    let mut block_total = [0.0; 3];
    let mut weight_total = 0.0;

    for sample in samples {
        let Some((cell, _, light)) = sample else {
            continue;
        };
        let weight = sample_open_fraction(cell);
        if weight <= f32::EPSILON {
            continue;
        }

        sky_total += light.sky() as f32 * weight;
        let block = light.block_srgb_levels();
        block_total[0] += block[0] as f32 * weight;
        block_total[1] += block[1] as f32 * weight;
        block_total[2] += block[2] as f32 * weight;
        weight_total += weight;
    }

    if weight_total <= f32::EPSILON {
        (0.0, [0.0; 3])
    } else {
        (
            sky_total / weight_total,
            [
                block_total[0] / weight_total,
                block_total[1] / weight_total,
                block_total[2] / weight_total,
            ],
        )
    }
}

fn sample_open_fraction(cell: Option<VoxelCell>) -> f32 {
    cell.map_or(1.0, |cell| {
        1.0 - crate::voxel::microblock::MicroblockMask::from_cell(cell).occupied_fraction()
    })
}

fn component_max(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[0].max(right[0]),
        left[1].max(right[1]),
        left[2].max(right[2]),
    ]
}

fn max_component(value: [f32; 3]) -> f32 {
    value[0].max(value[1]).max(value[2])
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
    use super::{ao_brightness, provisional_top_sky_sample, sample_occlusion, sample_open_fraction, should_flip_diagonal};
    use crate::voxel::{block_face::BlockFace, cell::VoxelCell, light::VoxelLight, microblock::MicroblockMask};

    const DARK: [f32; 3] = [0.0; 3];

    #[test]
    fn partial_microblocks_reduce_ambient_occlusion() {
        assert_eq!(ao_brightness(0.0), 1.0);
        assert_eq!(ao_brightness(3.0), 0.58);
        assert!(ao_brightness(1.5) > 0.72);
        assert!(sample_occlusion(Some((Some(VoxelCell::new("stone", Default::default())), None, VoxelLight::DARK))) > 0.0);
        let empty = MicroblockMask::EMPTY.apply_to_cell(VoxelCell::new("stone", Default::default()), true);
        assert_eq!(sample_occlusion(Some((Some(empty), None, VoxelLight::DARK))), 0.0);
    }

    #[test]
    fn partial_microblocks_contribute_open_light_fraction() {
        let full = VoxelCell::new("stone", Default::default());
        let empty = MicroblockMask::EMPTY.apply_to_cell(full, true);
        assert_eq!(sample_open_fraction(None), 1.0);
        assert_eq!(sample_open_fraction(Some(full)), 0.0);
        let mut half_mask = MicroblockMask::EMPTY;
        for layer in 0..4 {
            for y in 0..8 {
                for x in 0..8 {
                    half_mask.edit([x, y, layer], crate::voxel::microblock::ChiselResolution::ExtraThin, true);
                }
            }
        }
        let half = half_mask.apply_to_cell(full, true);
        assert!((sample_open_fraction(Some(half)) - 0.5).abs() < f32::EPSILON);
        assert_eq!(sample_open_fraction(Some(empty)), 1.0);
    }

    #[test]
    fn chooses_the_lower_error_ao_diagonal() {
        assert!(should_flip_diagonal([1.0, 0.6, 1.0, 0.6], [DARK; 4],));
        assert!(!should_flip_diagonal([0.6, 1.0, 0.6, 1.0], [DARK; 4],));
    }

    #[test]
    fn uses_shader_color_similarity_when_ao_is_tied() {
        let red = [1.0, 0.0, 0.0];
        let blue = [0.0, 0.0, 1.0];
        let purple = [0.5, 0.0, 0.5];

        assert!(should_flip_diagonal([1.0; 4], [red, purple, blue, purple],));
        assert!(!should_flip_diagonal([1.0; 4], [purple, red, purple, blue],));
    }

    #[test]
    fn only_unloaded_top_faces_receive_provisional_sky() {
        let sky = provisional_top_sky_sample(BlockFace::Top, None).unwrap();
        assert_eq!(sky.0, None);
        assert_eq!(sky.1, None);
        assert_eq!(sky.2.sky(), VoxelLight::MAX_LEVEL);
        assert_eq!(sky.2.block(), 0);
        assert!(provisional_top_sky_sample(BlockFace::Front, None).is_none());
        assert!(provisional_top_sky_sample(BlockFace::Bottom, None).is_none());

        let measured_dark = Some((None, None, VoxelLight::DARK));
        assert_eq!(
            provisional_top_sky_sample(BlockFace::Top, measured_dark),
            measured_dark
        );
    }
}
