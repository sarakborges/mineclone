use bevy::prelude::*;

use crate::{
    content::{
        block::{BlockDefinition, BlockLookup, BlockRegistry, BlockTextureRotations},
        block_orientation::BlockOrientation,
    },
    rendering::block_texture::block_face_material_face,
};

use self::{
    geometry::{face_geometry, is_face_exposed, orient_face_geometry},
    micro_mesh::{
        MicroMeshBuffers, MicroSurface, emit_neighbor_openings, emit_sculpted_faces,
        material_buffer, occludes,
    },
};
use super::{
    block_face::BlockFace,
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    mesh_lighting::{FaceLighting, face_lighting, push_lit_quad, surface_block_srgb},
    microblock::MicroblockMask,
    orientation::source_face_for_oriented_face,
    quad::VOXEL_FACE_UVS,
    read::VoxelRead,
    texture_rotation::TextureRotation,
};

pub(crate) mod geometry;
mod micro_mesh;

pub struct ChunkFaceMesh {
    pub block_id: &'static str,
    pub face: BlockFace,
    pub mesh: Mesh,
    pub casts_shadow: bool,
}

pub fn build_chunk_mesh<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    mut tint_at: F,
) -> Vec<ChunkFaceMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, VoxelCell, &crate::content::block::BlockDefinition) -> [f32; 3],
{
    let mut buffers = MicroMeshBuffers::default();
    let mut block_lookup = BlockLookup::new(blocks);
    let chunk_origin = chunk_coord * CHUNK_SIZE as i32;
    let mut visuals = vec![None; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];

    // Sculpted voxels keep the dedicated micro-mesher. Their geometry already
    // performs greedy rectangle merging at 1/8-block resolution.
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let Some(cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
                    continue;
                };
                if !MicroblockMask::is_modified(cell) {
                    continue;
                }

                let block = block_lookup.get(cell.block_id);
                let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
                let world_voxel = chunk_origin + local_voxel;
                let visual = visual_for_cell(
                    &mut visuals,
                    chunk,
                    [x, y, z],
                    world_voxel,
                    cell,
                    block,
                    &mut tint_at,
                );
                let surface = MicroSurface {
                    world,
                    cell,
                    block,
                    world_voxel,
                    local_voxel,
                    tint: visual.tint,
                    block_srgb: visual.block_srgb,
                };
                emit_sculpted_faces(&surface, &mut block_lookup, &mut buffers);
            }
        }
    }

    // Normal voxels are processed face-by-face so compatible exposed faces can
    // be merged into large rectangles. This first pass is intentionally
    // conservative: rotated, transparent, partially occluded, or non-uniformly
    // lit faces keep the old one-quad-per-face path.
    for face in BlockFace::ALL {
        for depth in 0..CHUNK_SIZE {
            let mut greedy = [None; CHUNK_SIZE * CHUNK_SIZE];

            for v in 0..CHUNK_SIZE {
                for u in 0..CHUNK_SIZE {
                    let [x, y, z] = face_cell(face, depth, u, v);
                    let Some(cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
                        continue;
                    };
                    if MicroblockMask::is_modified(cell) {
                        continue;
                    }

                    let block = block_lookup.get(cell.block_id);
                    let block_is_transparent = block.alpha_blend || block.alpha_cutoff.is_some();
                    let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
                    let world_voxel = chunk_origin + local_voxel;
                    let source_face = source_face_for_oriented_face(face, cell.orientation);
                    let partial_occluder = world
                        .cell_at(world_voxel + face.offset())
                        .filter(|neighbor| MicroblockMask::is_modified(*neighbor))
                        .filter(|neighbor| {
                            let definition = block_lookup.get(neighbor.block_id);
                            occludes(cell.block_id, block, neighbor.block_id, definition)
                        });

                    if partial_occluder.is_none()
                        && !is_face_exposed(
                            world,
                            &mut block_lookup,
                            cell.block_id,
                            block_is_transparent,
                            world_voxel,
                            face,
                        )
                    {
                        continue;
                    }

                    let visual = visual_for_cell(
                        &mut visuals,
                        chunk,
                        [x, y, z],
                        world_voxel,
                        cell,
                        block,
                        &mut tint_at,
                    );
                    let tint = visual.tint;
                    let source_block_srgb = visual.block_srgb;

                    if let Some(neighbor) = partial_occluder {
                        let surface = MicroSurface {
                            world,
                            cell,
                            block,
                            world_voxel,
                            local_voxel,
                            tint,
                            block_srgb: source_block_srgb,
                        };
                        emit_neighbor_openings(&surface, &mut buffers, face, neighbor);
                        continue;
                    }

                    let texture_rotation =
                        if face_uses_texture_rotation(block.rotate_texture, source_face) {
                            cell.texture_rotation
                        } else {
                            TextureRotation::default()
                        };
                    let lighting = face_lighting(world, world_voxel, face, source_block_srgb);
                    let material_face = block_face_material_face(source_face, block);

                    let candidate = GreedyFace {
                        block_id: cell.block_id,
                        material_face,
                        tint,
                        lighting,
                    };
                    let greedy_eligible = cell.orientation == BlockOrientation::Y
                        && !block_is_transparent
                        && texture_rotation == TextureRotation::Degrees0
                        && lighting_is_uniform(lighting);

                    if greedy_eligible {
                        greedy[u + v * CHUNK_SIZE] = Some(candidate);
                        continue;
                    }

                    let geometry = orient_face_geometry(
                        face_geometry(source_face, x, y, z, texture_rotation),
                        cell.orientation,
                        x,
                        y,
                        z,
                    );
                    push_lit_quad(
                        material_buffer(&mut buffers, cell.block_id, block, material_face),
                        geometry.vertices,
                        geometry.normal,
                        geometry.texture_rotation.rotate_uvs(VOXEL_FACE_UVS),
                        tint,
                        lighting,
                    );
                }
            }

            emit_greedy_plane(
                face,
                depth,
                &mut greedy,
                &mut block_lookup,
                &mut buffers,
            );
        }
    }

    let mut meshes = buffers
        .into_values()
        .filter_map(|entry| {
            entry.buffer.into_mesh().map(|mesh| ChunkFaceMesh {
                block_id: entry.block_id,
                face: entry.face,
                mesh,
                casts_shadow: entry.casts_shadow,
            })
        })
        .collect::<Vec<_>>();

    meshes.sort_by_key(|mesh| (mesh.block_id, face_sort_key(mesh.face), mesh.casts_shadow));
    meshes
}

