use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use bevy::prelude::{IVec2, IVec3};

use crate::content::{
    layer::LayerFace,
    structure::{
        StructureDefinition, StructureRegistry, StructureRotation, opposite_connector_face,
    },
};

use super::StructurePlacement;
use crate::world::generator::foundation::{
    GenerationDomain, GenerationEntropy, GenerationPoint3,
};

const CONNECTOR_DOMAIN: &str = "structure/connector/expansion/v1";
const CONNECTOR_INDEX_SALT: u64 = 0x9e37_79b1_85eb_ca87;
const CONNECTOR_DEPTH_SALT: u64 = 0xc2b2_ae3d_27d4_eb4f;
const CONNECTOR_ROTATION_SALT: u64 = 0x1656_67b1_9e37_79f9;

type ConnectorBoundKey = (String, StructureRotation, u32);

#[derive(Clone, Debug)]
pub(super) struct ConnectorGraph {
    members: Arc<HashMap<String, Arc<[Arc<StructureDefinition>]>>>,
    entropy_domain: GenerationDomain,
}

#[derive(Clone, Copy)]
struct PendingPiece {
    index: usize,
    remaining_strength: Option<f32>,
    depth: u32,
}

impl ConnectorGraph {
    pub(super) fn new(structures: &StructureRegistry) -> Self {
        let definitions = structures
            .iter()
            .map(|structure| (structure.id.clone(), Arc::new(structure.clone())))
            .collect::<HashMap<_, _>>();
        let mut members = HashMap::<String, Vec<Arc<StructureDefinition>>>::new();

        for structure in definitions.values() {
            members
                .entry(structure.id.clone())
                .or_default()
                .push(Arc::clone(structure));
            if let Some(group_id) = structure.group_id.as_deref() {
                let reference = structure.id.split_once(':').map_or_else(
                    || group_id.to_owned(),
                    |(namespace, _)| format!("{namespace}:{group_id}"),
                );
                members
                    .entry(reference)
                    .or_default()
                    .push(Arc::clone(structure));
            }
        }

        let members = members
            .into_iter()
            .map(|(reference, mut variants)| {
                variants.sort_unstable_by(|left, right| left.id.cmp(&right.id));
                (reference, Arc::<[Arc<StructureDefinition>]>::from(variants))
            })
            .collect();

        Self {
            members: Arc::new(members),
            entropy_domain: GenerationDomain::named(CONNECTOR_DOMAIN),
        }
    }

    pub(super) fn connected_horizontal_bounds_for_reference(
        &self,
        reference: &str,
    ) -> Option<(IVec2, IVec2)> {
        let members = self.members.get(reference)?;
        let mut memo = HashMap::<ConnectorBoundKey, (IVec2, IVec2)>::new();
        let mut active = HashSet::<ConnectorBoundKey>::new();
        let mut bounds = None;

        for structure in members.iter() {
            for &rotation in structure.supported_rotations() {
                let candidate = self.connected_horizontal_bounds_for_piece(
                    structure,
                    rotation,
                    1.0,
                    &mut memo,
                    &mut active,
                );
                extend_bounds(&mut bounds, candidate);
            }
        }

        bounds
    }

