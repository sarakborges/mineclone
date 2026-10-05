use std::{collections::HashMap, fmt, sync::Arc};

use bevy::prelude::{IVec2, IVec3};

use crate::{
    content::{
        block::BlockRegistry,
        block_id::intern_block_id,
        fluid::FluidRegistry,
        layer::LayerFace,
        structure::{
            StructureDefinition, StructureRegistry, StructureRotation, StructureSurfaceLayer,
            StructureVoxel,
        },
        structure_rules::{StructureFluidPolicy, StructureReplacePolicy},
    },
    voxel::{
        cell::VoxelCell,
        chunk::{CHUNK_SIZE, CHUNK_VOLUME, VoxelChunk, VoxelChunkStructureMut},
        fluid::{FluidCell, MAX_FLUID_LEVEL},
        layer::LayerCell,
        texture_rotation::TextureRotation,
    },
};

use super::{
    foundation::{
        GenerationChunkCoord, GenerationDomain, GenerationEntropy, GenerationPoint3,
        GenerationSnapshot,
    },
    material::MaterialField,
    structure::{StructureField, StructurePlacement},
};

const COLUMN_INDEX_MIN_VOXELS: usize = 512;
const STRUCTURE_OCCUPANCY_WORDS: usize = CHUNK_VOLUME.div_ceil(u64::BITS as usize);
const FLUID_SPREAD_TARGETS: [IVec3; 5] =
    [IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
const SURFACE_LAYER_PRESENCE_DOMAIN_PREFIX: &str =
    "chunk/structure/surface-layer/presence/v1";
const SURFACE_LAYER_ROTATION_DOMAIN_PREFIX: &str =
    "chunk/structure/surface-layer/rotation/v1";

/// Runtime encoding inputs frozen when the playable world's generator is installed.
///
/// Block/fluid registries translate already-authoritative generated identities into
/// compact runtime cells. They do not participate in biome, terrain, material, or
/// Structure placement decisions.
pub(super) struct ChunkRuntimeContent<'a> {
    pub(super) blocks: &'a BlockRegistry,
    pub(super) fluids: &'a FluidRegistry,
}

/// One runtime-fluid wake candidate exposed by generated chunk content.
///
/// Targets crossing the requested chunk boundary remain potential until the runtime
/// scheduler sees the neighboring materialized state. `enqueue_generated_fluid_frontier`
/// is responsible for revalidating them; synthesis never schedules every filled voxel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedFluidFrontierTarget {
    fluid: Arc<str>,
    position: IVec3,
}

impl GeneratedFluidFrontierTarget {
    pub(crate) fn fluid(&self) -> &str {
        &self.fluid
    }

    pub(crate) const fn position(&self) -> IVec3 {
        self.position
    }
}

/// Complete deterministic runtime representation of one generated chunk request.
///
/// The contained `VoxelChunk` is a materialized value, never a semantic generation
/// owner. The generator remains authoritative until Phase 9 persistence accepts this
/// result into playable world state.
pub(crate) struct MaterializedChunk {
    chunk: VoxelChunk,
    generated_fluid_frontiers: Vec<GeneratedFluidFrontierTarget>,
}

impl MaterializedChunk {
    pub(crate) fn chunk(&self) -> &VoxelChunk {
        &self.chunk
    }

    pub(crate) fn into_chunk(self) -> VoxelChunk {
        self.chunk
    }

    pub(crate) fn generated_fluid_frontiers(&self) -> &[GeneratedFluidFrontierTarget] {
        &self.generated_fluid_frontiers
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SurfaceLayerDomainKey {
    structure_hash: u64,
    layer_hash: u64,
    rotation_hash: u64,
    face: u8,
}

#[derive(Clone, Copy, Debug)]
struct SurfaceLayerDomains {
    presence: GenerationDomain,
    rotation: GenerationDomain,
}

/// Phase-7 adapter from immutable semantic generation owners to runtime chunk cells.
#[derive(Clone)]
pub(super) struct ChunkMaterializer {
    entropy: GenerationEntropy,
    materials: Arc<MaterialField>,
    structures: Arc<StructureField>,
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    surface_layer_domains: Arc<HashMap<SurfaceLayerDomainKey, SurfaceLayerDomains>>,
}

impl fmt::Debug for ChunkMaterializer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChunkMaterializer")
            .field("surface_layer_domain_count", &self.surface_layer_domains.len())
            .finish_non_exhaustive()
    }
}

