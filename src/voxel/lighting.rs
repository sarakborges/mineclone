use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use crate::content::{
    block::BlockRegistry,
    fluid::FluidRegistry,
};

use super::{
    chunk::CHUNK_SIZE,
    light::VoxelLight,
    world::VoxelWorld,
};

const NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

#[derive(Default)]
struct LightingContext {
    direct_sky_levels_by_column: HashMap<IVec2, Vec<u8>>,
    highest_loaded_y_by_chunk_column: HashMap<IVec2, Option<i32>>,
}

impl LightingContext {
    fn direct_sky_level(
        &mut self,
        world: &VoxelWorld,
        fluids: &FluidRegistry,
        position: IVec3,
    ) -> u8 {
        if position.y < 0 || world.is_solid(position) {
            return 0;
        }

        let column = IVec2::new(position.x, position.z);
        if !self.direct_sky_levels_by_column.contains_key(&column) {
            let size = CHUNK_SIZE as i32;
            let chunk_column = IVec2::new(
                position.x.div_euclid(size),
                position.z.div_euclid(size),
            );
            let highest_loaded_y = if let Some(cached) = self
                .highest_loaded_y_by_chunk_column
                .get(&chunk_column)
                .copied()
            {
                cached
            } else {
                let highest = world.highest_loaded_world_y_in_column(position.x, position.z);
                self.highest_loaded_y_by_chunk_column
                    .insert(chunk_column, highest);
                highest
            };
            let levels = build_direct_sky_column(world, fluids, column, highest_loaded_y);
            self.direct_sky_levels_by_column.insert(column, levels);
        }

        self.direct_sky_levels_by_column
            .get(&column)
            .and_then(|levels| levels.get(position.y as usize))
            .copied()
            .unwrap_or(VoxelLight::MAX_LEVEL)
    }
}

pub(crate) fn initialize_chunk_lighting(
    world: &mut VoxelWorld,
    coord: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    if !world.clear_chunk_light(coord) {
        return HashSet::new();
    }

    let size = CHUNK_SIZE as i32;
    let origin = coord * size;
    let mut queue = VecDeque::new();
    let mut queued = HashSet::new();

    enqueue_chunk_voxels(origin, &mut queue, &mut queued);
    enqueue_chunk_boundary_neighbors(origin, &mut queue, &mut queued);

    relax(world, blocks, fluids, &mut queue, &mut queued)
}

pub(crate) fn relight_after_voxel_edit(
    world: &mut VoxelWorld,
    position: IVec3,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    let mut queue = VecDeque::new();
    let mut queued = HashSet::new();

    enqueue(position, &mut queue, &mut queued);
    for direction in NEIGHBORS {
        enqueue(position + direction, &mut queue, &mut queued);
    }

    relax(world, blocks, fluids, &mut queue, &mut queued)
}

pub(crate) fn relight_after_chunk_unloads(
    world: &mut VoxelWorld,
    unloaded: &[IVec3],
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
) -> HashSet<IVec3> {
    let size = CHUNK_SIZE as i32;
    let mut queue = VecDeque::new();
    let mut queued = HashSet::new();

    for coord in unloaded {
        enqueue_chunk_boundary_neighbors(*coord * size, &mut queue, &mut queued);
    }

    relax(world, blocks, fluids, &mut queue, &mut queued)
}

fn relax(
    world: &mut VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    queue: &mut VecDeque<IVec3>,
    queued: &mut HashSet<IVec3>,
) -> HashSet<IVec3> {
    let mut changed_chunks = HashSet::new();
    let mut context = LightingContext::default();

    while let Some(position) = queue.pop_front() {
        queued.remove(&position);

        if !world.is_loaded_at(position) {
            continue;
        }

        let current = world.light_at(position);
        let desired = desired_light(world, blocks, fluids, position, &mut context);

        if current == desired {
            continue;
        }

        world.set_light_at(position, desired);
        changed_chunks.insert(chunk_coord(position));

        for direction in NEIGHBORS {
            enqueue(position + direction, queue, queued);
        }
    }

    changed_chunks
}

fn desired_light(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    fluids: &FluidRegistry,
    position: IVec3,
    context: &mut LightingContext,
) -> VoxelLight {
    let solid = world.is_solid(position);
    let attenuation = propagation_cost(world, fluids, position);
    let sky = if solid {
        0
    } else {
        context
            .direct_sky_level(world, fluids, position)
            .max(propagated_neighbor_level(world, position, attenuation, |light| {
                light.sky()
            }))
    };
    let emitted = block_emission(world, blocks, position);
    let block = if solid {
        emitted
    } else {
        emitted.max(propagated_neighbor_level(
            world,
            position,
            attenuation,
            |light| light.block(),
        ))
    };

    VoxelLight::new(sky, block)
}

fn build_direct_sky_column(
    world: &VoxelWorld,
    fluids: &FluidRegistry,
    column: IVec2,
    highest_y: Option<i32>,
) -> Vec<u8> {
    let Some(highest_y) = highest_y else {
        return Vec::new();
    };
    let mut levels = vec![0; highest_y as usize + 1];
    let mut level = VoxelLight::MAX_LEVEL;

    for y in (0..=highest_y).rev() {
        let position = IVec3::new(column.x, y, column.y);

        if world.is_solid(position) {
            level = 0;
        } else {
            level = level.saturating_sub(fluid_dampening(world, fluids, position));
        }

        levels[y as usize] = level;
    }

    levels
}