    fn connected_horizontal_bounds_for_piece(
        &self,
        structure: &StructureDefinition,
        rotation: StructureRotation,
        remaining_strength: f32,
        memo: &mut HashMap<ConnectorBoundKey, (IVec2, IVec2)>,
        active: &mut HashSet<ConnectorBoundKey>,
    ) -> (IVec2, IVec2) {
        let key = (
            structure.id.clone(),
            rotation,
            remaining_strength.to_bits(),
        );
        if let Some(bounds) = memo.get(&key) {
            return *bounds;
        }

        let mut bounds = structure.horizontal_bounds_for_rotation(rotation);
        if remaining_strength <= 0.0 {
            memo.insert(key, bounds);
            return bounds;
        }

        assert!(
            active.insert(key.clone()),
            "Structure connector graph reaches {} with unchanged strength {:.6}; recursive connector cycles must lose strength so placement bounds remain finite",
            structure.id,
            remaining_strength
        );

        for output in structure
            .connector_points()
            .iter()
            .filter(|connector| connector.target.is_some())
        {
            let effective_strength = remaining_strength.min(output.strength);
            if effective_strength <= 0.0 {
                continue;
            }

            let target = output
                .target
                .as_deref()
                .expect("filtered output connector target must exist");
            let variants = self
                .members
                .get(target)
                .expect("validated connector target must resolve in compiled graph");
            let world_face = rotation.rotate_face(output.face);
            let connector_position = rotation.rotate_offset(output.offset);
            let required_input_face = opposite_connector_face(world_face);
            let next_strength = (effective_strength - output.strength_loss_on_each_loop).max(0.0);

            for distance in connector_bound_distances(output.min_distance, output.max_distance) {
                let attachment_position = connector_position
                    + connector_face_offset(world_face)
                        * i32::try_from(distance)
                            .expect("validated connector distance must fit i32");
                for child in variants.iter() {
                    for (child_rotation, child_origin) in
                        child.compatible_input_attachments(attachment_position, required_input_face)
                    {
                        let child_bounds = self.connected_horizontal_bounds_for_piece(
                            child,
                            child_rotation,
                            next_strength,
                            memo,
                            active,
                        );
                        let child_horizontal = IVec2::new(child_origin.x, child_origin.z);
                        bounds.0 = bounds.0.min(child_horizontal + child_bounds.0);
                        bounds.1 = bounds.1.max(child_horizontal + child_bounds.1);
                    }
                }
            }
        }

        active.remove(&key);
        memo.insert(key, bounds);
        bounds
    }

    pub(super) fn expand(
        &self,
        entropy: GenerationEntropy,
        mut pieces: Vec<StructurePlacement>,
        mut resolve_child_origin: impl FnMut(
            &StructureDefinition,
            StructureRotation,
            IVec3,
        ) -> Option<IVec3>,
    ) -> Vec<StructurePlacement> {
        if pieces.is_empty()
            || pieces.iter().all(|piece| {
                !piece
                    .structure
                    .connector_points()
                    .iter()
                    .any(|connector| connector.target.is_some())
            })
        {
            return pieces;
        }

        let mut occupied = HashSet::<IVec3>::new();
        for piece in &pieces {
            occupied.extend(transformed_voxel_positions(piece));
        }

        let mut pending = (0..pieces.len())
            .map(|index| PendingPiece {
                index,
                remaining_strength: None,
                depth: 0,
            })
            .collect::<VecDeque<_>>();

        while let Some(state) = pending.pop_front() {
            let parent = pieces[state.index].clone();
            for (connector_index, output) in parent
                .structure
                .connector_points()
                .iter()
                .enumerate()
                .filter(|(_, connector)| connector.target.is_some())
            {
                let effective_strength = state
                    .remaining_strength
                    .map_or(output.strength, |remaining| remaining.min(output.strength));
                if effective_strength <= 0.0 {
                    continue;
                }

                let hash = self.connector_hash(entropy, &parent, connector_index, state.depth);
                let target = output
                    .target
                    .as_deref()
                    .expect("filtered output connector target must exist");
                let variants = self
                    .members
                    .get(target)
                    .expect("validated connector target must resolve in compiled graph");
                let variant_count = u64::try_from(variants.len())
                    .expect("connector target variant count must fit u64");
                let child_index = usize::try_from(hash % variant_count)
                    .expect("connector target variant index must fit usize");
                let child = Arc::clone(
                    variants
                        .get(child_index)
                        .expect("connector target variant index must exist"),
                );

                let world_face = parent.rotation.rotate_face(output.face);
                let distance = connector_distance(
                    hash.rotate_left(11),
                    output.min_distance,
                    output.max_distance,
                );
                let world_position = parent.origin
                    + parent.rotation.rotate_offset(output.offset)
                    + connector_face_offset(world_face)
                        * i32::try_from(distance)
                            .expect("validated connector distance must fit i32");
                let required_input_face = opposite_connector_face(world_face);
                let Some((child_rotation, geometric_origin)) = child.resolve_input_attachment(
                    world_position,
                    required_input_face,
                    hash.rotate_left(23),
                ) else {
                    continue;
                };
                let Some(child_origin) =
                    resolve_child_origin(&child, child_rotation, geometric_origin)
                else {
                    continue;
                };

                let child_piece = StructurePlacement {
                    reference: Arc::clone(&parent.reference),
                    placement_anchor: parent.placement_anchor,
                    structure: child,
                    rotation: child_rotation,
                    origin: child_origin,
                };
                let child_positions = transformed_voxel_positions(&child_piece).collect::<Vec<_>>();
                if child_positions
                    .iter()
                    .any(|position| occupied.contains(position))
                {
                    continue;
                }

                occupied.extend(child_positions);
                let piece_index = pieces.len();
                pieces.push(child_piece);

                let next_strength = effective_strength - output.strength_loss_on_each_loop;
                if next_strength > 0.0 {
                    pending.push_back(PendingPiece {
                        index: piece_index,
                        remaining_strength: Some(next_strength),
                        depth: state.depth.saturating_add(1),
                    });
                }
            }
        }

        pieces
    }

