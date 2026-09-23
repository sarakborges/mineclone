use bevy::prelude::*;
use smallvec::SmallVec;

use crate::content::{
    block::{BlockLookup, BlockRegistry},
    layer::{LayerDefinition, LayerRegistry},
};

use super::{
    block_face::BlockFace,
    chunk::{CHUNK_SIZE, VoxelChunk},
    layer::block_face,
    mesh::geometry::is_face_exposed,
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{
        ChunkLightingCache, face_lighting_with_cache, push_lit_quad,
        surface_block_srgb_with_cache,
    },
    meshlet::ChunkMeshletMask,
    microblock::{MICROBLOCK_EDGE, MicroblockMask, occupied_cell},
    quad::VOXEL_FACE_UVS,
    read::VoxelRead,
    texture_rotation::TextureRotation,
};

const LAYER_STACK_OFFSET: f32 = 1.0 / 8192.0;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
const MICRO_EDGE: usize = MICROBLOCK_EDGE as usize;
type LayerMeshKey = (&'static str, bool);
type LayerMeshBuffers = SmallVec<[(LayerMeshKey, VoxelMeshBuffer); 2]>;

fn block_face_index(face: BlockFace) -> usize {
    match face {
        BlockFace::Right => 0,
        BlockFace::Left => 1,
        BlockFace::Top => 2,
        BlockFace::Bottom => 3,
        BlockFace::Front => 4,
        BlockFace::Back => 5,
    }
}

pub(crate) struct ChunkLayerMesh {
    pub(crate) layer_id: &'static str,
    pub(crate) mesh: Mesh,
    pub(crate) casts_shadow: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_layer_meshlets<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    layers: &LayerRegistry,
    meshlets: ChunkMeshletMask,
    lighting_cache: Option<&ChunkLightingCache>,
    mut tint_at: F,
) -> Vec<ChunkLayerMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, &LayerDefinition) -> [f32; 3],
{
    let mut buffers = LayerMeshBuffers::new();
    let mut block_lookup = BlockLookup::new(blocks);
    let chunk_origin = chunk_coord * CHUNK_SIZE as i32;

    for (index, attached_layers) in chunk.layer_groups() {
        let (x, y, z) = coordinates(index);
        if !meshlets.contains_voxel(x, y, z) {
            continue;
        }
        let (support_cell, _, support_light) = chunk.sample_local_at(x, y, z);
        let Some(support_cell) = support_cell else {
            continue;
        };

        let support = block_lookup.get(support_cell.block_id);
        let support_is_transparent = support.alpha_blend || support.alpha_cutoff.is_some();
        let world_voxel = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
        let local_voxel = IVec3::new(x as i32, y as i32, z as i32);
        let source_block_srgb = surface_block_srgb_with_cache(
            lighting_cache,
            world_voxel,
            support_light,
            support.light_emission > 0,
        );
        let mut lighting_by_face = [None; 6];
        let mut exposure_by_face = [None; 6];

        for (order, attached) in attached_layers.iter().copied().enumerate() {
            let definition = layers.get(attached.cell.layer_id).unwrap_or_else(|| {
                panic!("missing layer definition: {}", attached.cell.layer_id)
            });
            if !definition.supports_face(attached.face) {
                continue;
            }

            let face = block_face(attached.face);
            let stack_index = attached_layers[..order]
                .iter()
                .filter(|earlier| earlier.face == attached.face)
                .count();
            let outward_offset =
                definition.offset + stack_index as f32 * LAYER_STACK_OFFSET;
            let face_index = block_face_index(face);
            let lighting = if let Some(lighting) = lighting_by_face[face_index] {
                lighting
            } else {
                let lighting =
                    face_lighting_with_cache(
                    lighting_cache,
                    world,
                    world_voxel,
                    face,
                    source_block_srgb,
                );
                lighting_by_face[face_index] = Some(lighting);
                lighting
            };
            let tint = tint_at(world_voxel, definition);

            if MicroblockMask::is_modified(support_cell) {
                emit_sculpted_layer(
                    world,
                    &mut block_lookup,
                    &mut buffers,
                    support_cell,
                    support_is_transparent,
                    world_voxel,
                    local_voxel,
                    attached.cell.layer_id,
                    attached.cell.texture_rotation,
                    face,
                    definition,
                    outward_offset,
                    tint,
                    lighting,
                );
                continue;
            }

            let is_exposed = if let Some(is_exposed) = exposure_by_face[face_index] {
                is_exposed
            } else {
                let is_exposed = is_face_exposed(
                    world,
                    &mut block_lookup,
                    support_cell.block_id,
                    support_is_transparent,
                    world_voxel,
                    face,
                );
                exposure_by_face[face_index] = Some(is_exposed);
                is_exposed
            };
            if !is_exposed {
                continue;
            }

            let origin = local_voxel.as_vec3();
            let normal = Vec3::from_array(face.normal());
            let vertices = face.unit_vertices().map(|vertex| {
                (Vec3::from_array(vertex) + origin + normal * outward_offset).to_array()
            });

            push_lit_quad(
                layer_buffer(&mut buffers, attached.cell.layer_id, definition),
                vertices,
                face.normal(),
                attached
                    .cell
                    .texture_rotation
                    .rotate_uvs(VOXEL_FACE_UVS),
                tint,
                lighting,
                0.0,
            );
        }
    }

    let mut meshes = buffers
        .into_iter()
        .filter_map(|((layer_id, casts_shadow), buffer)| {
            buffer.into_mesh().map(|mesh| ChunkLayerMesh {
                layer_id,
                mesh,
                casts_shadow,
            })
        })
        .collect::<Vec<_>>();
    meshes.sort_by_key(|mesh| (mesh.layer_id, mesh.casts_shadow));
    meshes
}