#[derive(Clone, Copy)]
struct CellVisual {
    tint: [f32; 3],
    block_srgb: [f32; 3],
}

fn visual_for_cell<F>(
    cache: &mut [Option<CellVisual>],
    chunk: &VoxelChunk,
    [x, y, z]: [usize; 3],
    world_voxel: IVec3,
    cell: VoxelCell,
    block: &BlockDefinition,
    tint_at: &mut F,
) -> CellVisual
where
    F: FnMut(IVec3, VoxelCell, &BlockDefinition) -> [f32; 3],
{
    let index = x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE;
    if let Some(visual) = cache[index] {
        return visual;
    }

    let visual = CellVisual {
        tint: if block.textures.is_empty() {
            [1.0, 1.0, 1.0]
        } else {
            tint_at(world_voxel, cell, block)
        },
        block_srgb: surface_block_srgb(
            chunk.light_at(x as i32, y as i32, z as i32),
            block.light_emission > 0,
        ),
    };
    cache[index] = Some(visual);
    visual
}

#[derive(Clone, Copy, PartialEq)]
struct GreedyFace {
    block_id: &'static str,
    material_face: BlockFace,
    tint: [f32; 3],
    lighting: FaceLighting,
}

fn lighting_is_uniform(lighting: FaceLighting) -> bool {
    lighting.channels[1..]
        .iter()
        .all(|value| *value == lighting.channels[0])
        && lighting.block_srgb[1..]
            .iter()
            .all(|value| *value == lighting.block_srgb[0])
        && lighting.ambient_occlusion[1..]
            .iter()
            .all(|value| *value == lighting.ambient_occlusion[0])
}