impl ChunkMaterializer {
    pub(super) fn new(
        snapshot: &GenerationSnapshot,
        materials: Arc<MaterialField>,
        structures: Arc<StructureField>,
        structure_registry: &StructureRegistry,
        content: ChunkRuntimeContent<'_>,
    ) -> Self {
        Self {
            entropy: GenerationEntropy::new(snapshot),
            materials,
            structures,
            blocks: content.blocks.clone(),
            fluids: content.fluids.clone(),
            surface_layer_domains: Arc::new(compile_surface_layer_domains(structure_registry)),
        }
    }

    pub(super) fn materialize(&self, runtime_coord: IVec3) -> MaterializedChunk {
        let bounds = GenerationChunkCoord::new(runtime_coord.x, runtime_coord.y, runtime_coord.z)
            .world_bounds()
            .unwrap_or_else(|| {
                panic!(
                    "cannot materialize chunk {runtime_coord:?}: world-space origin exceeds i32 range"
                )
            });
        let generation_origin = bounds.origin();
        let origin = IVec3::new(
            generation_origin.x(),
            generation_origin.y(),
            generation_origin.z(),
        );
        let edge = CHUNK_SIZE as u32;
        let material_queries = self.materials.queries();
        let solids = material_queries.sample_solid_volume(
            origin.x, origin.y, origin.z, edge, edge, edge,
        );
        let generated_fluids = material_queries.sample_generated_fluid_volume(
            origin.x, origin.y, origin.z, edge, edge, edge,
        );
        let mut chunk = VoxelChunk::empty();

        chunk.edit_initial_blocks(|content| {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let Some(generated) = solids.solid_block_at(x as u32, y as u32, z as u32)
                        else {
                            continue;
                        };
                        let definition = self.blocks.get(generated.as_str()).unwrap_or_else(|| {
                            panic!(
                                "generated material references missing block {}",
                                generated.as_str()
                            )
                        });
                        let world_position = origin + local_position(x, y, z);
                        let block_id = intern_block_id(&definition.id);
                        let texture_rotation = TextureRotation::for_position(
                            world_position,
                            definition.rotate_texture.any(),
                        );
                        content.set_block(
                            x,
                            y,
                            z,
                            VoxelCell::new(block_id, texture_rotation),
                        );
                    }
                }
            }
        });

        chunk.edit_initial_fluids(|content| {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let Some(generated) = generated_fluids.generated_fluid_at(
                            x as u32,
                            y as u32,
                            z as u32,
                        ) else {
                            continue;
                        };
                        assert!(
                            solids.solid_block_at(x as u32, y as u32, z as u32).is_none(),
                            "generated fluid {} overlaps generated solid terrain at {:?}",
                            generated.as_str(),
                            origin + local_position(x, y, z)
                        );
                        let fluid_id = self.fluids.id_of(generated.as_str()).unwrap_or_else(|| {
                            panic!(
                                "generated fluid references missing fluid {}",
                                generated.as_str()
                            )
                        });
                        content.set_fluid(
                            x,
                            y,
                            z,
                            FluidCell::source(fluid_id, MAX_FLUID_LEVEL),
                        );
                    }
                }
            }
        });

        let chunk_maximum_y = origin
            .y
            .checked_add(CHUNK_SIZE as i32 - 1)
            .expect("validated generation chunk bounds must fit i32");
        let placements = self
            .structures
            .queries()
            .placements_intersecting(origin.x, origin.z, edge, edge)
            .into_iter()
            .filter(|placement| {
                let (minimum_y, maximum_y) = placement.vertical_bounds();
                maximum_y >= origin.y && minimum_y <= chunk_maximum_y
            })
            .collect::<Vec<_>>();
        self.rasterize_structures(&mut chunk, origin, &placements);

        let generated_fluid_frontiers = self.collect_generated_fluid_frontiers(&chunk, origin);
        MaterializedChunk {
            chunk,
            generated_fluid_frontiers,
        }
    }

    fn rasterize_structures(
        &self,
        chunk: &mut VoxelChunk,
        chunk_origin: IVec3,
        placements: &[StructurePlacement],
    ) {
        if placements.is_empty() {
            return;
        }

        let needs_base_occupied = placements.iter().any(|placement| {
            placement.structure().generation.replace_policy == StructureReplacePolicy::AirOnly
        });
        let mut base_occupied = [0_u64; STRUCTURE_OCCUPANCY_WORDS];
        if needs_base_occupied {
            chunk.visit_content_voxels(|x, y, z, block, fluid| {
                if block.is_some() || fluid.is_some() {
                    bit_set(&mut base_occupied, chunk_index(x, y, z));
                }
            });
        }

        let mut claimed = [0_u64; STRUCTURE_OCCUPANCY_WORDS];
        chunk.edit_structure_content(|content| {
            for placement in placements {
                self.rasterize_structure(
                    content,
                    &mut claimed,
                    &base_occupied,
                    chunk_origin,
                    placement,
                );
            }
        });
    }

    fn rasterize_structure(
        &self,
        chunk: &mut VoxelChunkStructureMut<'_>,
        claimed: &mut [u64; STRUCTURE_OCCUPANCY_WORDS],
        base_occupied: &[u64; STRUCTURE_OCCUPANCY_WORDS],
        chunk_origin: IVec3,
        placement: &StructurePlacement,
    ) {
        let structure = placement.structure();
        let rotation = placement.rotation();
        let origin = placement.origin();

        visit_structure_voxels_in_chunk(
            structure,
            rotation,
            origin,
            chunk_origin,
            |voxel, world_position, local| {
                let x = local.x as usize;
                let y = local.y as usize;
                let z = local.z as usize;
                let voxel_index = chunk_index(x, y, z);
                let can_replace = match structure.generation.replace_policy {
                    StructureReplacePolicy::Any => true,
                    StructureReplacePolicy::AirOnly => {
                        !bit_get(claimed, voxel_index)
                            && !bit_get(base_occupied, voxel_index)
                    }
                    StructureReplacePolicy::Terrain => !bit_get(claimed, voxel_index),
                };
                if !can_replace {
                    return false;
                }

                let attached_objects = structure.objects_for_voxel(voxel);
                let attachment_only = voxel.block_id.is_none()
                    && structure.fluid_for_voxel(voxel).is_none()
                    && !structure.clears_voxel(voxel)
                    && (structure.layers_only_voxel(voxel) || !attached_objects.is_empty());

                if attachment_only {
                    self.rasterize_attachment_only(
                        chunk,
                        structure,
                        rotation,
                        voxel,
                        attached_objects,
                        chunk_origin,
                        x,
                        y,
                        z,
                    );
                    return false;
                }

                if let Some(block_id) = voxel.block_id {
                    let block = self.blocks.get(block_id).unwrap_or_else(|| {
                        panic!(
                            "structure {} references missing block {block_id}",
                            structure.id
                        )
                    });
                    let texture_rotation = TextureRotation::for_position(
                        world_position,
                        block.rotate_texture.any(),
                    );
                    chunk.set_block(
                        x,
                        y,
                        z,
                        VoxelCell::oriented(
                            block_id,
                            texture_rotation,
                            rotation.rotate_orientation(voxel.orientation),
                        ),
                    );
                    for (face, layer) in self.surface_layer_placements(
                        structure,
                        rotation,
                        voxel,
                        world_position,
                    ) {
                        let _ = chunk.add_layer(x, y, z, face, layer);
                    }
                    if structure.generation.fluid_policy == StructureFluidPolicy::Displace {
                        chunk.clear_fluid(x, y, z);
                    }
                    for attached in attached_objects {
                        let object = attached.object_cell(rotation, world_position);
                        let _ = chunk.set_object(x, y, z, object);
                    }
                } else if let Some(fluid_reference) = structure.fluid_for_voxel(voxel) {
                    let fluid_id = self.fluids.id_of(fluid_reference).unwrap_or_else(|| {
                        panic!(
                            "structure {} references missing fluid {fluid_reference}",
                            structure.id
                        )
                    });
                    chunk.clear_block(x, y, z);
                    chunk.set_fluid(x, y, z, FluidCell::source(fluid_id, MAX_FLUID_LEVEL));
                } else {
                    debug_assert!(
                        structure.clears_voxel(voxel),
                        "validated Structure voxel must contain block, fluid, attachment, or clear payload"
                    );
                    chunk.clear_block(x, y, z);
                    chunk.clear_fluid(x, y, z);
                }

                bit_set(claimed, voxel_index);
                false
            },
        );

        for world_position in structure.clear_above_positions(rotation, origin) {
            let local = world_position - chunk_origin;
            if !local_in_bounds(local) {
                continue;
            }
            let x = local.x as usize;
            let y = local.y as usize;
            let z = local.z as usize;
            let voxel_index = chunk_index(x, y, z);
            if bit_get(claimed, voxel_index) {
                continue;
            }
            chunk.clear_block(x, y, z);
            chunk.clear_fluid(x, y, z);
            bit_set(claimed, voxel_index);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rasterize_attachment_only(
        &self,
        chunk: &mut VoxelChunkStructureMut<'_>,
        structure: &StructureDefinition,
        rotation: StructureRotation,
        voxel: &StructureVoxel,
        attached_objects: &[crate::content::structure::StructureAttachedObject],
        chunk_origin: IVec3,
        x: usize,
        y: usize,
        z: usize,
    ) {
        let max_rise = structure.restrictions.max_slope.max(0) as usize;
        let Some(support_y) = (y..=y.saturating_add(max_rise))
            .rev()
            .find(|candidate_y| {
                *candidate_y < CHUNK_SIZE && chunk.cell_at(x, *candidate_y, z).is_some()
            })
        else {
            return;
        };
        let support_world_position =
            chunk_origin + IVec3::new(x as i32, support_y as i32, z as i32);

        if structure.layers_only_voxel(voxel) {
            for (face, layer) in self.surface_layer_placements(
                structure,
                rotation,
                voxel,
                support_world_position,
            ) {
                let _ = chunk.add_layer(x, support_y, z, face, layer);
            }
        }

        if attached_objects.is_empty() || chunk.fluid_at(x, support_y, z).is_some() {
            return;
        }
        for attached in attached_objects {
            let object = attached.object_cell(rotation, support_world_position);
            let target = IVec3::new(x as i32, support_y as i32, z as i32) + object.face.normal();
            let target_has_fluid = local_in_bounds(target)
                && chunk
                    .fluid_at(target.x as usize, target.y as usize, target.z as usize)
                    .is_some();
            if !target_has_fluid {
                let _ = chunk.set_object(x, support_y, z, object);
            }
        }
    }

    fn surface_layer_placements(
        &self,
        structure: &StructureDefinition,
        structure_rotation: StructureRotation,
        voxel: &StructureVoxel,
        world_position: IVec3,
    ) -> Vec<(LayerFace, LayerCell)> {
        let mut placements = Vec::new();
        let point = GenerationPoint3::new(world_position.x, world_position.y, world_position.z);

        for surface in structure.surface_layers_for_voxel(voxel) {
            for &authored_face in &surface.faces {
                let face = structure_rotation.rotate_face(authored_face);
                let domains = self.surface_layer_domains(structure, surface, face);
                if !probability_selected(
                    self.entropy.sample_3d(domains.presence, point),
                    surface.chance,
                ) {
                    continue;
                }
                let rotation_hash = self.entropy.sample_3d(domains.rotation, point);
                let rotation = TextureRotation::from_quarter_turn((rotation_hash & 3) as u8);
                placements.push((face, LayerCell::new(&surface.layer, rotation)));
            }
        }
        placements
    }

    fn surface_layer_domains(
        &self,
        structure: &StructureDefinition,
        surface: &StructureSurfaceLayer,
        face: LayerFace,
    ) -> SurfaceLayerDomains {
        let key = SurfaceLayerDomainKey {
            structure_hash: structure.runtime_hash(),
            layer_hash: surface.runtime_hash(),
            rotation_hash: surface.runtime_rotation_hash(),
            face: face.index(),
        };
        *self.surface_layer_domains.get(&key).unwrap_or_else(|| {
            panic!(
                "missing compiled surface-layer entropy domains for structure {} layer {} face {:?}",
                structure.id, surface.layer, face
            )
        })
    }

    fn collect_generated_fluid_frontiers(
        &self,
        chunk: &VoxelChunk,
        chunk_origin: IVec3,
    ) -> Vec<GeneratedFluidFrontierTarget> {
        let mut frontiers = Vec::new();
        chunk.visit_potential_fluid_frontier_sources(|source, fluid| {
            let definition = self.fluids.get(fluid.fluid_id).unwrap_or_else(|| {
                panic!(
                    "materialized chunk contains unknown fluid id {}",
                    fluid.fluid_id
                )
            });
            let fluid_name: Arc<str> = Arc::from(definition.id.as_str());
            for offset in FLUID_SPREAD_TARGETS {
                let target = source + offset;
                let available = if local_in_bounds(target) {
                    let (block, target_fluid) = chunk.content_at_local(
                        target.x as usize,
                        target.y as usize,
                        target.z as usize,
                    );
                    block.is_none() && target_fluid.is_none()
                } else {
                    // Neighbor content is deliberately not materialized recursively here.
                    // Publication/runtime scheduling revalidates this potential boundary target.
                    true
                };
                if available {
                    frontiers.push(GeneratedFluidFrontierTarget {
                        fluid: Arc::clone(&fluid_name),
                        position: chunk_origin + target,
                    });
                }
            }
        });
        frontiers.sort_by(|left, right| {
            left.position
                .y
                .cmp(&right.position.y)
                .then_with(|| left.position.z.cmp(&right.position.z))
                .then_with(|| left.position.x.cmp(&right.position.x))
                .then_with(|| left.fluid.cmp(&right.fluid))
        });
        frontiers.dedup_by(|left, right| {
            left.position == right.position && left.fluid == right.fluid
        });
        frontiers
    }
}

fn compile_surface_layer_domains(
    structures: &StructureRegistry,
) -> HashMap<SurfaceLayerDomainKey, SurfaceLayerDomains> {
    let mut domains = HashMap::new();
    for structure in structures.iter() {
        for voxel in structure.voxels() {
            for surface in structure.surface_layers_for_voxel(voxel) {
                for face in LayerFace::ALL {
                    let key = SurfaceLayerDomainKey {
                        structure_hash: structure.runtime_hash(),
                        layer_hash: surface.runtime_hash(),
                        rotation_hash: surface.runtime_rotation_hash(),
                        face: face.index(),
                    };
                    domains.entry(key).or_insert_with(|| SurfaceLayerDomains {
                        presence: GenerationDomain::named(&format!(
                            "{SURFACE_LAYER_PRESENCE_DOMAIN_PREFIX}/{}/{}/{}",
                            structure.id,
                            surface.runtime_hash(),
                            face.index()
                        )),
                        rotation: GenerationDomain::named(&format!(
                            "{SURFACE_LAYER_ROTATION_DOMAIN_PREFIX}/{}/{}/{}",
                            structure.id,
                            surface.runtime_rotation_hash(),
                            face.index()
                        )),
                    });
                }
            }
        }
    }
    domains
}

fn visit_structure_voxels_in_chunk(
    structure: &StructureDefinition,
    rotation: StructureRotation,
    origin: IVec3,
    chunk_origin: IVec3,
    mut visit: impl FnMut(&StructureVoxel, IVec3, IVec3) -> bool,
) -> bool {
    let chunk_size = CHUNK_SIZE as i32;
    if structure.voxels().len() < COLUMN_INDEX_MIN_VOXELS {
        for voxel in structure.voxels() {
            let world_position = origin + rotation.rotate_offset(voxel.offset);
            let local = world_position - chunk_origin;
            if !local_in_bounds(local) {
                continue;
            }
            if visit(voxel, world_position, local) {
                return true;
            }
        }
        return false;
    }

    let origin_horizontal = IVec2::new(origin.x, origin.z);
    let chunk_horizontal = IVec2::new(chunk_origin.x, chunk_origin.z);
    for local_z in 0..chunk_size {
        for local_x in 0..chunk_size {
            let world_horizontal = chunk_horizontal + IVec2::new(local_x, local_z);
            let rotated_offset = world_horizontal - origin_horizontal;
            let structure_offset = rotation.inverse().rotate_horizontal(rotated_offset);
            for voxel in structure.column_voxels(structure_offset) {
                let world_y = origin.y + voxel.offset.y;
                let local_y = world_y - chunk_origin.y;
                if local_y < 0 {
                    continue;
                }
                if local_y >= chunk_size {
                    break;
                }
                let local = IVec3::new(local_x, local_y, local_z);
                let world_position = IVec3::new(world_horizontal.x, world_y, world_horizontal.y);
                if visit(voxel, world_position, local) {
                    return true;
                }
            }
        }
    }
    false
}

fn probability_selected(value: u64, chance: f32) -> bool {
    if chance <= 0.0 {
        return false;
    }
    if chance >= 1.0 {
        return true;
    }
    value as f64 / (u64::MAX as f64) < f64::from(chance)
}

fn local_position(x: usize, y: usize, z: usize) -> IVec3 {
    IVec3::new(x as i32, y as i32, z as i32)
}

fn local_in_bounds(position: IVec3) -> bool {
    let edge = CHUNK_SIZE as i32;
    position.x >= 0
        && position.y >= 0
        && position.z >= 0
        && position.x < edge
        && position.y < edge
        && position.z < edge
}

fn chunk_index(x: usize, y: usize, z: usize) -> usize {
    x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE
}

fn bit_get(bits: &[u64; STRUCTURE_OCCUPANCY_WORDS], index: usize) -> bool {
    bits[index / u64::BITS as usize] & (1_u64 << (index % u64::BITS as usize)) != 0
}

fn bit_set(bits: &mut [u64; STRUCTURE_OCCUPANCY_WORDS], index: usize) {
    bits[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
}