#[expect(
    clippy::too_many_arguments,
    reason = "sculpted layer projection needs the host, face, visual and mesh context"
)]
fn emit_sculpted_layer<W: VoxelRead + ?Sized>(
    world: &W,
    block_lookup: &mut BlockLookup<'_>,
    buffers: &mut LayerMeshBuffers,
    support_cell: super::cell::VoxelCell,
    support_is_transparent: bool,
    world_voxel: IVec3,
    local_voxel: IVec3,
    layer_id: &'static str,
    texture_rotation: TextureRotation,
    face: BlockFace,
    definition: &LayerDefinition,
    outward_offset: f32,
    tint: [f32; 3],
    lighting: super::mesh_lighting::FaceLighting,
) {
    let shape = MicroblockMask::from_cell(support_cell);

    // A layer attached to a sculpted face must follow every exposed microface
    // with that normal, exactly like the host block's own texture. Restricting
    // it to the macroblock boundary leaves the layer floating or missing after
    // the Artisan's Kit recesses that surface.
    for depth in 0..MICRO_EDGE {
        let mut visible = [false; MICRO_EDGE * MICRO_EDGE];

        for v in 0..MICRO_EDGE {
            for u in 0..MICRO_EDGE {
                let position = micro_position_for(face, depth, u, v);
                if !shape.contains(position) {
                    continue;
                }

                // Match the host micro-mesh floor rule: only the actual
                // world-bottom microface is suppressed, not recessed surfaces
                // higher inside the same macroblock.
                if face == BlockFace::Bottom
                    && world_voxel.y == 0
                    && position[1] == 0
                {
                    continue;
                }

                let fine = world_voxel * MICROBLOCK_EDGE
                    + IVec3::new(position[0] as i32, position[1] as i32, position[2] as i32);
                let occluded =
                    occupied_cell(world, fine + face.offset()).is_some_and(|neighbor| {
                        let neighbor_definition = block_lookup.get(neighbor.block_id);
                        (support_cell.block_id == neighbor.block_id && support_is_transparent)
                            || (!neighbor_definition.alpha_blend
                                && neighbor_definition.alpha_cutoff.is_none())
                    });
                visible[u + v * MICRO_EDGE] = !occluded;
            }
        }

        for v in 0..MICRO_EDGE {
            for u in 0..MICRO_EDGE {
                if !visible[u + v * MICRO_EDGE] {
                    continue;
                }
                let mut width = 1;
                while u + width < MICRO_EDGE && visible[u + width + v * MICRO_EDGE] {
                    width += 1;
                }
                let mut height = 1;
                while v + height < MICRO_EDGE
                    && (u..u + width)
                        .all(|column| visible[column + (v + height) * MICRO_EDGE])
                {
                    height += 1;
                }
                for row in v..v + height {
                    for column in u..u + width {
                        visible[column + row * MICRO_EDGE] = false;
                    }
                }

                emit_sculpted_layer_rectangle(
                    buffers,
                    local_voxel,
                    layer_id,
                    texture_rotation,
                    face,
                    definition,
                    outward_offset,
                    tint,
                    lighting,
                    depth,
                    u,
                    v,
                    width,
                    height,
                );
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "greedy layer rectangles carry their face, bounds and visual state"
)]
fn emit_sculpted_layer_rectangle(
    buffers: &mut LayerMeshBuffers,
    local_voxel: IVec3,
    layer_id: &'static str,
    texture_rotation: TextureRotation,
    face: BlockFace,
    definition: &LayerDefinition,
    outward_offset: f32,
    tint: [f32; 3],
    lighting: super::mesh_lighting::FaceLighting,
    depth: usize,
    u: usize,
    v: usize,
    width: usize,
    height: usize,
) {
    let [min_x, min_y, min_z] = micro_position_for(face, depth, u, v);
    let mut lower = [min_x, min_y, min_z];
    let mut upper = lower;
    match face {
        BlockFace::Right | BlockFace::Left => {
            upper[0] += 1;
            upper[1] += height;
            upper[2] += width;
        }
        BlockFace::Top | BlockFace::Bottom => {
            upper[0] += width;
            upper[1] += 1;
            upper[2] += height;
        }
        BlockFace::Front | BlockFace::Back => {
            upper[0] += width;
            upper[1] += height;
            upper[2] += 1;
        }
    }
    match face {
        BlockFace::Right => lower[0] = upper[0],
        BlockFace::Left => upper[0] = lower[0],
        BlockFace::Top => lower[1] = upper[1],
        BlockFace::Bottom => upper[1] = lower[1],
        BlockFace::Front => lower[2] = upper[2],
        BlockFace::Back => upper[2] = lower[2],
    }

    let normal = Vec3::from_array(face.normal());
    let vertices = face.unit_vertices().map(|corner| {
        let mut point = [0.0; 3];
        for axis in 0..3 {
            let coordinate = if corner[axis] == 0.0 {
                lower[axis]
            } else {
                upper[axis]
            };
            point[axis] =
                local_voxel[axis] as f32 + coordinate as f32 / MICROBLOCK_EDGE as f32;
        }
        (Vec3::from_array(point) + normal * outward_offset).to_array()
    });
    let uvs = vertices.map(|vertex| {
        let local = Vec3::from_array(vertex)
            - local_voxel.as_vec3()
            - normal * outward_offset;
        rotate_macro_uv(macro_uv(face, local), texture_rotation)
    });

    push_lit_quad(
        layer_buffer(buffers, layer_id, definition),
        vertices,
        face.normal(),
        uvs,
        tint,
        lighting,
        0.0,
    );
}

fn layer_buffer<'a>(
    buffers: &'a mut LayerMeshBuffers,
    layer_id: &'static str,
    definition: &LayerDefinition,
) -> &'a mut VoxelMeshBuffer {
    let key = (layer_id, definition.casts_shadow);
    if let Some(index) = buffers
        .iter()
        .position(|(candidate, _)| *candidate == key)
    {
        return &mut buffers[index].1;
    }

    buffers.push((key, VoxelMeshBuffer::default()));
    &mut buffers
        .last_mut()
        .expect("layer mesh buffer was just inserted")
        .1
}