fn face_cell(face: BlockFace, depth: usize, u: usize, v: usize) -> [usize; 3] {
    match face {
        BlockFace::Right | BlockFace::Left => [depth, v, u],
        BlockFace::Top | BlockFace::Bottom => [u, depth, v],
        BlockFace::Front | BlockFace::Back => [u, v, depth],
    }
}

fn emit_greedy_plane<'a>(
    face: BlockFace,
    depth: usize,
    mask: &mut [Option<GreedyFace>; CHUNK_SIZE * CHUNK_SIZE],
    block_lookup: &mut BlockLookup<'a>,
    buffers: &mut MicroMeshBuffers<'a>,
) {
    for v in 0..CHUNK_SIZE {
        for u in 0..CHUNK_SIZE {
            let index = u + v * CHUNK_SIZE;
            let Some(candidate) = mask[index] else {
                continue;
            };

            let mut width = 1;
            while u + width < CHUNK_SIZE
                && mask[u + width + v * CHUNK_SIZE] == Some(candidate)
            {
                width += 1;
            }

            let mut height = 1;
            while v + height < CHUNK_SIZE
                && (u..u + width).all(|column| {
                    mask[column + (v + height) * CHUNK_SIZE] == Some(candidate)
                })
            {
                height += 1;
            }

            for row in v..v + height {
                for column in u..u + width {
                    mask[column + row * CHUNK_SIZE] = None;
                }
            }

            let block = block_lookup.get(candidate.block_id);
            push_lit_quad(
                material_buffer(
                    buffers,
                    candidate.block_id,
                    block,
                    candidate.material_face,
                ),
                greedy_vertices(face, depth, u, v, width, height),
                face.normal(),
                tiled_uvs(width, height),
                candidate.tint,
                candidate.lighting,
            );
        }
    }
}

fn greedy_vertices(
    face: BlockFace,
    depth: usize,
    u: usize,
    v: usize,
    width: usize,
    height: usize,
) -> [[f32; 3]; 4] {
    let d = depth as f32;
    let u0 = u as f32;
    let v0 = v as f32;
    let u1 = (u + width) as f32;
    let v1 = (v + height) as f32;

    match face {
        BlockFace::Right => [
            [d + 1.0, v0, u1],
            [d + 1.0, v0, u0],
            [d + 1.0, v1, u0],
            [d + 1.0, v1, u1],
        ],
        BlockFace::Left => [
            [d, v0, u0],
            [d, v0, u1],
            [d, v1, u1],
            [d, v1, u0],
        ],
        BlockFace::Top => [
            [u0, d + 1.0, v1],
            [u1, d + 1.0, v1],
            [u1, d + 1.0, v0],
            [u0, d + 1.0, v0],
        ],
        BlockFace::Bottom => [
            [u0, d, v0],
            [u1, d, v0],
            [u1, d, v1],
            [u0, d, v1],
        ],
        BlockFace::Front => [
            [u0, v0, d + 1.0],
            [u1, v0, d + 1.0],
            [u1, v1, d + 1.0],
            [u0, v1, d + 1.0],
        ],
        BlockFace::Back => [
            [u1, v0, d],
            [u0, v0, d],
            [u0, v1, d],
            [u1, v1, d],
        ],
    }
}

fn tiled_uvs(width: usize, height: usize) -> [[f32; 2]; 4] {
    let width = width as f32;
    let height = height as f32;
    [[0.0, height], [width, height], [width, 0.0], [0.0, 0.0]]
}

fn face_sort_key(face: BlockFace) -> u8 {
    match face {
        BlockFace::Right => 0,
        BlockFace::Left => 1,
        BlockFace::Top => 2,
        BlockFace::Bottom => 3,
        BlockFace::Front => 4,
        BlockFace::Back => 5,
    }
}

fn face_uses_texture_rotation(rotations: BlockTextureRotations, face: BlockFace) -> bool {
    match face {
        BlockFace::Right => rotations.right,
        BlockFace::Left => rotations.left,
        BlockFace::Top => rotations.top,
        BlockFace::Bottom => rotations.bottom,
        BlockFace::Front => rotations.front,
        BlockFace::Back => rotations.back,
    }
}
