use bevy::prelude::*;
use smallvec::SmallVec;

use crate::{content::fluid::FluidId, voxel::cell::VoxelCell};

const MIN_FLUID_CELLS_FOR_GREEDY_TOP: usize = 64;
const MIN_EXPOSED_FLUID_TOPS_FOR_HEIGHT_CACHE: usize = 64;
const FLUID_HEIGHT_PLANE_SIDE: usize = CHUNK_SIZE + 2;
const FLUID_HEIGHT_PLANE_AREA: usize = FLUID_HEIGHT_PLANE_SIDE * FLUID_HEIGHT_PLANE_SIDE;

use super::{
    block_face::BlockFace,
    chunk::{CHUNK_SIZE, VoxelChunk},
    fluid::FluidCell,
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{
        ChunkLightingCache, FaceLighting, face_lighting_with_cache, push_lit_quad,
        surface_block_srgb_with_cache,
    },
    meshlet::ChunkMeshletMask,
    microblock::{MICROBLOCK_EDGE, MicroblockMask},
    quad::VOXEL_FACE_UVS,
    read::VoxelRead,
};

pub struct ChunkFluidMesh {
    pub fluid_id: FluidId,
    pub mesh: Mesh,
}

#[derive(Clone, Copy, Default)]
struct FluidFaceHeights {
    h00: f32,
    h10: f32,
    h11: f32,
    h01: f32,
}

#[derive(Clone, Copy, PartialEq)]
struct FluidGreedyLighting {
    channels: [f32; 2],
    block_srgb: [f32; 3],
    ambient_occlusion: f32,
}

impl FluidGreedyLighting {
    fn from_uniform(lighting: FaceLighting) -> Option<Self> {
        (1..4)
            .all(|index| {
                lighting.channels[index] == lighting.channels[0]
                    && lighting.block_srgb[index] == lighting.block_srgb[0]
                    && lighting.ambient_occlusion[index] == lighting.ambient_occlusion[0]
            })
            .then_some(Self {
                channels: lighting.channels[0],
                block_srgb: lighting.block_srgb[0],
                ambient_occlusion: lighting.ambient_occlusion[0],
            })
    }

