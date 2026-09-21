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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
        let source = center - normal * 0.001;
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

pub(crate) fn patch_voxel_mesh(
    existing: &Mesh,
    replacement: Option<&Mesh>,
    dirty: ChunkMeshletMask,
) -> Option<Mesh> {
    if dirty.is_all() {
        return Some(match replacement {
            Some(mesh) => MeshArrays::from_mesh(mesh)?.into_mesh(),
            None => MeshArrays::default().into_mesh(),
        });
    }

    let mut output = MeshArrays::default();
    let existing = MeshArrays::from_mesh(existing)?;
    existing.append_filtered(&mut output, dirty, false)?;

    if let Some(replacement) = replacement {
        MeshArrays::from_mesh(replacement)?.append_filtered(&mut output, dirty, true)?;
    }

    Some(output.into_mesh())
}

#[derive(Default)]
struct MeshArrays {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    light_uvs: Vec<[f32; 2]>,
    tangents: Vec<[f32; 4]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshArrays {
    fn from_mesh(mesh: &Mesh) -> Option<Self> {
        Some(Self {
            positions: float32x3(mesh.attribute(Mesh::ATTRIBUTE_POSITION)?)?.to_vec(),
            normals: float32x3(mesh.attribute(Mesh::ATTRIBUTE_NORMAL)?)?.to_vec(),
            uvs: float32x2(mesh.attribute(Mesh::ATTRIBUTE_UV_0)?)?.to_vec(),
            light_uvs: float32x2(mesh.attribute(Mesh::ATTRIBUTE_UV_1)?)?.to_vec(),
            tangents: float32x4(mesh.attribute(Mesh::ATTRIBUTE_TANGENT)?)?.to_vec(),
            colors: float32x4(mesh.attribute(Mesh::ATTRIBUTE_COLOR)?)?.to_vec(),
            indices: mesh.indices()?.iter().collect(),
        })
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
            || self.tangents.len() != self.positions.len()
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
                .tangents
                .extend_from_slice(&self.tangents[source_base..source_base + 4]);
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
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.light_uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, self.tangents)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
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
