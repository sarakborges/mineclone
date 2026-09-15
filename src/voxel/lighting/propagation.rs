use bevy::{platform::collections::HashSet, prelude::*};

use crate::content::{
    block::BlockRegistry, fluid::FluidRegistry, secondary_property::SecondaryPropertyRegistry,
};
use crate::voxel::{
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    coordinates::split_world_position,
    fluid::FluidCell,
    light::{BlockLight, VoxelLight},
    neighbors::CARDINAL_NEIGHBORS,
    world::VoxelWorld,
};

use super::{
    context::LightingContext,
    medium::{block_emission_for_cell, light_transmission, medium_dampening_for_cells},
    queue::LightingQueue,
};

const HUE_VECTOR_SCALE: i32 = 1024;
const BUDGET_CHECK_INTERVAL_VOXELS: usize = 64;
const HUE_VECTOR_X: [i32; 32] = [
    1024, 1004, 946, 851, 724, 569, 392, 200, 0, -200, -392, -569, -724, -851, -946, -1004, -1024,
    -1004, -946, -851, -724, -569, -392, -200, 0, 200, 392, 569, 724, 851, 946, 1004,
];
const HUE_VECTOR_Y: [i32; 32] = [
    0, 200, 392, 569, 724, 851, 946, 1004, 1024, 1004, 946, 851, 724, 569, 392, 200, 0, -200, -392,
    -569, -724, -851, -946, -1004, -1024, -1004, -946, -851, -724, -569, -392, -200,
];

#[derive(Clone, Copy)]
pub(super) struct LightingRegistries<'a> {
    blocks: &'a BlockRegistry,
    fluids: &'a FluidRegistry,
    secondary_properties: &'a SecondaryPropertyRegistry,
}

impl<'a> LightingRegistries<'a> {
    pub(super) fn new(
        blocks: &'a BlockRegistry,
        fluids: &'a FluidRegistry,
        secondary_properties: &'a SecondaryPropertyRegistry,
    ) -> Self {
        Self {
            blocks,
            fluids,
            secondary_properties,
        }
    }
}

pub(super) fn relax(
    world: &mut VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    secondary_properties: &SecondaryPropertyRegistry,
    queue: &mut LightingQueue,
) -> HashSet<IVec3> {
    let mut changed_chunks = HashSet::new();
    let mut context = LightingContext::default();
    relax_budgeted(
        world,
        LightingRegistries::new(blocks, fluids, secondary_properties),
        queue,
        &mut context,
        &mut changed_chunks,
        |_| false,
    );
    changed_chunks
}

pub(super) fn relax_budgeted(
    world: &mut VoxelWorld,
    registries: LightingRegistries<'_>,
    queue: &mut LightingQueue,
    context: &mut LightingContext,
    changed_chunks: &mut HashSet<IVec3>,
    mut budget_exhausted: impl FnMut(usize) -> bool,
) {
    changed_chunks.clear();
    context.clear();
    let mut processed = 0;

    loop {
        if processed > 0
            && processed % BUDGET_CHECK_INTERVAL_VOXELS == 0
            && budget_exhausted(processed)
        {
            break;
        }

        let Some(position) = queue.pop() else {
            break;
        };
        processed += 1;

        let (chunk_coord, local_position) = split_world_position(position);
        let Some(chunk) = world.chunk(chunk_coord) else {
            continue;
        };
        let Some((cell, fluid, current)) =
            chunk.sample_local(local_position.x, local_position.y, local_position.z)
        else {
            continue;
        };
        let desired = desired_light(
            world,
            registries,
            position,
            (cell, fluid),
            chunk,
            local_position,
            context,
        );

        if current == desired {
            continue;
        }

        if !world.set_light_at_deferred_mesh_revision(chunk_coord, local_position, desired) {
            continue;
        }
        changed_chunks.insert(chunk_coord);
        queue.enqueue_with_neighbors(position);
    }

    world.commit_deferred_light_mesh_revisions(changed_chunks.iter().copied());
}

fn desired_light(
    world: &VoxelWorld,
    registries: LightingRegistries<'_>,
    position: IVec3,
    medium: (Option<VoxelCell>, Option<FluidCell>),
    chunk: &VoxelChunk,
    local_position: IVec3,
    context: &mut LightingContext,
) -> VoxelLight {
    let (cell, fluid) = medium;
    let dampening = medium_dampening_for_cells(cell, fluid, registries.blocks, registries.fluids);
    let blocks_light = dampening >= VoxelLight::MAX_LEVEL;
    let attenuation = dampening.max(1);
    let transmission = light_transmission(
        world,
        registries.blocks,
        registries.secondary_properties,
        position,
    );
    let emitted = block_emission_for_cell(
        cell,
        registries.blocks,
        registries.secondary_properties,
    );

    if blocks_light {
        return VoxelLight::new_hsi(0, emitted);
    }

    let neighbor_lights = cardinal_neighbor_lights(world, position, chunk, local_position);
    let sky = context
        .direct_sky_light(
            world,
            registries.blocks,
            registries.fluids,
            registries.secondary_properties,
            position,
        )
        .max(filtered_level(
            propagated_neighbor_sky(&neighbor_lights, attenuation),
            transmission,
        ));
    let block = mix_strongest_block_lights([
        emitted,
        propagated_neighbor_block(&neighbor_lights, attenuation),
    ]);

    VoxelLight::new_hsi(sky, block)
}

