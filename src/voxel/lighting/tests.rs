use super::*;
use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry, BlockTextures},
        color::Rgb,
        fluid::{FluidDefinition, FluidRegistry},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
        light::VoxelLight,
        texture_rotation::TextureRotation,
        world::VoxelWorld,
    },
};

const OPAQUE_BLOCK_ID: &str = "asteria:test/opaque";
const LAMP_BLOCK_ID: &str = "asteria:test/lamp";
const WATER_ID: &str = "asteria:test/water";

#[test]
fn direct_skylight_is_restored_after_removing_a_blocker() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let mut world = empty_world();
    let blocker = IVec3::new(8, 10, 8);
    let below = blocker - IVec3::Y;

    world.set_block_at(
        blocker,
        Some(VoxelCell::new(OPAQUE_BLOCK_ID, TextureRotation::default())),
    );
    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);

    assert!(world.light_at(below).sky() < VoxelLight::MAX_LEVEL);

    world.set_block_at(blocker, None);
    relight_after_voxel_edit(&mut world, blocker, &blocks, &fluids);

    assert_eq!(world.light_at(below).sky(), VoxelLight::MAX_LEVEL);
}

#[test]
fn loading_opaque_chunk_above_invalidates_existing_skylight_below() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let mut world = empty_world();
    let below = IVec3::new(8, CHUNK_SIZE as i32 - 1, 8);

    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);
    assert_eq!(world.light_at(below).sky(), VoxelLight::MAX_LEVEL);

    world.insert_chunk(IVec3::Y, opaque_chunk());
    initialize_chunk_lighting(&mut world, IVec3::Y, &blocks, &fluids);

    assert_eq!(world.light_at(below).sky(), 0);
}

#[test]
fn unloading_opaque_chunk_above_restores_existing_skylight_below() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let mut world = empty_world();
    let upper_chunk = IVec3::Y;
    let below = IVec3::new(8, CHUNK_SIZE as i32 - 1, 8);

    world.insert_chunk(upper_chunk, opaque_chunk());
    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);
    initialize_chunk_lighting(&mut world, upper_chunk, &blocks, &fluids);
    assert_eq!(world.light_at(below).sky(), 0);

    world.archive_chunk(upper_chunk);
    relight_after_chunk_unloads(&mut world, &[upper_chunk], &blocks, &fluids);

    assert_eq!(world.light_at(below).sky(), VoxelLight::MAX_LEVEL);
}

#[test]
fn skylight_dampens_with_fluid_depth() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let water_id = fluids.id_of(WATER_ID).expect("test water should exist");
    let mut chunk = VoxelChunk::empty();

    for y in 10..=12 {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set_fluid(x, y, z, Some(FluidCell::new(water_id, MAX_FLUID_LEVEL)));
            }
        }
    }

    let mut world = VoxelWorld::default();
    world.insert_chunk(IVec3::ZERO, chunk);
    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);

    assert_eq!(world.light_at(IVec3::new(8, 12, 8)).sky(), 13);
    assert_eq!(world.light_at(IVec3::new(8, 11, 8)).sky(), 11);
    assert_eq!(world.light_at(IVec3::new(8, 10, 8)).sky(), 9);
}

#[test]
fn blocklight_uses_fluid_dampening_as_propagation_cost() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let water_id = fluids.id_of(WATER_ID).expect("test water should exist");
    let source = IVec3::new(7, 8, 8);
    let water = source + IVec3::X;
    let after_water = water + IVec3::X;
    let mut chunk = VoxelChunk::empty();

    chunk.set_fluid(
        water.x as usize,
        water.y as usize,
        water.z as usize,
        Some(FluidCell::new(water_id, MAX_FLUID_LEVEL)),
    );

    let mut world = VoxelWorld::default();
    world.insert_chunk(IVec3::ZERO, chunk);
    world.set_block_at(
        source,
        Some(VoxelCell::new(LAMP_BLOCK_ID, TextureRotation::default())),
    );
    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);

    assert_eq!(world.light_at(water).block(), 13);
    assert_eq!(world.light_at(after_water).block(), 12);
}

#[test]
fn fluid_dampening_applies_across_chunk_boundary() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let water_id = fluids.id_of(WATER_ID).expect("test water should exist");
    let source = IVec3::new(CHUNK_SIZE as i32 - 1, 8, 8);
    let water = source + IVec3::X;
    let after_water = water + IVec3::X;
    let mut neighbor_chunk = VoxelChunk::empty();
    neighbor_chunk.set_fluid(0, 8, 8, Some(FluidCell::new(water_id, MAX_FLUID_LEVEL)));

    let mut world = VoxelWorld::default();
    world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());
    world.insert_chunk(IVec3::X, neighbor_chunk);
    world.set_block_at(
        source,
        Some(VoxelCell::new(LAMP_BLOCK_ID, TextureRotation::default())),
    );
    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);
    initialize_chunk_lighting(&mut world, IVec3::X, &blocks, &fluids);

    assert_eq!(world.light_at(water).block(), 13);
    assert_eq!(world.light_at(after_water).block(), 12);
}