fn micro_position_for(face: BlockFace, depth: usize, u: usize, v: usize) -> [usize; 3] {
    match face {
        BlockFace::Right | BlockFace::Left => [depth, v, u],
        BlockFace::Top | BlockFace::Bottom => [u, depth, v],
        BlockFace::Front | BlockFace::Back => [u, v, depth],
    }
}

fn macro_uv(face: BlockFace, point: Vec3) -> [f32; 2] {
    match face {
        BlockFace::Right => [1.0 - point.z, 1.0 - point.y],
        BlockFace::Left => [point.z, 1.0 - point.y],
        BlockFace::Top => [point.x, point.z],
        BlockFace::Bottom => [point.x, 1.0 - point.z],
        BlockFace::Front => [point.x, 1.0 - point.y],
        BlockFace::Back => [1.0 - point.x, 1.0 - point.y],
    }
}

fn rotate_macro_uv([u, v]: [f32; 2], rotation: TextureRotation) -> [f32; 2] {
    match rotation {
        TextureRotation::Degrees0 => [u, v],
        TextureRotation::Degrees90 => [1.0 - v, u],
        TextureRotation::Degrees180 => [1.0 - u, 1.0 - v],
        TextureRotation::Degrees270 => [v, 1.0 - u],
    }
}

fn coordinates(index: usize) -> (usize, usize, usize) {
    let y = index / CHUNK_AREA;
    let layer_index = index % CHUNK_AREA;
    let z = layer_index / CHUNK_SIZE;
    let x = layer_index % CHUNK_SIZE;
    (x, y, z)
}

