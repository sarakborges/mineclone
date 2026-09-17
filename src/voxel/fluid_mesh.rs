use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::fluid::FluidId;

use super::{
    block_face::BlockFace,
    chunk::{CHUNK_SIZE, VoxelChunk},
    fluid::FluidCell,
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{face_lighting, push_lit_quad, surface_block_srgb},
    quad::VOXEL_FACE_UVS,
    read::VoxelRead,
};

pub struct ChunkFluidMesh {
    pub fluid_id: FluidId,
    pub mesh: Mesh,
}

#[derive(Clone, Copy)]
struct FluidFaceHeights {
    h00: f32,
    h10: f32,
    h11: f32,
    h01: f32,
}

pub fn build_fluid_meshes<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    tint_at: F,
) -> Vec<ChunkFluidMesh>
where
    W: VoxelRead + ?Sized,
    F: Fn(IVec3, FluidId) -> [f32; 3],
{
    let mut buffers = HashMap::<FluidId, VoxelMeshBuffer>::new();
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = chunk_coord * chunk_size;

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let Some(cell) = chunk.fluid_at(x as i32, y as i32, z as i32) else {
                    continue;
                };

                let world_voxel = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
                let exposed = BlockFace::ALL.map(|face| {
                    if face == BlockFace::Bottom && world_voxel.y <= 0 {
                        return false;
                    }
                    face_is_exposed(
                        world,
                        world_voxel + face.offset(),
                        cell.fluid_id,
                        face,
                    )
                });
                if !exposed.iter().any(|value| *value) {
                    continue;
                }

                let tint = tint_at(world_voxel, cell.fluid_id);
                let heights = fluid_face_heights(world, world_voxel, cell.fluid_id);
                let source_block_srgb = surface_block_srgb(
                    chunk.light_at(x as i32, y as i32, z as i32),
                    false,
                );
                let fluid = buffers.entry(cell.fluid_id).or_default();

                for (face, is_exposed) in BlockFace::ALL.into_iter().zip(exposed) {
                    if !is_exposed {
                        continue;
                    }

                    push_lit_quad(
                        fluid,
                        fluid_face_vertices(face, x as f32, y as f32, z as f32, heights),
                        face.normal(),
                        VOXEL_FACE_UVS,
                        tint,
                        face_lighting(world, world_voxel, face, source_block_srgb),
                    );
                }
            }
        }
    }

    let mut meshes = buffers
        .into_iter()
        .filter_map(|(fluid_id, buffer)| {
            buffer
                .into_mesh()
                .map(|mesh| ChunkFluidMesh { fluid_id, mesh })
        })
        .collect::<Vec<_>>();
    meshes.sort_by_key(|mesh| mesh.fluid_id);
    meshes
}

fn fluid_face_vertices(
    face: BlockFace,
    x0: f32,
    y0: f32,
    z0: f32,
    heights: FluidFaceHeights,
) -> [[f32; 3]; 4] {
    let x1 = x0 + 1.0;
    let z1 = z0 + 1.0;

    match face {
        BlockFace::Right => [
            [x1, y0, z1],
            [x1, y0, z0],
            [x1, y0 + heights.h10, z0],
            [x1, y0 + heights.h11, z1],
        ],
        BlockFace::Left => [
            [x0, y0, z0],
            [x0, y0, z1],
            [x0, y0 + heights.h01, z1],
            [x0, y0 + heights.h00, z0],
        ],
        BlockFace::Top => [
            [x0, y0 + heights.h01, z1],
            [x1, y0 + heights.h11, z1],
            [x1, y0 + heights.h10, z0],
            [x0, y0 + heights.h00, z0],
        ],
        BlockFace::Bottom => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        BlockFace::Front => [
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y0 + heights.h11, z1],
            [x0, y0 + heights.h01, z1],
        ],
        BlockFace::Back => [
            [x1, y0, z0],
            [x0, y0, z0],
            [x0, y0 + heights.h00, z0],
            [x1, y0 + heights.h10, z0],
        ],
    }
}

fn fluid_face_heights<W: VoxelRead + ?Sized>(
    world: &W,
    position: IVec3,
    fluid_id: FluidId,
) -> FluidFaceHeights {
    let mut current = [[None; 3]; 3];
    let mut above = [[None; 3]; 3];

    for z in -1..=1 {
        for x in -1..=1 {
            let x_index = (x + 1) as usize;
            let z_index = (z + 1) as usize;
            let sample_position = position + IVec3::new(x, 0, z);
            current[z_index][x_index] = world.fluid_at(sample_position);
            above[z_index][x_index] = world.fluid_at(sample_position + IVec3::Y);
        }
    }

    FluidFaceHeights {
        h00: fluid_corner_height(&current, &above, fluid_id, 0, 0),
        h10: fluid_corner_height(&current, &above, fluid_id, 2, 0),
        h11: fluid_corner_height(&current, &above, fluid_id, 2, 2),
        h01: fluid_corner_height(&current, &above, fluid_id, 0, 2),
    }
}

