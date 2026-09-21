use bevy::prelude::*;

use crate::{
    content::{
        block::{BlockDefinition, BlockLookup, BlockRegistry, BlockTextureRotations},
        block_orientation::BlockOrientation,
    },
    rendering::block_texture::{
        TerrainTextureTable, block_face_material_face, block_face_texture_layers,
    },
};

use self::{
    geometry::{face_geometry, is_face_exposed_against_neighbor, orient_face_geometry},
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
    meshlet::{CHUNK_MESHLET_EDGE, ChunkMeshletMask},
    microblock::MicroblockMask,
    orientation::source_face_for_oriented_face,
    quad::VOXEL_FACE_UVS,
    read::VoxelRead,
    texture_rotation::TextureRotation,
};

pub(crate) mod geometry;
mod micro_mesh;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChunkTerrainBatch {
    Array {
        alpha_cutoff: Option<u32>,
        alpha_blend: bool,
        casts_shadow: bool,
    },
    Legacy {
        block_id: &'static str,
        face: BlockFace,
        casts_shadow: bool,
    },
}

impl ChunkTerrainBatch {
    pub(crate) fn casts_shadow(self) -> bool {
        match self {
            Self::Array { casts_shadow, .. } | Self::Legacy { casts_shadow, .. } => casts_shadow,
        }
    }
}

pub struct ChunkFaceMesh {
    pub(crate) batch: ChunkTerrainBatch,
    pub mesh: Mesh,
}

pub fn build_chunk_mesh<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    texture_table: &TerrainTextureTable,
    tint_at: F,
) -> Vec<ChunkFaceMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, VoxelCell, &crate::content::block::BlockDefinition) -> [f32; 3],
{
    build_chunk_meshlets(
        world,
        chunk_coord,
        chunk,
        blocks,
        texture_table,
        ChunkMeshletMask::ALL,
        tint_at,
    )
}

pub(crate) fn build_chunk_meshlets<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    texture_table: &TerrainTextureTable,
    meshlets: ChunkMeshletMask,
    mut tint_at: F,
) -> Vec<ChunkFaceMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, VoxelCell, &crate::content::block::BlockDefinition) -> [f32; 3],
{
    let mut buffers = MicroMeshBuffers::default();
    let mut block_lookup = BlockLookup::new(blocks);
    let chunk_origin = chunk_coord * CHUNK_SIZE as i32;
    let selected_voxel_count = meshlets.selected_voxel_count();
    let mut visuals = vec![None; selected_voxel_count];
    let mut sources = vec![None; selected_voxel_count];

    // Resolve selected chunk cells and definitions once. The six directional
    // meshing passes reuse these entries instead of re-reading storage and
    // searching the block registry for every face.
    meshlets.for_each_voxel(|x, y, z| {
        let Some(cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
            return;
        };
        let block = block_lookup.get(cell.block_id);
        let index = meshlets
            .compact_voxel_index(x, y, z)
            .expect("selected voxel must have a compact meshlet index");
        sources[index] = Some(VoxelMeshSource { cell, block });

        if !MicroblockMask::is_modified(cell) {
            return;
        }

        let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
        let world_voxel = chunk_origin + local_voxel;
        let visual = visual_for_cell(
            &mut visuals,
            meshlets,
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
            texture_table,
        };
        emit_sculpted_faces(&surface, &mut block_lookup, &mut buffers);
    });

    // Normal voxels are processed face-by-face so compatible exposed faces can
    // be merged into large rectangles. Partial remeshes only visit the selected
    // 8³ regions on each face plane.
    for face in BlockFace::ALL {
        for depth in 0..CHUNK_SIZE {
            let mut greedy = [None; CHUNK_SIZE * CHUNK_SIZE];

            for v_region in 0..CHUNK_SIZE / CHUNK_MESHLET_EDGE {
                for u_region in 0..CHUNK_SIZE / CHUNK_MESHLET_EDGE {
                    let u_start = u_region * CHUNK_MESHLET_EDGE;
                    let v_start = v_region * CHUNK_MESHLET_EDGE;
                    let [probe_x, probe_y, probe_z] =
                        face_cell(face, depth, u_start, v_start);
                    if !meshlets.contains_voxel(probe_x, probe_y, probe_z) {
                        continue;
                    }

                    for v in v_start..v_start + CHUNK_MESHLET_EDGE {
                        for u in u_start..u_start + CHUNK_MESHLET_EDGE {
                            let [x, y, z] = face_cell(face, depth, u, v);
                            let source_index = meshlets
                                .compact_voxel_index(x, y, z)
                                .expect("face pass only visits selected meshlets");
                            let Some(source) = sources[source_index] else {
                                continue;
                            };
                            let cell = source.cell;
                            let block = source.block;
                            if MicroblockMask::is_modified(cell) {
                                continue;
                            }

                            let block_is_transparent =
                                block.alpha_blend || block.alpha_cutoff.is_some();
                            let local_voxel =
                                IVec3::new(x as i32, y as i32, z as i32);
                            let world_voxel = chunk_origin + local_voxel;
                            let source_face = if cell.orientation == BlockOrientation::Y {
                                face
                            } else {
                                source_face_for_oriented_face(face, cell.orientation)
                            };
                            let neighbor_cell = face_neighbor_cell(
                                world,
                                chunk,
                                local_voxel,
                                world_voxel,
                                face,
                            );
                            let partial_occluder = neighbor_cell
                                .filter(|neighbor| MicroblockMask::is_modified(*neighbor))
                                .filter(|neighbor| {
                                    let definition = block_lookup.get(neighbor.block_id);
                                    occludes(
                                        cell.block_id,
                                        block,
                                        neighbor.block_id,
                                        definition,
                                    )
                                });

                            if partial_occluder.is_none()
                                && !is_face_exposed_against_neighbor(
                                    &mut block_lookup,
                                    cell.block_id,
                                    block_is_transparent,
                                    world_voxel,
                                    face,
                                    neighbor_cell,
                                )
                            {
                                continue;
                            }

                            let visual = visual_for_cell(
                                &mut visuals,
                                meshlets,
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
                                    texture_table,
                                };
                                emit_neighbor_openings(
                                    &surface,
                                    &mut buffers,
                                    face,
                                    neighbor,
                                );
                                continue;
                            }

                            let texture_rotation =
                                if face_uses_texture_rotation(block.rotate_texture, source_face) {
                                    cell.texture_rotation
                                } else {
                                    TextureRotation::default()
                                };
                            let lighting =
                                face_lighting(world, world_voxel, face, source_block_srgb);
                            let material_face =
                                block_face_material_face(source_face, block);
                            let material_code = texture_table
                                .encoded_layers(block_face_texture_layers(
                                    material_face,
                                    block,
                                ))
                                .unwrap_or(0.0);

                            let candidate = GreedyFace {
                                block_id: cell.block_id,
                                material_face,
                                tint,
                                lighting,
                                material_code,
                            };
                            let greedy_eligible = cell.orientation == BlockOrientation::Y
                                // Alpha-cutout is order-independent and safe to merge.
                                // Only true alpha blending must keep independent quads.
                                && !block.alpha_blend
                                && texture_rotation == TextureRotation::Degrees0
                                && lighting_is_uniform(lighting);

                            if greedy_eligible {
                                greedy[u + v * CHUNK_SIZE] = Some(candidate);
                                continue;
                            }

                            let geometry = orient_face_geometry(
                                face_geometry(
                                    source_face,
                                    x,
                                    y,
                                    z,
                                    texture_rotation,
                                ),
                                cell.orientation,
                                x,
                                y,
                                z,
                            );
                            push_lit_quad(
                                material_buffer(
                                    &mut buffers,
                                    cell.block_id,
                                    block,
                                    material_face,
                                ),
                                geometry.vertices,
                                geometry.normal,
                                geometry.texture_rotation.rotate_uvs(VOXEL_FACE_UVS),
                                tint,
                                lighting,
                                material_code,
                            );
                        }
                    }
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
                batch: entry.batch,
                mesh,
            })
        })
        .collect::<Vec<_>>();

    meshes.sort_by_key(|mesh| terrain_batch_sort_key(mesh.batch));
    meshes
}