#[test]
fn blocklight_propagates_and_converges_after_source_removal() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let mut world = empty_world();
    let source = IVec3::new(8, 8, 8);
    let neighbor = source + IVec3::X;

    world.set_block_at(
        source,
        Some(VoxelCell::new(LAMP_BLOCK_ID, TextureRotation::default())),
    );
    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);

    assert_eq!(world.light_at(source).block(), VoxelLight::MAX_LEVEL);
    assert_eq!(world.light_at(neighbor).block(), VoxelLight::MAX_LEVEL - 1);

    world.set_block_at(source, None);
    relight_after_voxel_edit(&mut world, source, &blocks, &fluids);

    assert_eq!(world.light_at(source).block(), 0);
    assert_eq!(world.light_at(neighbor).block(), 0);
}

#[test]
fn blocklight_addition_crosses_chunk_boundary_after_voxel_edit() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let mut world = two_chunk_world(IVec3::ZERO, IVec3::X);
    let source = IVec3::new(CHUNK_SIZE as i32 - 1, 8, 8);
    let neighbor = source + IVec3::X;

    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);
    initialize_chunk_lighting(&mut world, IVec3::X, &blocks, &fluids);
    assert_eq!(world.light_at(neighbor).block(), 0);

    world.set_block_at(
        source,
        Some(VoxelCell::new(LAMP_BLOCK_ID, TextureRotation::default())),
    );
    relight_after_voxel_edit(&mut world, source, &blocks, &fluids);

    assert_eq!(world.light_at(source).block(), VoxelLight::MAX_LEVEL);
    assert_eq!(world.light_at(neighbor).block(), VoxelLight::MAX_LEVEL - 1);
}

#[test]
fn blocklight_addition_crosses_negative_chunk_boundary() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let negative_chunk = IVec3::NEG_X;
    let mut world = two_chunk_world(negative_chunk, IVec3::ZERO);
    let source = IVec3::new(-1, 8, 8);
    let neighbor = IVec3::new(0, 8, 8);

    initialize_chunk_lighting(&mut world, negative_chunk, &blocks, &fluids);
    initialize_chunk_lighting(&mut world, IVec3::ZERO, &blocks, &fluids);
    assert_eq!(world.light_at(neighbor).block(), 0);

    world.set_block_at(
        source,
        Some(VoxelCell::new(LAMP_BLOCK_ID, TextureRotation::default())),
    );
    relight_after_voxel_edit(&mut world, source, &blocks, &fluids);

    assert_eq!(world.light_at(source).block(), VoxelLight::MAX_LEVEL);
    assert_eq!(world.light_at(neighbor).block(), VoxelLight::MAX_LEVEL - 1);
}

#[test]
fn blocklight_is_removed_across_chunk_boundary_after_source_chunk_unloads() {
    let blocks = test_blocks();
    let fluids = test_fluids();
    let mut world = two_chunk_world(IVec3::ZERO, IVec3::X);
    let source_chunk = IVec3::ZERO;
    let neighbor_chunk = IVec3::X;
    let source = IVec3::new(CHUNK_SIZE as i32 - 1, 8, 8);
    let neighbor = source + IVec3::X;

    world.set_block_at(
        source,
        Some(VoxelCell::new(LAMP_BLOCK_ID, TextureRotation::default())),
    );
    initialize_chunk_lighting(&mut world, source_chunk, &blocks, &fluids);
    initialize_chunk_lighting(&mut world, neighbor_chunk, &blocks, &fluids);

    assert_eq!(world.light_at(neighbor).block(), VoxelLight::MAX_LEVEL - 1);

    world.archive_chunk(source_chunk);
    relight_after_chunk_unloads(&mut world, &[source_chunk], &blocks, &fluids);

    assert_eq!(world.light_at(neighbor).block(), 0);
}

fn empty_world() -> VoxelWorld {
    let mut world = VoxelWorld::default();
    world.insert_chunk(IVec3::ZERO, VoxelChunk::empty());
    world
}

fn two_chunk_world(first: IVec3, second: IVec3) -> VoxelWorld {
    let mut world = VoxelWorld::default();
    world.insert_chunk(first, VoxelChunk::empty());
    world.insert_chunk(second, VoxelChunk::empty());
    world
}

fn opaque_chunk() -> VoxelChunk {
    let mut chunk = VoxelChunk::empty();

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set_block(
                    x,
                    y,
                    z,
                    Some(VoxelCell::new(OPAQUE_BLOCK_ID, TextureRotation::default())),
                );
            }
        }
    }

    chunk
}

fn test_blocks() -> BlockRegistry {
    let mut blocks = BlockRegistry::default();
    blocks.insert(test_block(OPAQUE_BLOCK_ID, 0));
    blocks.insert(test_block(LAMP_BLOCK_ID, VoxelLight::MAX_LEVEL));
    blocks
}

fn test_fluids() -> FluidRegistry {
    let mut fluids = FluidRegistry::default();
    fluids.insert(FluidDefinition {
        id: WATER_ID.to_owned(),
        name: WATER_ID.to_owned(),
        color: Rgb {
            r: 0.0,
            g: 0.0,
            b: 1.0,
        },
        opacity: 0.5,
        roughness: 0.0,
        metallic: 0.0,
        light_dampening: 2,
    });
    fluids
}

fn test_block(id: &str, light_emission: u8) -> BlockDefinition {
    BlockDefinition {
        id: id.to_owned(),
        name: id.to_owned(),
        textures: BlockTextures::default(),
        rotate_texture: false,
        light_emission,
        light_dampening: VoxelLight::MAX_LEVEL,
        casts_shadow: true,
    }
}