fn fluid_corner_height(
    current: &[[Option<FluidCell>; 3]; 3],
    above: &[[Option<FluidCell>; 3]; 3],
    fluid_id: FluidId,
    x_index: usize,
    z_index: usize,
) -> f32 {
    let positions = [(1, 1), (x_index, 1), (1, z_index), (x_index, z_index)];

    if positions.iter().any(|&(x, z)| {
        above[z][x].is_some_and(|cell| cell.fluid_id == fluid_id)
    }) {
        return 1.0;
    }

    let mut total = 0.0;
    let mut count = 0.0;
    for (x, z) in positions {
        if let Some(cell) = current[z][x].filter(|cell| cell.fluid_id == fluid_id) {
            total += cell.height();
            count += 1.0;
        }
    }

    if count > 0.0 { total / count } else { 0.0 }
}

fn face_is_exposed<W: VoxelRead + ?Sized>(
    world: &W,
    position: IVec3,
    fluid_id: FluidId,
    face: BlockFace,
) -> bool {
    let Some((cell, fluid, _)) = world.sample_at(position) else {
        return face == BlockFace::Top;
    };

    if let Some(block) = cell {
        if !crate::voxel::microblock::MicroblockMask::is_modified(block) {
            return false;
        }
        if !partial_block_face_has_opening(block, face) {
            return false;
        }
    }

    match fluid {
        Some(neighbor) => neighbor.fluid_id != fluid_id,
        None => true,
    }
}

fn partial_block_face_has_opening(cell: VoxelCell, face: BlockFace) -> bool {
    let mask = crate::voxel::microblock::MicroblockMask::from_cell(cell);
    if mask == crate::voxel::microblock::MicroblockMask::FULL {
        return false;
    }

    for a in 0..crate::voxel::microblock::MICROBLOCK_EDGE as usize {
        for b in 0..crate::voxel::microblock::MICROBLOCK_EDGE as usize {
            let position = match face {
                BlockFace::Right => [0, a, b],
                BlockFace::Left => [7, a, b],
                BlockFace::Top => [a, b, 0],
                BlockFace::Bottom => [a, b, 7],
                BlockFace::Front => [a, b, 0],
                BlockFace::Back => [a, b, 7],
            };
            if !mask.contains(position) {
                return true;
            }
        }
    }
    false

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{cell::VoxelCell, microblock::ChiselResolution};

    #[test]
    fn partial_block_face_opening_exposes_fluid() {
        let cell = VoxelCell::new("stone", Default::default());
        for (face, position) in [
            (BlockFace::Right, [0, 0, 0]),
            (BlockFace::Left, [7, 0, 0]),
            (BlockFace::Top, [0, 0, 0]),
            (BlockFace::Bottom, [0, 7, 0]),
            (BlockFace::Front, [0, 0, 0]),
            (BlockFace::Back, [0, 0, 7]),
        ] {
            let mut mask = crate::voxel::microblock::MicroblockMask::FULL;
            mask.edit(position, ChiselResolution::ExtraThin, false);
            let partial = mask.apply_to_cell(cell, true);
            assert!(partial_block_face_has_opening(partial, face));
        }
    }

    #[test]
    fn full_block_face_stays_closed_to_fluid() {
        let cell = VoxelCell::new("stone", Default::default());
        assert!(!partial_block_face_has_opening(cell, BlockFace::Right));
    }

    #[test]
    fn partial_block_face_requires_opening_on_that_face() {
        let cell = VoxelCell::new("stone", Default::default());
        for (face, open_position, wrong_position) in [
            (BlockFace::Right, [0, 0, 0], [7, 0, 0]),
            (BlockFace::Left, [7, 0, 0], [0, 0, 0]),
            (BlockFace::Top, [0, 0, 0], [0, 7, 0]),
            (BlockFace::Bottom, [0, 7, 0], [0, 0, 0]),
            (BlockFace::Front, [0, 0, 0], [0, 0, 7]),
            (BlockFace::Back, [0, 0, 7], [0, 0, 0]),
        ] {
            let mut mask = crate::voxel::microblock::MicroblockMask::FULL;
            mask.edit(wrong_position, ChiselResolution::ExtraThin, false);
            let partial = mask.apply_to_cell(cell, true);
            assert!(!partial_block_face_has_opening(partial, face));

            let mut mask = crate::voxel::microblock::MicroblockMask::FULL;
            mask.edit(open_position, ChiselResolution::ExtraThin, false);
            let partial = mask.apply_to_cell(cell, true);
            assert!(partial_block_face_has_opening(partial, face));
        }
    }
}
