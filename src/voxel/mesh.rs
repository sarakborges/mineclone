use bevy::prelude::*;
use smallvec::SmallVec;

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
    block_face::{BlockFace, BlockFaces},
    cell::VoxelCell,
    chunk::{CHUNK_SIZE, VoxelChunk},
    mesh_lighting::{
        ChunkLightingCache, FaceLighting, face_lighting_with_cache, push_lit_quad,
        surface_block_srgb_with_cache,
    },
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

pub struct ChunkFaceMesh {
    pub(crate) batch: ChunkTerrainBatch,
    pub mesh: Mesh,
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_chunk_meshlets<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    texture_table: &TerrainTextureTable,
    meshlets: ChunkMeshletMask,
    lighting_cache: Option<&ChunkLightingCache>,
    mut tint_at: F,
) -> Vec<ChunkFaceMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, VoxelCell, &crate::content::block::BlockDefinition) -> [f32; 3],
{
    let mut buffers = MicroMeshBuffers::default();
    let mut block_lookup = BlockLookup::new(blocks);
    let chunk_origin = chunk_coord * CHUNK_SIZE as i32;
    let active_capacity = chunk
        .block_count()
        .min(meshlets.selected_voxel_count());
    let mut active_sources = Vec::<VoxelMeshSource>::with_capacity(active_capacity);
    let mut active_visuals = Vec::<Option<CellVisual>>::with_capacity(active_capacity);
    let mut block_visual_indices =
        SmallVec::<[((usize, usize), usize); 16]>::new();
    let mut block_visuals = Vec::<BlockMeshVisual>::new();
    let plane_capacity = active_capacity.div_ceil(CHUNK_SIZE).max(4);
    let mut active_by_x: [Vec<u32>; CHUNK_SIZE] =
        std::array::from_fn(|_| Vec::with_capacity(plane_capacity));
    let mut active_by_y: [Vec<u32>; CHUNK_SIZE] =
        std::array::from_fn(|_| Vec::with_capacity(plane_capacity));
    let mut active_by_z: [Vec<u32>; CHUNK_SIZE] =
        std::array::from_fn(|_| Vec::with_capacity(plane_capacity));

    // Resolve selected chunk cells and definitions once. Sparse chunks iterate
    // only occupied palette positions; dense chunks keep the straight meshlet
    // scan to avoid bitset iteration overhead.
    let mut collect_source = |x: usize, y: usize, z: usize, cell: &VoxelCell| {
        let block = block_lookup.get(cell.block_id);
        let block_key = (cell.block_id.as_ptr() as usize, cell.block_id.len());
        let block_visual_index = if let Some((_, index)) = block_visual_indices
            .iter()
            .find(|(candidate, _)| *candidate == block_key)
        {
            *index
        } else {
            let index = block_visuals.len();
            block_visuals.push(BlockMeshVisual::new(
                cell.block_id,
                block,
                texture_table,
            ));
            block_visual_indices.push((block_key, index));
            index
        };
        let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
        let world_voxel = chunk_origin + local_voxel;

        if !MicroblockMask::is_modified(*cell) {
            let source_index = active_sources.len();
            debug_assert!(source_index < CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE);
            active_sources.push(VoxelMeshSource {
                cell: *cell,
                block_visual_index: u16::try_from(block_visual_index)
                    .expect("chunk block visual index must fit in u16"),
            });
            active_visuals.push(None);
            let active = pack_active_voxel(x, y, z, source_index);
            active_by_x[x].push(active);
            active_by_y[y].push(active);
            active_by_z[z].push(active);
            return;
        }

        let visual = compute_cell_visual(
            lighting_cache,
            chunk,
            [x, y, z],
            world_voxel,
            *cell,
            block,
            &mut tint_at,
        );
        let surface = MicroSurface {
            world,
            lighting_cache,
            cell: *cell,
            block,
            world_voxel,
            local_voxel,
            tint: visual.tint,
            block_srgb: visual.block_srgb,
            texture_table,
        };
        emit_sculpted_faces(&surface, &mut block_lookup, &mut buffers);
    };

    let selected_voxel_count = meshlets.selected_voxel_count();
    if chunk.block_count() * 2 < selected_voxel_count {
        chunk.visit_block_voxels(|x, y, z, cell| {
            if meshlets.contains_voxel(x, y, z) {
                collect_source(x, y, z, cell);
            }
        });
    } else {
        meshlets.for_each_voxel(|x, y, z| {
            if let Some(cell) = chunk.cell_ref_at(x as i32, y as i32, z as i32) {
                collect_source(x, y, z, cell);
            }
        });
    }

    // Normal voxels are processed face-by-face so compatible exposed faces can
    // be merged into large rectangles. Only occupied, non-sculpted voxels are
    // revisited here; sparse tree/foliage chunks no longer scan 4096 empty cells
    // for each of the six face directions.
    for face in BlockFace::ALL {
        for depth in 0..CHUNK_SIZE {
            let active_voxels = match face {
                BlockFace::Right | BlockFace::Left => &active_by_x[depth],
                BlockFace::Top | BlockFace::Bottom => &active_by_y[depth],
                BlockFace::Front | BlockFace::Back => &active_by_z[depth],
            };
            if active_voxels.is_empty() {
                continue;
            }

            let mut greedy = [None; CHUNK_SIZE * CHUNK_SIZE];
            for &packed in active_voxels {
                let (x, y, z, source_index) = unpack_active_voxel(packed);
                let source = active_sources[source_index];
                let (u, v) = face_uv(face, x, y, z);
                let cell = source.cell;
                let block_visual =
                    &block_visuals[usize::from(source.block_visual_index)];
                let block = block_visual.block;
                let block_is_transparent = block_visual.is_transparent;
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

                let visual = visual_for_active_cell(
                    &mut active_visuals,
                    source_index,
                    lighting_cache,
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
                        lighting_cache,
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

                let face_visual = block_visual.faces.get(source_face);
                let texture_rotation = if face_visual.uses_texture_rotation {
                    cell.texture_rotation
                } else {
                    TextureRotation::default()
                };
                let lighting = face_lighting_with_cache(
                    lighting_cache,
                    world,
                    world_voxel,
                    face,
                    source_block_srgb,
                );
                let material_face = face_visual.material_face;
                let material_code = face_visual.material_code;

                let uv_rotation = oriented_face_uv_rotation(
                    face,
                    cell.orientation,
                    texture_rotation,
                );
                // Alpha-cutout is order-independent and safe to merge.
                // Only true alpha blending must keep independent quads. Greedy
                // lighting is accepted only when all four quantized vertices
                // are identical, so the large FaceLighting payload can be
                // represented by one compact packed value in the plane mask.
                if !block_visual.alpha_blend
                    && let Some(packed_lighting) =
                        pack_uniform_greedy_lighting(lighting)
                {
                    greedy[u + v * CHUNK_SIZE] = Some(GreedyFace {
                        block_visual_index: source.block_visual_index,
                        material_face,
                        tint: pack_greedy_tint(tint),
                        lighting: packed_lighting,
                        material_code: material_code as u32,
                        uv_rotation,
                    });
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

            emit_greedy_plane(
                face,
                depth,
                &mut greedy,
                &block_visuals,
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
struct VoxelMeshSource {
    cell: VoxelCell,
    block_visual_index: u16,
}

#[derive(Clone, Copy)]
struct BlockFaceMeshVisual {
    material_face: BlockFace,
    material_code: f32,
    uses_texture_rotation: bool,
}

struct BlockMeshVisual<'a> {
    block_id: &'static str,
    block: &'a BlockDefinition,
    faces: BlockFaces<BlockFaceMeshVisual>,
    is_transparent: bool,
    alpha_blend: bool,
}

impl<'a> BlockMeshVisual<'a> {
    fn new(
        block_id: &'static str,
        block: &'a BlockDefinition,
        texture_table: &TerrainTextureTable,
    ) -> Self {
        Self {
            block_id,
            block,
            faces: BlockFaces::from_fn(|face| {
                let material_face = block_face_material_face(face, block);
                BlockFaceMeshVisual {
                    material_face,
                    material_code: texture_table
                        .encoded_layers(block_face_texture_layers(material_face, block))
                        .unwrap_or(0.0),
                    uses_texture_rotation: face_uses_texture_rotation(
                        block.rotate_texture,
                        face,
                    ),
                }
            }),
            is_transparent: block.alpha_blend || block.alpha_cutoff.is_some(),
            alpha_blend: block.alpha_blend,
        }
    }
}

#[derive(Clone, Copy)]
struct CellVisual {
    tint: [f32; 3],
    block_srgb: [f32; 3],
}

#[allow(clippy::too_many_arguments)]
fn visual_for_active_cell<F>(
    cache: &mut [Option<CellVisual>],
    index: usize,
    lighting_cache: Option<&ChunkLightingCache>,
    chunk: &VoxelChunk,
    local: [usize; 3],
    world_voxel: IVec3,
    cell: VoxelCell,
    block: &BlockDefinition,
    tint_at: &mut F,
) -> CellVisual
where
    F: FnMut(IVec3, VoxelCell, &BlockDefinition) -> [f32; 3],
{
    if let Some(visual) = cache[index] {
        return visual;
    }

    let visual = compute_cell_visual(
        lighting_cache,
        chunk,
        local,
        world_voxel,
        cell,
        block,
        tint_at,
    );
    cache[index] = Some(visual);
    visual
}

fn compute_cell_visual<F>(
    lighting_cache: Option<&ChunkLightingCache>,
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
    CellVisual {
        tint: if block.textures.is_empty() {
            [1.0, 1.0, 1.0]
        } else {
            tint_at(world_voxel, cell, block)
        },
        block_srgb: surface_block_srgb_with_cache(
            lighting_cache,
            world_voxel,
            chunk.light_at(x as i32, y as i32, z as i32),
            block.light_emission > 0,
        ),
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct GreedyFace {
    block_visual_index: u16,
    material_face: BlockFace,
    tint: u32,
    lighting: u64,
    material_code: u32,
    uv_rotation: TextureRotation,
}

fn pack_greedy_tint(tint: [f32; 3]) -> u32 {
    let tint = tint.map(|channel| quantize_to_u32(channel, 127));
    tint[0] | (tint[1] << 7) | (tint[2] << 14)
}

fn unpack_greedy_tint(packed: u32) -> [f32; 3] {
    [
        (packed & 127) as f32 / 127.0,
        ((packed >> 7) & 127) as f32 / 127.0,
        ((packed >> 14) & 127) as f32 / 127.0,
    ]
}

fn pack_uniform_greedy_lighting(lighting: FaceLighting) -> Option<u64> {
    let packed = std::array::from_fn::<u64, 4, _>(|index| {
        let sky = quantize_to_u64(lighting.channels[index][0], 15);
        let block = lighting.block_srgb[index]
            .map(|channel| quantize_to_u64(channel, 255));
        let ao = quantize_to_u64(lighting.ambient_occlusion[index], 255);

        sky
            | (block[0] << 4)
            | (block[1] << 12)
            | (block[2] << 20)
            | (ao << 28)
    });

    packed[1..]
        .iter()
        .all(|candidate| *candidate == packed[0])
        .then_some(packed[0])
}

fn unpack_greedy_lighting(packed: u64) -> FaceLighting {
    let sky = (packed & 15) as f32 / 15.0;
    let block = [
        ((packed >> 4) & 255) as f32 / 255.0,
        ((packed >> 12) & 255) as f32 / 255.0,
        ((packed >> 20) & 255) as f32 / 255.0,
    ];
    let ao = ((packed >> 28) & 255) as f32 / 255.0;

    FaceLighting {
        channels: [[sky, 0.0]; 4],
        block_srgb: [block; 4],
        ambient_occlusion: [ao; 4],
    }
}

fn quantize_to_u32(value: f32, steps: u32) -> u32 {
    (value.clamp(0.0, 1.0) * steps as f32).round() as u32
}

fn quantize_to_u64(value: f32, steps: u64) -> u64 {
    (value.clamp(0.0, 1.0) * steps as f32).round() as u64
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

fn pack_active_voxel(
    x: usize,
    y: usize,
    z: usize,
    source_index: usize,
) -> u32 {
    debug_assert!(x < 16 && y < 16 && z < 16);
    debug_assert!(source_index < 4096);
    x as u32
        | ((y as u32) << 4)
        | ((z as u32) << 8)
        | ((source_index as u32) << 12)
}

fn unpack_active_voxel(packed: u32) -> (usize, usize, usize, usize) {
    (
        (packed & 0x0f) as usize,
        ((packed >> 4) & 0x0f) as usize,
        ((packed >> 8) & 0x0f) as usize,
        (packed >> 12) as usize,
    )
}

fn face_uv(face: BlockFace, x: usize, y: usize, z: usize) -> (usize, usize) {
    match face {
        BlockFace::Right | BlockFace::Left => (z, y),
        BlockFace::Top | BlockFace::Bottom => (x, z),
        BlockFace::Front | BlockFace::Back => (x, y),
    }
}

fn emit_greedy_plane<'a>(
    face: BlockFace,
    depth: usize,
    mask: &mut [Option<GreedyFace>; CHUNK_SIZE * CHUNK_SIZE],
    block_visuals: &[BlockMeshVisual<'a>],
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

            let block_visual =
                &block_visuals[usize::from(candidate.block_visual_index)];
            push_lit_quad(
                material_buffer(
                    buffers,
                    block_visual.block_id,
                    block_visual.block,
                    candidate.material_face,
                ),
                greedy_vertices(face, depth, u, v, width, height),
                face.normal(),
                tiled_uvs(width, height, candidate.uv_rotation),
                unpack_greedy_tint(candidate.tint),
                unpack_greedy_lighting(candidate.lighting),
                candidate.material_code as f32,
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

fn tiled_uvs(
    width: usize,
    height: usize,
    rotation: TextureRotation,
) -> [[f32; 2]; 4] {
    let (u_extent, v_extent) = match rotation {
        TextureRotation::Degrees0 | TextureRotation::Degrees180 => {
            (width as f32, height as f32)
        }
        TextureRotation::Degrees90 | TextureRotation::Degrees270 => {
            (height as f32, width as f32)
        }
    };
    rotation.rotate_uvs([
        [0.0, v_extent],
        [u_extent, v_extent],
        [u_extent, 0.0],
        [0.0, 0.0],
    ])
}

fn oriented_face_uv_rotation(
    face: BlockFace,
    orientation: BlockOrientation,
    texture_rotation: TextureRotation,
) -> TextureRotation {
    let orientation_turn = match orientation {
        BlockOrientation::Y => 0,
        BlockOrientation::Z => match face {
            BlockFace::Right => 1,
            BlockFace::Left => 3,
            BlockFace::Top | BlockFace::Back => 2,
            BlockFace::Bottom | BlockFace::Front => 0,
        },
        BlockOrientation::X => match face {
            BlockFace::Back => 1,
            BlockFace::Right
            | BlockFace::Left
            | BlockFace::Top
            | BlockFace::Bottom
            | BlockFace::Front => 3,
        },
    };

    TextureRotation::from_quarter_turn(
        orientation_turn + texture_rotation as u8,
    )
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
