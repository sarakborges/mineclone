use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use super::{
    chunk::CHUNK_SIZE,
    coordinates::chunk_origin,
    mesh_buffer::{ATTRIBUTE_VOXEL_LIGHT, ATTRIBUTE_VOXEL_PAYLOAD},
};

pub(crate) const CHUNK_MESHLET_EDGE: usize = 8;
const MESHLETS_PER_AXIS: usize = CHUNK_SIZE / CHUNK_MESHLET_EDGE;
const MESHLET_COUNT: usize =
    MESHLETS_PER_AXIS * MESHLETS_PER_AXIS * MESHLETS_PER_AXIS;

const _: () = assert!(CHUNK_SIZE.is_multiple_of(CHUNK_MESHLET_EDGE));
const _: () = assert!(MESHLETS_PER_AXIS == 2);
const _: () = assert!(MESHLET_COUNT == 8);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct ChunkMeshletMask(u8);

impl ChunkMeshletMask {
    pub(crate) const ALL: Self = Self(u8::MAX);

    pub(crate) fn is_all(self) -> bool {
        self == Self::ALL
    }

    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub(crate) fn for_world_position(chunk_coord: IVec3, world_position: IVec3) -> Self {
        let local = world_position - chunk_origin(chunk_coord);
        let mut mask = Self::default();

        for meshlet_y in 0..MESHLETS_PER_AXIS {
            for meshlet_z in 0..MESHLETS_PER_AXIS {
                for meshlet_x in 0..MESHLETS_PER_AXIS {
                    let minimum = IVec3::new(
                        (meshlet_x * CHUNK_MESHLET_EDGE) as i32,
                        (meshlet_y * CHUNK_MESHLET_EDGE) as i32,
                        (meshlet_z * CHUNK_MESHLET_EDGE) as i32,
                    );
                    let maximum = minimum + IVec3::splat(CHUNK_MESHLET_EDGE as i32);

                    if local.x < minimum.x - 1
                        || local.y < minimum.y - 1
                        || local.z < minimum.z - 1
                        || local.x > maximum.x
                        || local.y > maximum.y
                        || local.z > maximum.z
                    {
                        continue;
                    }

                    mask.0 |= 1 << meshlet_index(meshlet_x, meshlet_y, meshlet_z);
                }
            }
        }

        mask
    }

    pub(crate) fn selected_voxel_count(self) -> usize {
        self.0.count_ones() as usize
            * CHUNK_MESHLET_EDGE
            * CHUNK_MESHLET_EDGE
            * CHUNK_MESHLET_EDGE
    }