#[derive(Clone, Copy)]
struct VoxelMeshSource<'a> {
    cell: VoxelCell,
    block: &'a BlockDefinition,
}

#[derive(Clone, Copy)]
struct CellVisual {
    tint: [f32; 3],
    block_srgb: [f32; 3],
}

fn visual_for_cell<F>(
    cache: &mut [Option<CellVisual>],
    meshlets: ChunkMeshletMask,
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
    let index = meshlets
        .compact_voxel_index(x, y, z)
        .expect("visual cache only receives voxels from selected meshlets");
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
    material_code: f32,
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

fn face_neighbor_cell<W: VoxelRead + ?Sized>(
    world: &W,
    chunk: &VoxelChunk,
    local_voxel: IVec3,
    world_voxel: IVec3,
    face: BlockFace,
) -> Option<VoxelCell> {
    let local = local_voxel + face.offset();
    if local.x >= 0
        && local.y >= 0
        && local.z >= 0
        && local.x < CHUNK_SIZE as i32
        && local.y < CHUNK_SIZE as i32
        && local.z < CHUNK_SIZE as i32
    {
        chunk.cell_at(local.x, local.y, local.z)
    } else {
        world.cell_at(world_voxel + face.offset())
    }
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

            let u_limit = ((u / CHUNK_MESHLET_EDGE) + 1) * CHUNK_MESHLET_EDGE;
            let v_limit = ((v / CHUNK_MESHLET_EDGE) + 1) * CHUNK_MESHLET_EDGE;

            let mut width = 1;
            while u + width < u_limit
                && mask[u + width + v * CHUNK_SIZE] == Some(candidate)
            {
                width += 1;
            }

            let mut height = 1;
            while v + height < v_limit
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
                candidate.material_code,
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

fn terrain_batch_sort_key(
    batch: ChunkTerrainBatch,
) -> (u8, u8, u32, bool, &'static str, u8, bool) {
    match batch {
        ChunkTerrainBatch::Array {
            alpha_cutoff,
            alpha_blend,
            casts_shadow,
        } => (
            0,
            u8::from(alpha_cutoff.is_some()),
            alpha_cutoff.unwrap_or(0),
            alpha_blend,
            "",
            0,
            casts_shadow,
        ),
        ChunkTerrainBatch::Legacy {
            block_id,
            face,
            casts_shadow,
        } => (
            1,
            0,
            0,
            false,
            block_id,
            face_sort_key(face),
            casts_shadow,
        ),
    }
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