    fn connector_hash(
        &self,
        entropy: GenerationEntropy,
        parent: &StructurePlacement,
        connector_index: usize,
        depth: u32,
    ) -> u64 {
        let mut hash = entropy.sample_3d(
            self.entropy_domain,
            GenerationPoint3::new(parent.origin.x, parent.origin.y, parent.origin.z),
        );
        hash ^= parent.structure.runtime_hash().rotate_left(17);
        hash ^= u64::try_from(connector_index + 1)
            .expect("connector index must fit u64")
            .wrapping_mul(CONNECTOR_INDEX_SALT);
        hash ^= u64::from(depth)
            .saturating_add(1)
            .wrapping_mul(CONNECTOR_DEPTH_SALT);
        hash ^= u64::from(rotation_index(parent.rotation) + 1)
            .wrapping_mul(CONNECTOR_ROTATION_SALT);
        avalanche(hash)
    }
}

fn extend_bounds(bounds: &mut Option<(IVec2, IVec2)>, candidate: (IVec2, IVec2)) {
    *bounds = Some(match *bounds {
        Some((minimum, maximum)) => (minimum.min(candidate.0), maximum.max(candidate.1)),
        None => candidate,
    });
}

fn connector_distance(hash: u64, minimum: u32, maximum: u32) -> u32 {
    if minimum == maximum {
        return minimum;
    }
    let span = u64::from(maximum - minimum) + 1;
    minimum + u32::try_from(hash % span).expect("connector distance must fit u32")
}

fn connector_bound_distances(minimum: u32, maximum: u32) -> [u32; 2] {
    [minimum, maximum]
}

fn connector_face_offset(face: LayerFace) -> IVec3 {
    match face {
        LayerFace::Right => IVec3::X,
        LayerFace::Left => IVec3::NEG_X,
        LayerFace::Top => IVec3::Y,
        LayerFace::Bottom => IVec3::NEG_Y,
        LayerFace::Front => IVec3::Z,
        LayerFace::Back => IVec3::NEG_Z,
    }
}

fn transformed_voxel_positions(piece: &StructurePlacement) -> impl Iterator<Item = IVec3> + '_ {
    piece
        .structure
        .voxels()
        .iter()
        .map(|voxel| piece.origin + piece.rotation.rotate_offset(voxel.offset))
}

fn rotation_index(rotation: StructureRotation) -> u8 {
    match rotation {
        StructureRotation::Degrees0 => 0,
        StructureRotation::Degrees90 => 1,
        StructureRotation::Degrees180 => 2,
        StructureRotation::Degrees270 => 3,
    }
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_distance_stays_in_authored_range() {
        for hash in 0..128 {
            assert!((3..=7).contains(&connector_distance(hash, 3, 7)));
        }
    }

    #[test]
    fn fixed_connector_distance_is_exact() {
        assert_eq!(connector_distance(42, 5, 5), 5);
    }
}