    pub(crate) fn for_each_voxel(
        self,
        mut visit: impl FnMut(usize, usize, usize),
    ) {
        for meshlet_y in 0..MESHLETS_PER_AXIS {
            for meshlet_z in 0..MESHLETS_PER_AXIS {
                for meshlet_x in 0..MESHLETS_PER_AXIS {
                    let index = meshlet_index(meshlet_x, meshlet_y, meshlet_z);
                    if !self.contains_index(index) {
                        continue;
                    }

                    let x0 = meshlet_x * CHUNK_MESHLET_EDGE;
                    let y0 = meshlet_y * CHUNK_MESHLET_EDGE;
                    let z0 = meshlet_z * CHUNK_MESHLET_EDGE;
                    for y in y0..y0 + CHUNK_MESHLET_EDGE {
                        for z in z0..z0 + CHUNK_MESHLET_EDGE {
                            for x in x0..x0 + CHUNK_MESHLET_EDGE {
                                visit(x, y, z);
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn contains_index(self, index: usize) -> bool {
        debug_assert!(index < MESHLET_COUNT);
        self.0 & (1 << index) != 0
    }

    pub(crate) fn for_dependency_offset(offset: IVec3) -> Self {
        let mut mask = Self::default();
        for index in 0..MESHLET_COUNT {
            let single = Self(1 << index);
            if single.depends_on_neighbor_offset(offset) {
                mask.0 |= 1 << index;
            }
        }
        mask
    }

    pub(crate) fn depends_on_neighbor_offset(self, offset: IVec3) -> bool {
        if self.is_empty()
            || offset.x.abs() > 1
            || offset.y.abs() > 1
            || offset.z.abs() > 1
        {
            return false;
        }
        if offset == IVec3::ZERO {
            return true;
        }

        (0..MESHLET_COUNT).any(|index| {
            if !self.contains_index(index) {
                return false;
            }

            let meshlet_x = index % MESHLETS_PER_AXIS;
            let meshlet_z = (index / MESHLETS_PER_AXIS) % MESHLETS_PER_AXIS;
            let meshlet_y =
                index / (MESHLETS_PER_AXIS * MESHLETS_PER_AXIS);
            let side_x = if meshlet_x == 0 { -1 } else { 1 };
            let side_y = if meshlet_y == 0 { -1 } else { 1 };
            let side_z = if meshlet_z == 0 { -1 } else { 1 };

            (offset.x == 0 || offset.x == side_x)
                && (offset.y == 0 || offset.y == side_y)
                && (offset.z == 0 || offset.z == side_z)
        })
    }

    pub(crate) fn contains_voxel(self, x: usize, y: usize, z: usize) -> bool {
        let meshlet_x = x / CHUNK_MESHLET_EDGE;
        let meshlet_y = y / CHUNK_MESHLET_EDGE;
        let meshlet_z = z / CHUNK_MESHLET_EDGE;
        let bit = 1 << meshlet_index(meshlet_x, meshlet_y, meshlet_z);
        self.0 & bit != 0
    }

    fn contains_payload(self, payload: u32) -> bool {
        let meshlet = ((payload >> 28) & 0x7) as usize;
        debug_assert!(meshlet < MESHLET_COUNT);
        self.contains_index(meshlet)
    }
}

fn meshlet_index(x: usize, y: usize, z: usize) -> usize {
    x + z * MESHLETS_PER_AXIS + y * MESHLETS_PER_AXIS * MESHLETS_PER_AXIS
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum VoxelMeshPatch {
    Unchanged,
    Changed(Mesh),
}

pub(crate) fn patch_voxel_mesh(
    existing: &Mesh,
    replacement: Option<&Mesh>,
    dirty: ChunkMeshletMask,
) -> Option<VoxelMeshPatch> {
    if dirty.is_all() {
        return Some(VoxelMeshPatch::Changed(match replacement {
            Some(mesh) => MeshArrays::from_mesh(mesh)?.into_mesh(),
            None => MeshArrays::default().into_mesh(),
        }));
    }

    let existing = MeshView::from_mesh(existing)?;
    let replacement = match replacement {
        Some(mesh) => Some(MeshView::from_mesh(mesh)?),
        None => None,
    };
    let existing_dirty = existing.has_quad_in(dirty);
    let replacement_dirty = replacement
        .as_ref()
        .is_some_and(|replacement| replacement.has_quad_in(dirty));

    if !existing_dirty && !replacement_dirty {
        return Some(VoxelMeshPatch::Unchanged);
    }

    let replacement_vertices = replacement
        .as_ref()
        .map_or(0, |replacement| replacement.positions.len());
    let replacement_indices = replacement
        .as_ref()
        .map_or(0, |replacement| replacement.indices.len());
    let mut output = MeshArrays::with_capacity(
        existing.positions.len() + replacement_vertices,
        existing.indices.len() + replacement_indices,
    );
    existing.append_filtered(&mut output, dirty, false)?;

    if let Some(replacement) = &replacement {
        replacement.append_filtered(&mut output, dirty, true)?;
    }

    Some(VoxelMeshPatch::Changed(output.into_mesh()))
}

/// Validated, borrowed inputs keep no-op patches allocation-free and avoid
/// copying the complete source meshes before selecting the surviving quads.
struct MeshView<'a> {
    positions: &'a [[f32; 3]],
    uvs: &'a [[f32; 2]],
    payloads: &'a [u32],
    colors: &'a [[u8; 4]],
    indices: &'a Indices,
}

impl<'a> MeshView<'a> {
    fn from_mesh(mesh: &'a Mesh) -> Option<Self> {
        if !mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD) {
            return None;
        }

        let view = Self {
            positions: float32x3(mesh.attribute(Mesh::ATTRIBUTE_POSITION)?)?,
            uvs: float32x2(mesh.attribute(Mesh::ATTRIBUTE_UV_0)?)?,
            payloads: uint32(mesh.attribute(Mesh::ATTRIBUTE_UV_1)?)?,
            colors: unorm8x4(mesh.attribute(Mesh::ATTRIBUTE_COLOR)?)?,
            indices: mesh.indices()?,
        };
        let vertices = view.positions.len();
        if !vertices.is_multiple_of(4)
            || !view.indices.len().is_multiple_of(6)
            || view.indices.len() / 6 != vertices / 4
            || view.uvs.len() != vertices
            || view.payloads.len() != vertices
            || view.colors.len() != vertices
        {
            return None;
        }
        Some(view)
    }

    fn has_quad_in(&self, dirty: ChunkMeshletMask) -> bool {
        self.payloads
            .as_chunks::<4>()
            .0
            .iter()
            .any(|quad| dirty.contains_payload(quad[0]))
    }

    fn append_filtered(
        &self,
        output: &mut MeshArrays,
        dirty: ChunkMeshletMask,
        keep_dirty: bool,
    ) -> Option<()> {
        let quad_count = self.positions.len() / 4;

        for quad in 0..quad_count {
            let source_base = quad * 4;
            let is_dirty = dirty.contains_payload(self.payloads[source_base]);
            if is_dirty != keep_dirty {
                continue;
            }

            let target_base = output.positions.len() as u32;
            output
                .positions
                .extend_from_slice(&self.positions[source_base..source_base + 4]);
            output
                .uvs
                .extend_from_slice(&self.uvs[source_base..source_base + 4]);
            output
                .payloads
                .extend_from_slice(&self.payloads[source_base..source_base + 4]);
            output
                .colors
                .extend_from_slice(&self.colors[source_base..source_base + 4]);

            let source_index_base = quad * 6;
            for offset in 0..6 {
                let index = match self.indices {
                    Indices::U16(indices) => u32::from(indices[source_index_base + offset]),
                    Indices::U32(indices) => indices[source_index_base + offset],
                };
                let relative = index.checked_sub(source_base as u32)?;
                if relative >= 4 {
                    return None;
                }
                output.indices.push(target_base + relative);
            }
        }

        Some(())
    }
}

#[derive(Default)]
struct MeshArrays {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    payloads: Vec<u32>,
    colors: Vec<[u8; 4]>,
    indices: Vec<u32>,
}

impl MeshArrays {
    fn with_capacity(vertices: usize, indices: usize) -> Self {
        Self {
            positions: Vec::with_capacity(vertices),
            uvs: Vec::with_capacity(vertices),
            payloads: Vec::with_capacity(vertices),
            colors: Vec::with_capacity(vertices),
            indices: Vec::with_capacity(indices),
        }
    }

    fn from_mesh(mesh: &Mesh) -> Option<Self> {
        let view = MeshView::from_mesh(mesh)?;
        let mut arrays = Self::with_capacity(view.positions.len(), view.indices.len());
        view.append_filtered(&mut arrays, ChunkMeshletMask::ALL, true)?;
        Some(arrays)
    }

    fn into_mesh(self) -> Mesh {
        let indices = if self.positions.len() <= usize::from(u16::MAX) + 1 {
            Indices::U16(
                self.indices
                    .into_iter()
                    .map(|index| index as u16)
                    .collect(),
            )
        } else {
            Indices::U32(self.indices)
        };

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(
            ATTRIBUTE_VOXEL_PAYLOAD,
            VertexAttributeValues::Uint32(self.payloads),
        )
        .with_inserted_attribute(
            ATTRIBUTE_VOXEL_LIGHT,
            VertexAttributeValues::Unorm8x4(self.colors),
        )
        .with_inserted_indices(indices)
    }
}

fn float32x3(values: &VertexAttributeValues) -> Option<&[[f32; 3]]> {
    match values {
        VertexAttributeValues::Float32x3(values) => Some(values),
        _ => None,
    }
}

fn float32x2(values: &VertexAttributeValues) -> Option<&[[f32; 2]]> {
    match values {
        VertexAttributeValues::Float32x2(values) => Some(values),
        _ => None,
    }
}

fn uint32(values: &VertexAttributeValues) -> Option<&[u32]> {
    match values {
        VertexAttributeValues::Uint32(values) => Some(values),
        _ => None,
    }
}

fn unorm8x4(values: &VertexAttributeValues) -> Option<&[[u8; 4]]> {
    match values {
        VertexAttributeValues::Unorm8x4(values) => Some(values),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::quad::quad_triangle_indices;

    fn quad_mesh(meshlets: &[u32], position_offset: f32, wide_indices: bool) -> Mesh {
        let mut arrays = MeshArrays::with_capacity(meshlets.len() * 4, meshlets.len() * 6);
        for (quad, &meshlet) in meshlets.iter().enumerate() {
            let x = position_offset + quad as f32;
            arrays.positions.extend([
                [x, 0.0, 0.0], [x + 1.0, 0.0, 0.0],
                [x + 1.0, 1.0, 0.0], [x, 1.0, 0.0],
            ]);
            arrays.uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
            arrays.payloads.extend([meshlet << 28; 4]);
            arrays.colors.extend([[17, 39, 83, 255]; 4]);
            arrays.indices.extend(quad_triangle_indices((quad * 4) as u32, true));
        }
        let mut mesh = arrays.into_mesh();
        if wide_indices {
            let indices = mesh.indices().unwrap().iter().map(|index| index as u32).collect();
            mesh.insert_indices(Indices::U32(indices));
        }
        mesh
    }

    #[test]
    fn partial_patch_preserves_other_meshlets_and_both_index_widths() {
        for existing_wide in [false, true] {
            for replacement_wide in [false, true] {
                let existing = quad_mesh(&[0, 1], 0.0, existing_wide);
                let replacement = quad_mesh(&[0, 2], 100.0, replacement_wide);
                let VoxelMeshPatch::Changed(patched) = patch_voxel_mesh(
                    &existing, Some(&replacement), ChunkMeshletMask(1),
                ).unwrap() else {
                    panic!("the selected meshlet must change");
                };

                let actual = MeshView::from_mesh(&patched).unwrap();
                let original = MeshView::from_mesh(&existing).unwrap();
                let updated = MeshView::from_mesh(&replacement).unwrap();
                assert_eq!(&actual.positions[..4], &original.positions[4..]);
                assert_eq!(&actual.positions[4..], &updated.positions[..4]);
                assert_eq!(&actual.uvs[..4], &original.uvs[4..]);
                assert_eq!(&actual.uvs[4..], &updated.uvs[..4]);
                assert_eq!(&actual.colors[..4], &original.colors[4..]);
                assert_eq!(&actual.colors[4..], &updated.colors[..4]);
                assert_eq!(actual.payloads, &[1 << 28, 1 << 28, 1 << 28, 1 << 28, 0, 0, 0, 0]);
                assert_eq!(
                    actual.indices.iter().collect::<Vec<_>>(),
                    vec![0, 1, 3, 1, 2, 3, 4, 5, 7, 5, 6, 7],
                );
            }
        }
    }

    #[test]
    fn unrelated_patch_is_unchanged_and_removal_can_empty_a_mesh() {
        let existing = quad_mesh(&[0], 0.0, false);
        assert!(matches!(
            patch_voxel_mesh(&existing, None, ChunkMeshletMask(2)),
            Some(VoxelMeshPatch::Unchanged),
        ));
        let VoxelMeshPatch::Changed(empty) =
            patch_voxel_mesh(&existing, None, ChunkMeshletMask(1)).unwrap()
        else {
            panic!("removing the last quad must change the mesh");
        };
        assert_eq!(empty.count_vertices(), 0);
        assert_eq!(empty.indices().unwrap().len(), 0);
    }

    #[test]
    fn patch_rejects_mismatched_attributes_and_indices_outside_a_quad() {
        let mismatched = quad_mesh(&[1], 0.0, false)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; 3]);
        assert!(patch_voxel_mesh(&mismatched, None, ChunkMeshletMask(1)).is_none());

        let invalid_indices = quad_mesh(&[1], 0.0, false)
            .with_inserted_indices(Indices::U16(vec![0, 1, 4, 1, 2, 3]));
        let replacement = quad_mesh(&[0], 10.0, false);
        assert!(patch_voxel_mesh(
            &invalid_indices, Some(&replacement), ChunkMeshletMask(1),
        ).is_none());
        assert!(patch_voxel_mesh(
            &replacement, Some(&invalid_indices), ChunkMeshletMask::ALL,
        ).is_none());
    }

    #[test]
    fn growing_patch_promotes_indices_before_u16_overflow() {
        let quad_count = (usize::from(u16::MAX) + 1) / 4;
        let existing = quad_mesh(&vec![1; quad_count], 0.0, false);
        let replacement = quad_mesh(&[0], 100.0, false);
        assert!(matches!(existing.indices(), Some(Indices::U16(_))));
        let VoxelMeshPatch::Changed(patched) = patch_voxel_mesh(
            &existing, Some(&replacement), ChunkMeshletMask(1),
        ).unwrap() else {
            panic!("adding a meshlet must change the mesh");
        };
        assert!(matches!(patched.indices(), Some(Indices::U32(_))));
        assert_eq!(patched.count_vertices(), (quad_count + 1) * 4);
        assert_eq!(patched.indices().unwrap().iter().max(), Some(quad_count * 4 + 3));
    }
}