fn cardinal_neighbor_lights(
    world: &VoxelWorld,
    position: IVec3,
    chunk: &VoxelChunk,
    local_position: IVec3,
) -> [VoxelLight; CARDINAL_NEIGHBORS.len()] {
    CARDINAL_NEIGHBORS.map(|direction| {
        let local_neighbor = local_position + direction;
        if local_neighbor.x >= 0
            && local_neighbor.y >= 0
            && local_neighbor.z >= 0
            && local_neighbor.x < CHUNK_SIZE as i32
            && local_neighbor.y < CHUNK_SIZE as i32
            && local_neighbor.z < CHUNK_SIZE as i32
        {
            chunk.light_at(local_neighbor.x, local_neighbor.y, local_neighbor.z)
        } else {
            world.light_at(position + direction)
        }
    })
}

fn propagated_neighbor_sky(neighbor_lights: &[VoxelLight], attenuation: u8) -> u8 {
    let mut result = 0;

    for light in neighbor_lights {
        result = result.max(light.sky().saturating_sub(attenuation));
    }

    result
}

fn propagated_neighbor_block(neighbor_lights: &[VoxelLight], attenuation: u8) -> BlockLight {
    let mut incoming = [BlockLight::DARK; CARDINAL_NEIGHBORS.len()];

    for (index, light) in neighbor_lights.iter().enumerate() {
        incoming[index] = light.block_hsi().attenuated(attenuation);
    }

    mix_strongest_block_lights(incoming)
}

fn mix_strongest_block_lights<const N: usize>(lights: [BlockLight; N]) -> BlockLight {
    let mut strongest_intensity = 0;
    for light in &lights {
        strongest_intensity = strongest_intensity.max(light.intensity());
    }

    if strongest_intensity == 0 {
        return BlockLight::DARK;
    }

    let mut vector_x = 0_i32;
    let mut vector_y = 0_i32;
    let mut saturation_sum = 0_u32;
    let mut strongest_count = 0_u32;

    for light in &lights {
        if light.intensity() != strongest_intensity {
            continue;
        }

        strongest_count += 1;
        let saturation = light.saturation() as i32;
        saturation_sum += saturation as u32;
        if saturation == 0 {
            continue;
        }

        let hue = light.hue() as usize;
        vector_x += HUE_VECTOR_X[hue] * saturation;
        vector_y += HUE_VECTOR_Y[hue] * saturation;
    }

    if saturation_sum == 0 {
        return BlockLight::new(0, 0, strongest_intensity);
    }

    let vector_length =
        ((vector_x as f32 * vector_x as f32) + (vector_y as f32 * vector_y as f32)).sqrt();
    let maximum_length = HUE_VECTOR_SCALE as f32 * saturation_sum as f32;
    let coherence = (vector_length / maximum_length).clamp(0.0, 1.0);
    let average_saturation = saturation_sum as f32 / strongest_count as f32;
    let saturation = (average_saturation * coherence.sqrt())
        .round()
        .clamp(0.0, BlockLight::MAX_SATURATION as f32) as u8;

    if saturation == 0 {
        return BlockLight::new(0, 0, strongest_intensity);
    }

    let hue = nearest_hue(vector_x, vector_y);
    BlockLight::new(hue, saturation, strongest_intensity)
}

fn nearest_hue(vector_x: i32, vector_y: i32) -> u8 {
    let mut best_hue = 0_u8;
    let mut best_dot = i64::MIN;

    for hue in 0..32_usize {
        let dot =
            vector_x as i64 * HUE_VECTOR_X[hue] as i64 + vector_y as i64 * HUE_VECTOR_Y[hue] as i64;
        if dot > best_dot {
            best_dot = dot;
            best_hue = hue as u8;
        }
    }

    best_hue
}

fn filtered_level(level: u8, factor: f32) -> u8 {
    (level as f32 * factor.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, VoxelLight::MAX_LEVEL as f32) as u8
}

#[cfg(test)]
mod tests {
    use super::mix_strongest_block_lights;
    use crate::{content::color::Hsi, voxel::light::BlockLight};

    #[test]
    fn colored_attenuation_preserves_hue_and_saturation() {
        let source = BlockLight::from_hsi(Hsi::new(5.0, 0.9, 0.35), 15);
        let faded = source.attenuated(4);

        assert_eq!(faded.hue(), source.hue());
        assert_eq!(faded.saturation(), source.saturation());
        assert_eq!(faded.intensity(), 11);
    }

    #[test]
    fn equal_red_and_blue_mix_toward_magenta() {
        let red = BlockLight::from_hsi(Hsi::new(0.0, 1.0, 1.0 / 3.0), 15);
        let blue = BlockLight::from_hsi(Hsi::new(240.0, 1.0, 1.0 / 3.0), 15);
        let mixed = mix_strongest_block_lights([red, blue]);
        let color = mixed.color();

        assert_eq!(mixed.intensity(), 15);
        assert!(
            color.hue > 270.0 && color.hue < 330.0,
            "hue should be magenta: {color:?}"
        );
    }

    #[test]
    fn stronger_color_wins_before_hue_blending() {
        let red = BlockLight::from_hsi(Hsi::new(0.0, 1.0, 1.0 / 3.0), 15);
        let blue = BlockLight::from_hsi(Hsi::new(240.0, 1.0, 1.0 / 3.0), 14);
        let mixed = mix_strongest_block_lights([red, blue]);

        assert_eq!(mixed, red);
    }

    #[test]
    fn opposing_equal_hues_desaturate() {
        let red = BlockLight::from_hsi(Hsi::new(0.0, 1.0, 1.0 / 3.0), 15);
        let cyan = BlockLight::from_hsi(Hsi::new(180.0, 1.0, 2.0 / 3.0), 15);
        let mixed = mix_strongest_block_lights([red, cyan]);

        assert_eq!(mixed.intensity(), 15);
        assert_eq!(mixed.saturation(), 0);
    }
}
