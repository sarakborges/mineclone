use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        biome_density::BiomeDensityModifier,
        block::BlockRegistry,
        dimension::DimensionDefinition,
        fluid::{FluidId, FluidRegistry},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{VoxelChunk, CHUNK_SIZE},
        fluid::FluidCell,
        texture_rotation::TextureRotation,
    },
};

use super::{
    biome_field::{BiomeField, BiomeFieldSample},
    terrain::{chunk_y_bounds, surface_height_from_sample, terrain_density},
};

const GRASS_BLOCK_ID: &str = "mineclone:grass";
const DENSITY_NOISE_EDGE: f32 = 0.15;

struct GenerationColumnSample {
    surface_height: i32,
    surface_fluid: Option<FluidId>,
}

pub(crate) fn generate_chunk(
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> VoxelChunk {
    if coord.y < 0 {
        return VoxelChunk::empty();
    }

    let (_, maximum_surface_chunk_y) = chunk_y_bounds(dimension, biomes);

    if coord.y > maximum_surface_chunk_y && !biomes.has_volume_density_modifiers() {
        return VoxelChunk::empty();
    }

    let grass = blocks
        .get(GRASS_BLOCK_ID)
        .unwrap_or_else(|| panic!("missing block definition: {GRASS_BLOCK_ID}"));
    let chunk_origin = coord * CHUNK_SIZE as i32;
    let columns = sample_generation_columns(
        chunk_origin,
        fluids,
        dimension,
        biomes,
        biome_field,
    );
    let mut chunk = VoxelChunk::empty();

    rasterize_solid_pass(
        &mut chunk,
        chunk_origin,
        &columns,
        grass.rotate_texture,
        biomes,
        biome_field,
    );
    rasterize_fluid_pass(
        &mut chunk,
        chunk_origin,
        &columns,
        dimension.sea_level,
        biomes,
        biome_field,
    );

    chunk
}

fn sample_generation_columns(
    chunk_origin: IVec3,
    fluids: &FluidRegistry,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> Vec<GenerationColumnSample> {
    let mut columns = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;
            let position = IVec2::new(world_x, world_z);
            let sample =
                biome_field.sample_surface(position.as_vec2() + Vec2::splat(0.5));

            columns.push(GenerationColumnSample {
                surface_height: surface_height_from_sample(
                    position,
                    dimension,
                    biomes,
                    biome_field.seed(),
                    &sample,
                ),
                surface_fluid: surface_fluid_from_sample(&sample, biomes, fluids),
            });
        }
    }

    columns
}

fn rasterize_solid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    rotate_texture: bool,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) {
    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let world_x = chunk_origin.x + local_x as i32;
            let world_z = chunk_origin.z + local_z as i32;

            for local_y in 0..CHUNK_SIZE {
                let world_y = chunk_origin.y + local_y as i32;
                let world_position = IVec3::new(world_x, world_y, world_z);

                if final_terrain_density(
                    column.surface_height,
                    world_position,
                    biomes,
                    biome_field,
                ) <= 0.0
                {
                    continue;
                }

                let rotation = texture_rotation_for(world_position, rotate_texture);
                chunk.set_block(
                    local_x,
                    local_y,
                    local_z,
                    Some(VoxelCell::new(GRASS_BLOCK_ID, rotation)),
                );
            }
        }
    }
}

fn rasterize_fluid_pass(
    chunk: &mut VoxelChunk,
    chunk_origin: IVec3,
    columns: &[GenerationColumnSample],
    sea_level: i32,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) {
    for local_z in 0..CHUNK_SIZE {
        for local_x in 0..CHUNK_SIZE {
            let column = &columns[column_index(local_x, local_z)];
            let Some(fluid_id) = column.surface_fluid else {
                continue;
            };

            for local_y in 0..CHUNK_SIZE {
                let world_y = chunk_origin.y + local_y as i32;
                let world_position = IVec3::new(
                    chunk_origin.x + local_x as i32,
                    world_y,
                    chunk_origin.z + local_z as i32,
                );

                if world_y >= sea_level
                    || final_terrain_density(
                        column.surface_height,
                        world_position,
                        biomes,
                        biome_field,
                    ) > 0.0
                {
                    continue;
                }

                chunk.set_fluid(
                    local_x,
                    local_y,
                    local_z,
                    Some(FluidCell::source(fluid_id)),
                );
            }
        }
    }
}

fn final_terrain_density(
    surface_height: i32,
    world_position: IVec3,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
) -> f32 {
    let base_density = terrain_density(surface_height, world_position.y);
    let sample_position = world_position.as_vec3() + Vec3::splat(0.5);
    let Some(volume) = biome_field.sample_volume(sample_position) else {
        return base_density;
    };

    let mut density_delta = 0.0;

    for influence in &volume.influences {
        let biome = biomes
            .get(influence.id)
            .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
        let Some(modifier) = biome.density_modifier else {
            continue;
        };
        let seed = mix_seed(biome_field.seed() ^ string_hash(&biome.id));

        density_delta += density_modifier_delta(modifier, sample_position, seed)
            * influence.weight
            * volume.strength;
    }

    base_density + density_delta
}

