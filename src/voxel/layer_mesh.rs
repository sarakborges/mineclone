use bevy::{platform::collections::HashMap, prelude::*};

use crate::content::{
    block::BlockRegistry,
    layer::{LayerDefinition, LayerRegistry},
};

use super::{
    block_face::BlockFace,
    chunk::{CHUNK_SIZE, VoxelChunk},
    layer::block_face,
    mesh::geometry::is_face_exposed,
    mesh_buffer::VoxelMeshBuffer,
    mesh_lighting::{face_lighting, push_lit_quad, surface_block_srgb},
    microblock::MicroblockMask,
    quad::VOXEL_FACE_UVS,
    read::VoxelRead,
};

const LAYER_STACK_OFFSET: f32 = 1.0 / 8192.0;
const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;

pub(crate) struct ChunkLayerMesh {
    pub(crate) layer_id: &'static str,
    pub(crate) face: BlockFace,
    pub(crate) mesh: Mesh,
    pub(crate) casts_shadow: bool,
}

pub(crate) fn build_layer_meshes<W, F>(
    world: &W,
    chunk_coord: IVec3,
    chunk: &VoxelChunk,
    blocks: &BlockRegistry,
    layers: &LayerRegistry,
    mut tint_at: F,
) -> Vec<ChunkLayerMesh>
where
    W: VoxelRead + ?Sized,
    F: FnMut(IVec3, &LayerDefinition) -> [f32; 3],
{
    let mut buffers =
        HashMap::<(&'static str, BlockFace, bool), VoxelMeshBuffer>::new();
    let chunk_origin = chunk_coord * CHUNK_SIZE as i32;

    for (index, attached_layers) in chunk.layer_groups() {
        let (x, y, z) = coordinates(index);
        let Some(support_cell) = chunk.cell_at(x as i32, y as i32, z as i32) else {
            continue;
        };

        // A layer is a full-face surface. Keep it attached authoritatively while
        // the host is sculpted, but hide it until partial-face projection exists.
        if MicroblockMask::is_modified(support_cell) {
            continue;
        }

        let support = blocks.get(support_cell.block_id).unwrap_or_else(|| {
            panic!("missing block definition: {}", support_cell.block_id)
        });
        let support_is_transparent = support.alpha_blend || support.alpha_cutoff.is_some();
        let world_voxel = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
        let source_block_srgb = surface_block_srgb(
            chunk.light_at(x as i32, y as i32, z as i32),
            support.light_emission > 0,
        );

        for (order, attached) in attached_layers.iter().copied().enumerate() {
            let definition = layers.get(attached.cell.layer_id).unwrap_or_else(|| {
                panic!("missing layer definition: {}", attached.cell.layer_id)
            });
            if !definition.supports_face(attached.face) {
                continue;
            }

            let face = block_face(attached.face);
            if !is_face_exposed(
                world,
                blocks,
                support_cell.block_id,
                support_is_transparent,
                world_voxel,
                face,
            ) {
                continue;
            }

            let stack_index = attached_layers[..order]
                .iter()
                .filter(|earlier| earlier.face == attached.face)
                .count();
            let outward_offset =
                definition.offset + stack_index as f32 * LAYER_STACK_OFFSET;
            let origin = Vec3::new(x as f32, y as f32, z as f32);
            let normal = Vec3::from_array(face.normal());
            let vertices = face.unit_vertices().map(|vertex| {
                (Vec3::from_array(vertex) + origin + normal * outward_offset).to_array()
            });
            let lighting = face_lighting(world, world_voxel, face, source_block_srgb);
            let tint = tint_at(world_voxel, definition);

            push_lit_quad(
                buffers
                    .entry((attached.cell.layer_id, face, definition.casts_shadow))
                    .or_default(),
                vertices,
                face.normal(),
                attached
                    .cell
                    .texture_rotation
                    .rotate_uvs(VOXEL_FACE_UVS),
                tint,
                lighting,
            );
        }
    }

    let mut meshes = buffers
        .into_iter()
        .filter_map(|((layer_id, face, casts_shadow), buffer)| {
            buffer.into_mesh().map(|mesh| ChunkLayerMesh {
                layer_id,
                face,
                mesh,
                casts_shadow,
            })
        })
        .collect::<Vec<_>>();
    meshes.sort_by_key(|mesh| {
        (
            mesh.layer_id,
            face_sort_key(mesh.face),
            mesh.casts_shadow,
        )
    });
    meshes
}

fn coordinates(index: usize) -> (usize, usize, usize) {
    let y = index / CHUNK_AREA;
    let layer_index = index % CHUNK_AREA;
    let z = layer_index / CHUNK_SIZE;
    let x = layer_index % CHUNK_SIZE;
    (x, y, z)
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