    fn expand(self) -> FaceLighting {
        FaceLighting {
            channels: [self.channels; 4],
            block_srgb: [self.block_srgb; 4],
            ambient_occlusion: [self.ambient_occlusion; 4],
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct FluidGreedyTop {
    fluid_id: FluidId,
    tint: [f32; 3],
    lighting: FluidGreedyLighting,
    height: f32,
}

#[derive(Clone, Copy)]
struct FluidTopSource {
    x: u8,
    z: u8,
    cell: FluidCell,
}

#[derive(Clone, Copy)]
struct ExposedFluidTop {
    source: FluidTopSource,
    top_sample: FluidNeighborContent,
}

struct FluidHeightPlaneCache {
    current: [Option<FluidCell>; FLUID_HEIGHT_PLANE_AREA],
    above: [Option<FluidCell>; FLUID_HEIGHT_PLANE_AREA],
}

impl FluidHeightPlaneCache {
    fn capture<W: VoxelRead + ?Sized>(
        world: &W,
        chunk: &VoxelChunk,
        chunk_origin: IVec3,
        y: usize,
    ) -> Self {
        let mut current = [None; FLUID_HEIGHT_PLANE_AREA];
        let mut above = [None; FLUID_HEIGHT_PLANE_AREA];

        for cache_z in 0..FLUID_HEIGHT_PLANE_SIDE {
            for cache_x in 0..FLUID_HEIGHT_PLANE_SIDE {
                let local = IVec3::new(
                    cache_x as i32 - 1,
                    y as i32,
                    cache_z as i32 - 1,
                );
                let world_position = chunk_origin + local;
                let index = cache_x + cache_z * FLUID_HEIGHT_PLANE_SIDE;
                current[index] =
                    fluid_at_local_or_world(world, chunk, local, world_position);
                above[index] = fluid_at_local_or_world(
                    world,
                    chunk,
                    local + IVec3::Y,
                    world_position + IVec3::Y,
                );
            }
        }

        Self { current, above }
    }

    fn heights_at(&self, x: usize, z: usize, fluid_id: FluidId) -> FluidFaceHeights {
        FluidFaceHeights {
            h00: self.corner_height(x, z, fluid_id, 0, 0),
            h10: self.corner_height(x, z, fluid_id, 2, 0),
            h11: self.corner_height(x, z, fluid_id, 2, 2),
            h01: self.corner_height(x, z, fluid_id, 0, 2),
        }
    }

    fn corner_height(
        &self,
        x: usize,
        z: usize,
        fluid_id: FluidId,
        x_offset: usize,
        z_offset: usize,
    ) -> f32 {
        let positions = [(1, 1), (x_offset, 1), (1, z_offset), (x_offset, z_offset)];

        if positions.iter().any(|&(dx, dz)| {
            let index = (x + dx) + (z + dz) * FLUID_HEIGHT_PLANE_SIDE;
            self.above[index].is_some_and(|cell| cell.fluid_id == fluid_id)
        }) {
            return 1.0;
        }

        let mut total = 0.0;
        let mut count = 0.0;
        for (dx, dz) in positions {
            let index = (x + dx) + (z + dz) * FLUID_HEIGHT_PLANE_SIDE;
            if let Some(cell) = self.current[index].filter(|cell| cell.fluid_id == fluid_id) {
                total += cell.height();
                count += 1.0;
            }
        }

        if count > 0.0 { total / count } else { 0.0 }
    }
}

struct FluidTopPlanes {
    offsets: [usize; CHUNK_SIZE + 1],
    sources: Vec<FluidTopSource>,
}

impl FluidTopPlanes {
    fn collect(chunk: &VoxelChunk, meshlets: ChunkMeshletMask) -> Self {
        let active_capacity = chunk
            .fluid_count()
            .min(meshlets.selected_voxel_count());
        let mut sources = Vec::with_capacity(active_capacity);
        let mut counts = [0_usize; CHUNK_SIZE];

        // Palette occupancy is indexed x + z * size + y * area, so occupied
        // iteration is already stable and grouped by ascending Y.
        chunk.visit_fluid_voxels(|x, y, z, cell| {
            if meshlets.contains_voxel(x, y, z) {
                counts[y] += 1;
                sources.push(FluidTopSource {
                    x: x as u8,
                    z: z as u8,
                    cell,
                });
            }
        });

        let mut offsets = [0_usize; CHUNK_SIZE + 1];
        for (index, count) in counts.into_iter().enumerate() {
            offsets[index + 1] = offsets[index] + count;
        }

        Self { offsets, sources }
    }

    fn plane(&self, y: usize) -> &[FluidTopSource] {
        &self.sources[self.offsets[y]..self.offsets[y + 1]]
    }
}

pub(crate) fn build_fluid_meshlets<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    meshlets: ChunkMeshletMask,
    lighting_cache: Option<&ChunkLightingCache>,
    mut tint_at: F,
) -> Vec<ChunkFluidMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, FluidId) -> [f32; 3],
{
    let mut buffers = SmallVec::<[(FluidId, VoxelMeshBuffer); 2]>::new();
    let chunk_size = CHUNK_SIZE as i32;
    let chunk_origin = chunk_coord * chunk_size;

    let greedy_top = chunk.fluid_count() >= MIN_FLUID_CELLS_FOR_GREEDY_TOP;
    if greedy_top {
        emit_greedy_fluid_top_faces(
            world,
            chunk,
            chunk_origin,
            meshlets,
            lighting_cache,
            &mut tint_at,
            &mut buffers,
        );
    }

    let mut emit_fluid_voxel = |x: usize, y: usize, z: usize, cell: FluidCell| {
        let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
        let world_voxel = chunk_origin + local_voxel;
        let neighbor_samples = BlockFace::ALL.map(|face| {
            if greedy_top && face == BlockFace::Top {
                None
            } else {
                fluid_neighbor_content(
                    world,
                    chunk,
                    local_voxel,
                    world_voxel,
                    face,
                )
            }
        });
        let exposed: [bool; 6] = std::array::from_fn(|index| {
            let face = BlockFace::ALL[index];
            if (greedy_top && face == BlockFace::Top)
                || (face == BlockFace::Bottom && world_voxel.y <= 0)
            {
                return false;
            }
            fluid_face_is_exposed(
                neighbor_samples[index],
                cell.fluid_id,
                face,
            )
        });
        if !exposed.iter().any(|value| *value) {
            return;
        }

        let tint = tint_at(world_voxel, cell.fluid_id);
        let mut heights = None::<FluidFaceHeights>;
        let (source_block, _, source_light) = chunk.sample_local_at(x, y, z);
        let source_block_srgb = surface_block_srgb_with_cache(
            lighting_cache,
            world_voxel,
            source_light,
            false,
        );
        let source_mask = source_block
            .filter(|block| MicroblockMask::has_partial_geometry(*block))
            .map(MicroblockMask::geometry_for_cell);
        let fluid = fluid_buffer(&mut buffers, cell.fluid_id);

        for (index, (face, is_exposed)) in
            BlockFace::ALL.into_iter().zip(exposed).enumerate()
        {
            if !is_exposed {
                continue;
            }

            let lighting =
                face_lighting_with_cache(
                    lighting_cache,
                    world,
                    world_voxel,
                    face,
                    source_block_srgb,
                );
            let face_heights = if fluid_face_needs_heights(face) {
                *heights.get_or_insert_with(|| {
                    fluid_face_heights(
                        world,
                        chunk,
                        local_voxel,
                        world_voxel,
                        cell.fluid_id,
                    )
                })
            } else {
                FluidFaceHeights::default()
            };
            let neighbor_mask = neighbor_samples[index]
                .and_then(|(block, _)| block)
                .filter(|block| MicroblockMask::has_partial_geometry(*block))
                .map(MicroblockMask::geometry_for_cell);

            if source_mask.is_some() || neighbor_mask.is_some() {
                emit_fluid_openings(
                    fluid,
                    face,
                    x as f32,
                    y as f32,
                    z as f32,
                    face_heights,
                    source_mask,
                    neighbor_mask,
                    tint,
                    lighting,
                );
            } else {
                push_lit_quad(
                    fluid,
                    fluid_face_vertices(
                        face,
                        x as f32,
                        y as f32,
                        z as f32,
                        face_heights,
                    ),
                    face.normal(),
                    VOXEL_FACE_UVS,
                    tint,
                    lighting,
                    0.0,
                );
            }
        }
    
    };

    let selected_voxels = meshlets.selected_voxel_count();
    if chunk.fluid_count() * 2 < selected_voxels {
        chunk.visit_fluid_voxels(|x, y, z, cell| {
            if meshlets.contains_voxel(x, y, z) {
                emit_fluid_voxel(x, y, z, cell);
            }
        });
    } else {
        meshlets.for_each_voxel(|x, y, z| {
            if let Some(cell) = chunk.fluid_at_local(x, y, z) {
                emit_fluid_voxel(x, y, z, cell);
            }
        });
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

fn emit_greedy_fluid_top_faces<W, F>(
    world: &W,
    chunk: &VoxelChunk,
    chunk_origin: IVec3,
    meshlets: ChunkMeshletMask,
    lighting_cache: Option<&ChunkLightingCache>,
    tint_at: &mut F,
    buffers: &mut SmallVec<[(FluidId, VoxelMeshBuffer); 2]>,
) where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, FluidId) -> [f32; 3],
{
    let active_by_y = FluidTopPlanes::collect(chunk, meshlets);
    let mut mask = vec![None::<FluidGreedyTop>; CHUNK_SIZE * CHUNK_SIZE];
    let mut exposed_tops = Vec::<ExposedFluidTop>::with_capacity(CHUNK_SIZE * CHUNK_SIZE);

    for y in 0..CHUNK_SIZE {
        let active = active_by_y.plane(y);
        if active.is_empty() {
            continue;
        }

        exposed_tops.clear();
        for &source in active {
            let x = usize::from(source.x);
            let z = usize::from(source.z);
            let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
            let world_voxel = chunk_origin + local_voxel;
            let top_sample = fluid_neighbor_content(
                world,
                chunk,
                local_voxel,
                world_voxel,
                BlockFace::Top,
            );
            if fluid_face_is_exposed(top_sample, source.cell.fluid_id, BlockFace::Top) {
                exposed_tops.push(ExposedFluidTop { source, top_sample });
            }
        }
        if exposed_tops.is_empty() {
            continue;
        }

        let height_cache =
            (exposed_tops.len() >= MIN_EXPOSED_FLUID_TOPS_FOR_HEIGHT_CACHE)
                .then(|| FluidHeightPlaneCache::capture(world, chunk, chunk_origin, y));

        for exposed in &exposed_tops {
            let source = exposed.source;
            let top_sample = exposed.top_sample;
            let x = usize::from(source.x);
            let z = usize::from(source.z);
            let cell = source.cell;
            let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
            let world_voxel = chunk_origin + local_voxel;
            let heights = height_cache.as_ref().map_or_else(
                || {
                    fluid_face_heights(
                        world,
                        chunk,
                        local_voxel,
                        world_voxel,
                        cell.fluid_id,
                    )
                },
                |cache| cache.heights_at(x, z, cell.fluid_id),
            );
            let (source_block, _, source_light) = chunk.sample_local_at(x, y, z);
            let source_block_srgb = surface_block_srgb_with_cache(
                lighting_cache,
                world_voxel,
                source_light,
                false,
            );
            let lighting = face_lighting_with_cache(
                lighting_cache,
                world,
                world_voxel,
                BlockFace::Top,
                source_block_srgb,
            );
            let tint = tint_at(world_voxel, cell.fluid_id);
            let source_mask = source_block
                .filter(|block| MicroblockMask::has_partial_geometry(*block))
                .map(MicroblockMask::geometry_for_cell);
            let neighbor_mask = top_sample
                .and_then(|(block, _)| block)
                .filter(|block| MicroblockMask::has_partial_geometry(*block))
                .map(MicroblockMask::geometry_for_cell);

            if source_mask.is_some() || neighbor_mask.is_some() {
                emit_fluid_openings(
                    fluid_buffer(buffers, cell.fluid_id),
                    BlockFace::Top,
                    x as f32,
                    y as f32,
                    z as f32,
                    heights,
                    source_mask,
                    neighbor_mask,
                    tint,
                    lighting,
                );
                continue;
            }

            let Some(height) = flat_fluid_height(heights) else {
                push_lit_quad(
                    fluid_buffer(buffers, cell.fluid_id),
                    fluid_face_vertices(
                        BlockFace::Top,
                        x as f32,
                        y as f32,
                        z as f32,
                        heights,
                    ),
                    BlockFace::Top.normal(),
                    VOXEL_FACE_UVS,
                    tint,
                    lighting,
                    0.0,
                );
                continue;
            };

            let Some(greedy_lighting) = FluidGreedyLighting::from_uniform(lighting) else {
                push_lit_quad(
                    fluid_buffer(buffers, cell.fluid_id),
                    fluid_face_vertices(
                        BlockFace::Top,
                        x as f32,
                        y as f32,
                        z as f32,
                        heights,
                    ),
                    BlockFace::Top.normal(),
                    VOXEL_FACE_UVS,
                    tint,
                    lighting,
                    0.0,
                );
                continue;
            };

            mask[x + z * CHUNK_SIZE] = Some(FluidGreedyTop {
                fluid_id: cell.fluid_id,
                tint,
                lighting: greedy_lighting,
                height,
            });
        }

        emit_greedy_fluid_top_plane(y, &mut mask, buffers);
        mask.fill(None);
    }
}

fn emit_greedy_fluid_top_plane(
    y: usize,
    mask: &mut [Option<FluidGreedyTop>],
    buffers: &mut SmallVec<[(FluidId, VoxelMeshBuffer); 2]>,
) {
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let index = x + z * CHUNK_SIZE;
            let Some(candidate) = mask[index] else {
                continue;
            };

            let x_limit =
                ((x / crate::voxel::meshlet::CHUNK_MESHLET_EDGE) + 1)
                    * crate::voxel::meshlet::CHUNK_MESHLET_EDGE;
            let z_limit =
                ((z / crate::voxel::meshlet::CHUNK_MESHLET_EDGE) + 1)
                    * crate::voxel::meshlet::CHUNK_MESHLET_EDGE;

            let mut width = 1;
            while x + width < x_limit
                && mask[x + width + z * CHUNK_SIZE] == Some(candidate)
            {
                width += 1;
            }

            let mut depth = 1;
            while z + depth < z_limit
                && (x..x + width).all(|column| {
                    mask[column + (z + depth) * CHUNK_SIZE] == Some(candidate)
                })
            {
                depth += 1;
            }

            for row in z..z + depth {
                for column in x..x + width {
                    mask[column + row * CHUNK_SIZE] = None;
                }
            }

            let x0 = x as f32;
            let x1 = (x + width) as f32;
            let z0 = z as f32;
            let z1 = (z + depth) as f32;
            let surface_y = y as f32 + candidate.height;
            push_lit_quad(
                fluid_buffer(buffers, candidate.fluid_id),
                [
                    [x0, surface_y, z1],
                    [x1, surface_y, z1],
                    [x1, surface_y, z0],
                    [x0, surface_y, z0],
                ],
                BlockFace::Top.normal(),
                [
                    [0.0, depth as f32],
                    [width as f32, depth as f32],
                    [width as f32, 0.0],
                    [0.0, 0.0],
                ],
                candidate.tint,
                candidate.lighting.expand(),
                0.0,
            );
        }
    }
}