fn density_modifier_delta(
    modifier: BiomeDensityModifier,
    position: Vec3,
    seed: u64,
) -> f32 {
    match modifier {
        BiomeDensityModifier::Cavern {
            carve_strength,
            noise_scale,
            openness,
        } => {
            let noise = value_noise_3d(position * noise_scale, seed) * 0.5 + 0.5;
            let mask = coverage_mask(noise, openness);
            -carve_strength * mask
        }
        BiomeDensityModifier::Solid {
            fill_strength,
            noise_scale,
            coverage,
        } => {
            let noise = value_noise_3d(position * noise_scale, seed) * 0.5 + 0.5;
            let mask = coverage_mask(noise, coverage);
            fill_strength * mask
        }
    }
}

fn coverage_mask(noise: f32, coverage: f32) -> f32 {
    if coverage <= 0.0 {
        return 0.0;
    }

    if coverage >= 1.0 {
        return 1.0;
    }

    let threshold = 1.0 - coverage;
    smoothstep(((noise - threshold) / DENSITY_NOISE_EDGE).clamp(0.0, 1.0))
}

fn surface_fluid_from_sample(
    sample: &BiomeFieldSample<'_>,
    biomes: &BiomeRegistry,
    fluids: &FluidRegistry,
) -> Option<FluidId> {
    sample
        .influences
        .iter()
        .filter_map(|influence| {
            let biome = biomes
                .get(influence.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", influence.id));
            let fluid_id = biome.surface_fluid.as_deref()?;
            let fluid = fluids.id_of(fluid_id).unwrap_or_else(|| {
                panic!(
                    "biome {} references missing surface fluid: {fluid_id}",
                    biome.id
                )
            });

            Some((fluid, influence.weight))
        })
        .max_by(|(_, left_weight), (_, right_weight)| left_weight.total_cmp(right_weight))
        .map(|(fluid_id, _)| fluid_id)
}

fn value_noise_3d(position: Vec3, seed: u64) -> f32 {
    let x0 = position.x.floor() as i32;
    let y0 = position.y.floor() as i32;
    let z0 = position.z.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;
    let tx = smoothstep(position.x - x0 as f32);
    let ty = smoothstep(position.y - y0 as f32);
    let tz = smoothstep(position.z - z0 as f32);

    let c000 = lattice_noise_3d(x0, y0, z0, seed);
    let c100 = lattice_noise_3d(x1, y0, z0, seed);
    let c010 = lattice_noise_3d(x0, y1, z0, seed);
    let c110 = lattice_noise_3d(x1, y1, z0, seed);
    let c001 = lattice_noise_3d(x0, y0, z1, seed);
    let c101 = lattice_noise_3d(x1, y0, z1, seed);
    let c011 = lattice_noise_3d(x0, y1, z1, seed);
    let c111 = lattice_noise_3d(x1, y1, z1, seed);

    let x00 = lerp(c000, c100, tx);
    let x10 = lerp(c010, c110, tx);
    let x01 = lerp(c001, c101, tx);
    let x11 = lerp(c011, c111, tx);
    let y0 = lerp(x00, x10, ty);
    let y1 = lerp(x01, x11, ty);

    lerp(y0, y1, tz)
}

fn lattice_noise_3d(x: i32, y: i32, z: i32, seed: u64) -> f32 {
    let mut hash = seed;
    hash ^= (x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (y as i64 as u64).wrapping_mul(0xd6e8_feb8_6659_fd93);
    hash ^= (z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    let normalized = (hash & 0xffff) as f32 / u16::MAX as f32;

    normalized * 2.0 - 1.0
}

fn string_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    hash
}

fn mix_seed(mut seed: u64) -> u64 {
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xff51_afd7_ed55_8ccd);
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    seed ^= seed >> 33;
    seed
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn column_index(x: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE
}

fn texture_rotation_for(position: IVec3, enabled: bool) -> TextureRotation {
    if !enabled {
        return TextureRotation::default();
    }

    let mut hash = position.x as u32;
    hash ^= (position.y as u32).wrapping_mul(0x9e37_79b9);
    hash = hash.rotate_left(13);
    hash ^= (position.z as u32).wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 16;

    TextureRotation::from_quarter_turn((hash & 3) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cavern_and_solid_modifiers_move_density_in_opposite_directions() {
        let position = Vec3::new(12.5, 30.5, -8.5);
        let cavern = density_modifier_delta(
            BiomeDensityModifier::Cavern {
                carve_strength: 20.0,
                noise_scale: 0.01,
                openness: 1.0,
            },
            position,
            7,
        );
        let solid = density_modifier_delta(
            BiomeDensityModifier::Solid {
                fill_strength: 20.0,
                noise_scale: 0.01,
                coverage: 1.0,
            },
            position,
            7,
        );

        assert_eq!(cavern, -20.0);
        assert_eq!(solid, 20.0);
    }
}