fn propagated_neighbor_level<F>(
    world: &VoxelWorld,
    position: IVec3,
    attenuation: u8,
    channel: F,
) -> u8
where
    F: Fn(VoxelLight) -> u8,
{
    NEIGHBORS
        .into_iter()
        .map(|direction| {
            channel(world.light_at(position + direction)).saturating_sub(attenuation)
        })
        .max()
        .unwrap_or(0)
}

fn propagation_cost(world: &VoxelWorld, fluids: &FluidRegistry, position: IVec3) -> u8 {
    fluid_dampening(world, fluids, position).max(1)
}

fn fluid_dampening(world: &VoxelWorld, fluids: &FluidRegistry, position: IVec3) -> u8 {
    let Some(cell) = world.fluid_at(position) else {
        return 0;
    };

    fluids
        .get(cell.fluid_id)
        .unwrap_or_else(|| panic!("missing fluid definition for id {}", cell.fluid_id))
        .light_dampening
        .min(VoxelLight::MAX_LEVEL)
}

fn block_emission(world: &VoxelWorld, blocks: &BlockRegistry, position: IVec3) -> u8 {
    let Some(block_id) = world.block_id_at(position) else {
        return 0;
    };

    blocks
        .get(block_id)
        .map(|block| block.light_emission.min(VoxelLight::MAX_LEVEL))
        .unwrap_or(0)
}

fn enqueue_chunk_voxels(
    origin: IVec3,
    queue: &mut VecDeque<IVec3>,
    queued: &mut HashSet<IVec3>,
) {
    let size = CHUNK_SIZE as i32;

    for y in 0..size {
        for z in 0..size {
            for x in 0..size {
                enqueue(origin + IVec3::new(x, y, z), queue, queued);
            }
        }
    }
}

fn enqueue_chunk_boundary_neighbors(
    origin: IVec3,
    queue: &mut VecDeque<IVec3>,
    queued: &mut HashSet<IVec3>,
) {
    let size = CHUNK_SIZE as i32;

    for y in 0..size {
        for z in 0..size {
            enqueue(origin + IVec3::new(-1, y, z), queue, queued);
            enqueue(origin + IVec3::new(size, y, z), queue, queued);
        }
    }

    for y in 0..size {
        for x in 0..size {
            enqueue(origin + IVec3::new(x, y, -1), queue, queued);
            enqueue(origin + IVec3::new(x, y, size), queue, queued);
        }
    }

    for z in 0..size {
        for x in 0..size {
            enqueue(origin + IVec3::new(x, -1, z), queue, queued);
            enqueue(origin + IVec3::new(x, size, z), queue, queued);
        }
    }
}

fn enqueue(position: IVec3, queue: &mut VecDeque<IVec3>, queued: &mut HashSet<IVec3>) {
    if position.y >= 0 && queued.insert(position) {
        queue.push_back(position);
    }
}

fn chunk_coord(world_position: IVec3) -> IVec3 {
    let size = CHUNK_SIZE as i32;

    IVec3::new(
        world_position.x.div_euclid(size),
        world_position.y.div_euclid(size),
        world_position.z.div_euclid(size),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        content::{
            block::{BlockDefinition, BlockTextures},
            color::Rgb,
            fluid::FluidDefinition,
        },
        voxel::{
            cell::VoxelCell,
            chunk::VoxelChunk,
            fluid::FluidCell,
            texture_rotation::TextureRotation,
        },
    };

    const OPAQUE_BLOCK_ID: &str = "test:opaque";
    const LAMP_BLOCK_ID: &str = "test:lamp";
    const WATER_ID: &str = "test:water";

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

        let mut upper = VoxelChunk::empty();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    upper.set_block(
                        x,
                        y,
                        z,
                        Some(VoxelCell::new(
                            OPAQUE_BLOCK_ID,
                            TextureRotation::default(),
                        )),
                    );
                }
            }
        }

        world.insert_chunk(IVec3::Y, upper);
        initialize_chunk_lighting(&mut world, IVec3::Y, &blocks, &fluids);

        assert_eq!(world.light_at(below).sky(), 0);
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
                    chunk.set_fluid(x, y, z, Some(FluidCell::source(water_id)));
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
            Some(FluidCell::source(water_id)),
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
    fn blocklight_is_removed_across_chunk_boundary_after_source_chunk_unloads() {
        let blocks = test_blocks();
        let fluids = test_fluids();
        let mut world = VoxelWorld::default();
        let source_chunk = IVec3::ZERO;
        let neighbor_chunk = IVec3::X;
        let source = IVec3::new(CHUNK_SIZE as i32 - 1, 8, 8);
        let neighbor = source + IVec3::X;

        world.insert_chunk(source_chunk, VoxelChunk::empty());
        world.insert_chunk(neighbor_chunk, VoxelChunk::empty());
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
            textures: BlockTextures {
                top: String::new(),
                bottom: String::new(),
                left: String::new(),
                right: String::new(),
                front: String::new(),
                back: String::new(),
            },
            rotate_texture: false,
            light_emission,
        }
    }
}