fn flat_fluid_height(heights: FluidFaceHeights) -> Option<f32> {
    (heights.h00.to_bits() == heights.h10.to_bits()
        && heights.h00.to_bits() == heights.h11.to_bits()
        && heights.h00.to_bits() == heights.h01.to_bits())
        .then_some(heights.h00)
}

fn fluid_buffer(
    buffers: &mut SmallVec<[(FluidId, VoxelMeshBuffer); 2]>,
    fluid_id: FluidId,
) -> &mut VoxelMeshBuffer {
    if let Some(index) = buffers
        .iter()
        .position(|(candidate, _)| *candidate == fluid_id)
    {
        return &mut buffers[index].1;
    }

    buffers.push((fluid_id, VoxelMeshBuffer::default()));
    &mut buffers
        .last_mut()
        .expect("fluid mesh buffer was just inserted")
        .1
}

#[expect(
    clippy::too_many_arguments,
    reason = "clipped fluid emission needs face bounds, mask, tint, and lighting"
)]
fn emit_fluid_openings(
    buffer: &mut VoxelMeshBuffer,
    face: BlockFace,
    x0: f32,
    y0: f32,
    z0: f32,
    heights: FluidFaceHeights,
    source_mask: Option<MicroblockMask>,
    neighbor_mask: Option<MicroblockMask>,
    tint: [f32; 3],
    lighting: crate::voxel::mesh_lighting::FaceLighting,
) {
    const EDGE: usize = MICROBLOCK_EDGE as usize;
    for v in 0..EDGE {
        for u in 0..EDGE {
            let source_boundary_position = match face {
                BlockFace::Right => [EDGE - 1, v, u],
                BlockFace::Left => [0, v, u],
                BlockFace::Top => [u, EDGE - 1, v],
                BlockFace::Bottom => [u, 0, v],
                BlockFace::Front => [u, v, EDGE - 1],
                BlockFace::Back => [u, v, 0],
            };
            let neighbor_boundary_position = match face {
                BlockFace::Right => [0, v, u],
                BlockFace::Left => [EDGE - 1, v, u],
                BlockFace::Top => [u, 0, v],
                BlockFace::Bottom => [u, EDGE - 1, v],
                BlockFace::Front => [u, v, 0],
                BlockFace::Back => [u, v, EDGE - 1],
            };
            if source_mask.is_some_and(|mask| mask.contains(source_boundary_position))
                || neighbor_mask.is_some_and(|mask| mask.contains(neighbor_boundary_position))
            {
                continue;
            }

            let min_u = u as f32 / EDGE as f32;
            let max_u = (u + 1) as f32 / EDGE as f32;
            let min_v = v as f32 / EDGE as f32;
            let max_v = (v + 1) as f32 / EDGE as f32;
            let Some(vertices) = fluid_micro_face_vertices(
                face, x0, y0, z0, heights, min_u, max_u, min_v, max_v,
            ) else {
                continue;
            };
            let uvs = vertices.map(|vertex| {
                let local = Vec3::new(vertex[0] - x0, vertex[1] - y0, vertex[2] - z0);
                fluid_uv(face, local)
            });
            push_lit_quad(buffer, vertices, face.normal(), uvs, tint, lighting, 0.0);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a clipped fluid microface needs its face, origin, heights, and UV bounds"
)]
fn fluid_micro_face_vertices(
    face: BlockFace,
    x0: f32,
    y0: f32,
    z0: f32,
    heights: FluidFaceHeights,
    min_u: f32,
    max_u: f32,
    min_v: f32,
    max_v: f32,
) -> Option<[[f32; 3]; 4]> {
    let side_height = |u: f32| -> f32 {
        match face {
            BlockFace::Right => heights.h10 * (1.0 - u) + heights.h11 * u,
            BlockFace::Left => heights.h00 * (1.0 - u) + heights.h01 * u,
            BlockFace::Front => heights.h01 * (1.0 - u) + heights.h11 * u,
            BlockFace::Back => heights.h00 * (1.0 - u) + heights.h10 * u,
            BlockFace::Top | BlockFace::Bottom => 0.0,
        }
    };

    if matches!(
        face,
        BlockFace::Right | BlockFace::Left | BlockFace::Front | BlockFace::Back
    ) {
        let bottom = min_v;
        let corner_height_min = side_height(min_u);
        let corner_height_max = side_height(max_u);
        let top = corner_height_min.max(corner_height_max).min(max_v);
        if bottom >= top {
            return None;
        }
        let top_min = corner_height_min.min(max_v).max(bottom);
        let top_max = corner_height_max.min(max_v).max(bottom);
        let bottom = bottom.max(0.0);
        let vertices = match face {
            BlockFace::Right => [
                [x0 + 1.0, y0 + bottom, z0 + 1.0 - max_u],
                [x0 + 1.0, y0 + bottom, z0 + 1.0 - min_u],
                [x0 + 1.0, y0 + top_min, z0 + 1.0 - min_u],
                [x0 + 1.0, y0 + top_max, z0 + 1.0 - max_u],
            ],
            BlockFace::Left => [
                [x0, y0 + bottom, z0 + min_u],
                [x0, y0 + bottom, z0 + max_u],
                [x0, y0 + top_max, z0 + max_u],
                [x0, y0 + top_min, z0 + min_u],
            ],
            BlockFace::Front => [
                [x0 + min_u, y0 + bottom, z0 + 1.0],
                [x0 + max_u, y0 + bottom, z0 + 1.0],
                [x0 + max_u, y0 + top_max, z0 + 1.0],
                [x0 + min_u, y0 + top_min, z0 + 1.0],
            ],
            BlockFace::Back => [
                [x0 + 1.0 - max_u, y0 + bottom, z0],
                [x0 + 1.0 - min_u, y0 + bottom, z0],
                [x0 + 1.0 - min_u, y0 + top_min, z0],
                [x0 + 1.0 - max_u, y0 + top_max, z0],
            ],
            BlockFace::Top | BlockFace::Bottom => unreachable!(),
        };
        return Some(vertices);
    }

    Some(match face {
        BlockFace::Top => [
            [x0 + min_u, y0 + bilinear_height(heights, min_u, max_v), z0 + max_v],
            [x0 + max_u, y0 + bilinear_height(heights, max_u, max_v), z0 + max_v],
            [x0 + max_u, y0 + bilinear_height(heights, max_u, min_v), z0 + min_v],
            [x0 + min_u, y0 + bilinear_height(heights, min_u, min_v), z0 + min_v],
        ],
        BlockFace::Bottom => [
            [x0 + min_u, y0, z0 + min_v],
            [x0 + max_u, y0, z0 + min_v],
            [x0 + max_u, y0, z0 + max_v],
            [x0 + min_u, y0, z0 + max_v],
        ],
        _ => unreachable!(),
    })
}

fn bilinear_height(heights: FluidFaceHeights, u: f32, v: f32) -> f32 {
    let h0 = heights.h00 * (1.0 - u) + heights.h10 * u;
    let h1 = heights.h01 * (1.0 - u) + heights.h11 * u;
    h0 * (1.0 - v) + h1 * v
}

fn fluid_uv(face: BlockFace, local: Vec3) -> [f32; 2] {
    match face {
        BlockFace::Right => [1.0 - local.z, 1.0 - local.y],
        BlockFace::Left => [local.z, 1.0 - local.y],
        BlockFace::Top => [local.x, local.z],
        BlockFace::Bottom => [local.x, 1.0 - local.z],
        BlockFace::Front => [local.x, 1.0 - local.y],
        BlockFace::Back => [1.0 - local.x, 1.0 - local.y],
    }
}

fn fluid_face_needs_heights(face: BlockFace) -> bool {
    face != BlockFace::Bottom
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
    chunk: &VoxelChunk,
    local_position: IVec3,
    world_position: IVec3,
    fluid_id: FluidId,
) -> FluidFaceHeights {
    let mut current = [[None; 3]; 3];
    let mut above = [[None; 3]; 3];

    for z in -1..=1 {
        for x in -1..=1 {
            let x_index = (x + 1) as usize;
            let z_index = (z + 1) as usize;
            let offset = IVec3::new(x, 0, z);
            current[z_index][x_index] = fluid_at_local_or_world(
                world,
                chunk,
                local_position + offset,
                world_position + offset,
            );
            above[z_index][x_index] = fluid_at_local_or_world(
                world,
                chunk,
                local_position + offset + IVec3::Y,
                world_position + offset + IVec3::Y,
            );
        }
    }

    FluidFaceHeights {
        h00: fluid_corner_height(&current, &above, fluid_id, 0, 0),
        h10: fluid_corner_height(&current, &above, fluid_id, 2, 0),
        h11: fluid_corner_height(&current, &above, fluid_id, 2, 2),
        h01: fluid_corner_height(&current, &above, fluid_id, 0, 2),
    }
}

fn fluid_at_local_or_world<W: VoxelRead + ?Sized>(
    world: &W,
    chunk: &VoxelChunk,
    local_position: IVec3,
    world_position: IVec3,
) -> Option<FluidCell> {
    if local_position.x >= 0
        && local_position.y >= 0
        && local_position.z >= 0
        && local_position.x < CHUNK_SIZE as i32
        && local_position.y < CHUNK_SIZE as i32
        && local_position.z < CHUNK_SIZE as i32
    {
        chunk.fluid_at_local(
            local_position.x as usize,
            local_position.y as usize,
            local_position.z as usize,
        )
    } else {
        world.fluid_at(world_position)
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

type FluidNeighborContent = Option<(Option<VoxelCell>, Option<FluidCell>)>;

fn fluid_neighbor_content<W: VoxelRead + ?Sized>(
    world: &W,
    chunk: &VoxelChunk,
    local_voxel: IVec3,
    world_voxel: IVec3,
    face: BlockFace,
) -> FluidNeighborContent {
    let local = local_voxel + face.offset();
    if local.x >= 0
        && local.y >= 0
        && local.z >= 0
        && local.x < CHUNK_SIZE as i32
        && local.y < CHUNK_SIZE as i32
        && local.z < CHUNK_SIZE as i32
    {
        Some(chunk.content_at_local(
            local.x as usize,
            local.y as usize,
            local.z as usize,
        ))
    } else {
        world
            .sample_at(world_voxel + face.offset())
            .map(|(block, fluid, _)| (block, fluid))
    }
}

fn fluid_face_is_exposed(
    sample: FluidNeighborContent,
    fluid_id: FluidId,
    face: BlockFace,
) -> bool {
    let Some((block, fluid)) = sample else {
        return face == BlockFace::Top;
    };

    if let Some(block) = block {
        if !MicroblockMask::has_partial_geometry(block) {
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
    let mask = crate::voxel::microblock::MicroblockMask::geometry_for_cell(cell);
    if mask == crate::voxel::microblock::MicroblockMask::FULL {
        return false;
    }

    for a in 0..crate::voxel::microblock::MICROBLOCK_EDGE as usize {
        for b in 0..crate::voxel::microblock::MICROBLOCK_EDGE as usize {
            let position = match face {
                BlockFace::Right => [0, a, b],
                BlockFace::Left => [7, a, b],
                BlockFace::Top => [a, 0, b],
                BlockFace::Bottom => [a, 7, b],
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
    use crate::voxel::{
        cell::VoxelCell, microblock::ArtisansKitResolution, world::VoxelWorld,
    };

    #[test]
    fn only_bottom_fluid_face_skips_height_sampling() {
        for face in BlockFace::ALL {
            assert_eq!(
                fluid_face_needs_heights(face),
                face != BlockFace::Bottom,
            );
        }

        let default = FluidFaceHeights::default();
        let varied = FluidFaceHeights {
            h00: 0.1,
            h10: 0.4,
            h11: 0.7,
            h01: 1.0,
        };
        assert_eq!(
            fluid_face_vertices(BlockFace::Bottom, 2.0, 3.0, 4.0, default),
            fluid_face_vertices(BlockFace::Bottom, 2.0, 3.0, 4.0, varied),
        );
    }

    #[test]
    fn greedy_fluid_lighting_compacts_uniform_face_without_loss() {
        let lighting = FaceLighting {
            channels: [[0.6, 0.4]; 4],
            block_srgb: [[0.2, 0.5, 0.8]; 4],
            ambient_occlusion: [0.86; 4],
        };

        let compact = FluidGreedyLighting::from_uniform(lighting)
            .expect("uniform fluid lighting should compact");

        assert!(compact.expand() == lighting);
        assert_eq!(
            std::mem::size_of::<FluidGreedyLighting>(),
            6 * std::mem::size_of::<f32>(),
        );

        let mut non_uniform = lighting;
        non_uniform.channels[3][0] = 0.7;
        assert!(FluidGreedyLighting::from_uniform(non_uniform).is_none());
    }

    #[test]
    fn fluid_height_plane_cache_matches_direct_sampling() {
        let mut chunk = VoxelChunk::empty();
        chunk.set_fluid(0, 3, 0, Some(FluidCell::source(0, 8)));
        chunk.set_fluid(1, 3, 0, Some(FluidCell::spreading(0, 5, 1)));
        chunk.set_fluid(0, 3, 1, Some(FluidCell::spreading(0, 3, 2)));
        chunk.set_fluid(1, 4, 1, Some(FluidCell::source(0, 8)));

        let mut world = VoxelWorld::default();
        world.insert_chunk(IVec3::ZERO, chunk);
        let chunk = world.chunk(IVec3::ZERO).expect("test chunk should be loaded");
        let cache = FluidHeightPlaneCache::capture(&world, chunk, IVec3::ZERO, 3);

        for (x, z) in [(0, 0), (1, 0), (0, 1), (15, 15)] {
            let local = IVec3::new(x as i32, 3, z as i32);
            let direct =
                fluid_face_heights(&world, chunk, local, local, 0);
            let cached = cache.heights_at(x, z, 0);

            assert_eq!(cached.h00.to_bits(), direct.h00.to_bits());
            assert_eq!(cached.h10.to_bits(), direct.h10.to_bits());
            assert_eq!(cached.h11.to_bits(), direct.h11.to_bits());
            assert_eq!(cached.h01.to_bits(), direct.h01.to_bits());
        }
    }

    #[test]
    fn fluid_top_planes_keep_voxels_in_their_y_ranges() {
        let mut chunk = VoxelChunk::empty();
        let fluid = FluidCell::source(0, 8);
        chunk.set_fluid(4, 3, 5, Some(fluid));
        chunk.set_fluid(2, 1, 7, Some(fluid));
        chunk.set_fluid(6, 3, 1, Some(fluid));

        let planes = FluidTopPlanes::collect(&chunk, ChunkMeshletMask::ALL);

        assert_eq!(planes.plane(1).len(), 1);
        assert_eq!(planes.plane(1)[0].x, 2);
        assert_eq!(planes.plane(3).len(), 2);
        assert_eq!(planes.plane(3)[0].x, 6);
        assert_eq!(planes.plane(3)[1].x, 4);
        assert!(planes.plane(2).is_empty());
    }

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
            mask.edit(position, ArtisansKitResolution::ExtraThin, false);
            let partial = mask.apply_to_cell(cell, true);
            assert!(partial_block_face_has_opening(partial, face));
        }
    }

    #[test]
    fn clipped_fluid_side_stops_at_surface_height() {
        let heights = FluidFaceHeights {
            h00: 0.5,
            h10: 0.5,
            h11: 0.5,
            h01: 0.5,
        };

        assert!(fluid_micro_face_vertices(
            BlockFace::Front,
            0.0,
            0.0,
            0.0,
            heights,
            0.0,
            1.0,
            0.0,
            0.5,
        )
        .is_some());
        assert!(fluid_micro_face_vertices(
            BlockFace::Front,
            0.0,
            0.0,
            0.0,
            heights,
            0.0,
            1.0,
            0.5,
            0.625,
        )
        .is_none());
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
            mask.edit(wrong_position, ArtisansKitResolution::ExtraThin, false);
            let partial = mask.apply_to_cell(cell, true);
            assert!(!partial_block_face_has_opening(partial, face));

            let mut mask = crate::voxel::microblock::MicroblockMask::FULL;
            mask.edit(open_position, ArtisansKitResolution::ExtraThin, false);
            let partial = mask.apply_to_cell(cell, true);
            assert!(partial_block_face_has_opening(partial, face));
        }
    }
}
