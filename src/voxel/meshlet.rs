use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use super::{chunk::CHUNK_SIZE, coordinates::chunk_origin};

pub(crate) const CHUNK_MESHLET_EDGE: usize = 8;
const MESHLETS_PER_AXIS: usize = CHUNK_SIZE / CHUNK_MESHLET_EDGE;
const MESHLET_COUNT: usize =
    MESHLETS_PER_AXIS * MESHLETS_PER_AXIS * MESHLETS_PER_AXIS;

const _: () = assert!(CHUNK_SIZE % CHUNK_MESHLET_EDGE == 0);
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

    pub(crate) fn compact_voxel_index(
        self,
        x: usize,
        y: usize,
        z: usize,
    ) -> Option<usize> {
        let meshlet_x = x / CHUNK_MESHLET_EDGE;
        let meshlet_y = y / CHUNK_MESHLET_EDGE;
        let meshlet_z = z / CHUNK_MESHLET_EDGE;
        let index = meshlet_index(meshlet_x, meshlet_y, meshlet_z);
        if !self.contains_index(index) {
            return None;
        }

        let lower_mask = if index == 0 { 0 } else { (1_u8 << index) - 1 };
        let rank = (self.0 & lower_mask).count_ones() as usize;
        let local_x = x % CHUNK_MESHLET_EDGE;
        let local_y = y % CHUNK_MESHLET_EDGE;
        let local_z = z % CHUNK_MESHLET_EDGE;
        let voxels_per_meshlet =
            CHUNK_MESHLET_EDGE * CHUNK_MESHLET_EDGE * CHUNK_MESHLET_EDGE;
        let local_index = local_x
            + local_z * CHUNK_MESHLET_EDGE
            + local_y * CHUNK_MESHLET_EDGE * CHUNK_MESHLET_EDGE;
        Some(rank * voxels_per_meshlet + local_index)
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

    fn contains_quad(self, positions: &[[f32; 3]], normals: &[[f32; 3]], base: usize) -> bool {
        let mut center = Vec3::ZERO;
        for position in &positions[base..base + 4] {
            center += Vec3::from_array(*position);
        }
        center *= 0.25;

        let normal = Vec3::from_array(normals[base]);
        // Layer surfaces are deliberately pushed ~0.001 blocks outward to
        // avoid z-fighting. Step farther back than that offset, while staying
        // well inside the smallest 1/8-block sculpted cell.
        let source = center - normal * 0.01;
        let source = source.floor().as_ivec3();
        if source.x < 0
            || source.y < 0
            || source.z < 0
            || source.x >= CHUNK_SIZE as i32
            || source.y >= CHUNK_SIZE as i32
            || source.z >= CHUNK_SIZE as i32
        {
            return false;
        }

        self.contains_voxel(source.x as usize, source.y as usize, source.z as usize)
    }
}

fn meshlet_index(x: usize, y: usize, z: usize) -> usize {
    x + z * MESHLETS_PER_AXIS + y * MESHLETS_PER_AXIS * MESHLETS_PER_AXIS
}

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

    let existing = MeshArrays::from_mesh(existing)?;
    let replacement = match replacement {
        Some(mesh) => Some(MeshArrays::from_mesh(mesh)?),
        None => None,
    };
    let existing_dirty = existing.has_quad_in(dirty)?;
    let replacement_dirty = replacement
        .as_ref()
        .is_some_and(|replacement| replacement.has_quad_in(dirty).unwrap_or(true));

    if !existing_dirty && !replacement_dirty {
        return Some(VoxelMeshPatch::Unchanged);
    }

    let mut output = MeshArrays::default();
    existing.append_filtered(&mut output, dirty, false)?;

    if let Some(replacement) = &replacement {
        replacement.append_filtered(&mut output, dirty, true)?;
    }

    Some(VoxelMeshPatch::Changed(output.into_mesh()))
}

#[derive(Default)]
struct MeshArrays {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    light_uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshArrays {
    fn from_mesh(mesh: &Mesh) -> Option<Self> {
        if !mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD) {
            return None;
        }

        Some(Self {
            positions: float32x3(mesh.attribute(Mesh::ATTRIBUTE_POSITION)?)?.to_vec(),
            normals: float32x3(mesh.attribute(Mesh::ATTRIBUTE_NORMAL)?)?.to_vec(),
            uvs: float32x2(mesh.attribute(Mesh::ATTRIBUTE_UV_0)?)?.to_vec(),
            light_uvs: float32x2(mesh.attribute(Mesh::ATTRIBUTE_UV_1)?)?.to_vec(),
            colors: float32x4(mesh.attribute(Mesh::ATTRIBUTE_COLOR)?)?.to_vec(),
            indices: mesh.indices()?.iter().collect(),
        })
    }

    fn has_quad_in(&self, dirty: ChunkMeshletMask) -> Option<bool> {
        if self.positions.len() % 4 != 0 || self.indices.len() % 6 != 0 {
            return None;
        }
        let quad_count = self.positions.len() / 4;
        if self.indices.len() / 6 != quad_count
            || self.normals.len() != self.positions.len()
            || self.uvs.len() != self.positions.len()
            || self.light_uvs.len() != self.positions.len()
            || self.colors.len() != self.positions.len()
        {
            return None;
        }

        Some((0..quad_count).any(|quad| {
            dirty.contains_quad(&self.positions, &self.normals, quad * 4)
        }))
    }

    fn append_filtered(
        &self,
        output: &mut Self,
        dirty: ChunkMeshletMask,
        keep_dirty: bool,
    ) -> Option<()> {
        if self.positions.len() % 4 != 0 || self.indices.len() % 6 != 0 {
            return None;
        }
        let quad_count = self.positions.len() / 4;
        if self.indices.len() / 6 != quad_count
            || self.normals.len() != self.positions.len()
            || self.uvs.len() != self.positions.len()
            || self.light_uvs.len() != self.positions.len()
            || self.colors.len() != self.positions.len()
        {
            return None;
        }

        for quad in 0..quad_count {
            let source_base = quad * 4;
            let is_dirty = dirty.contains_quad(&self.positions, &self.normals, source_base);
            if is_dirty != keep_dirty {
                continue;
            }

            let target_base = output.positions.len() as u32;
            output
                .positions
                .extend_from_slice(&self.positions[source_base..source_base + 4]);
            output
                .normals
                .extend_from_slice(&self.normals[source_base..source_base + 4]);
            output
                .uvs
                .extend_from_slice(&self.uvs[source_base..source_base + 4]);
            output
                .light_uvs
                .extend_from_slice(&self.light_uvs[source_base..source_base + 4]);
            output
                .colors
                .extend_from_slice(&self.colors[source_base..source_base + 4]);

            let source_index_base = quad * 6;
            for &index in &self.indices[source_index_base..source_index_base + 6] {
                let relative = index.checked_sub(source_base as u32)?;
                if relative >= 4 {
                    return None;
                }
                output.indices.push(target_base + relative);
            }
        }

        Some(())
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
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.light_uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(indices)
    }
}

fn float32x2(values: &VertexAttributeValues) -> Option<&[[f32; 2]]> {
    match values {
        VertexAttributeValues::Float32x2(values) => Some(values),
        _ => None,
    }
}

fn float32x3(values: &VertexAttributeValues) -> Option<&[[f32; 3]]> {
    match values {
        VertexAttributeValues::Float32x3(values) => Some(values),
        _ => None,
    }
}

fn float32x4(values: &VertexAttributeValues) -> Option<&[[f32; 4]]> {
    match values {
        VertexAttributeValues::Float32x4(values) => Some(values),
        _ => None,
    }
}
