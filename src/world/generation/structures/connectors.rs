use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::content::structure::{
    StructureDefinition, StructureRegistry, StructureRotation, opposite_connector_face,
};

use super::hash::{avalanche, string_hash};

const MAX_CONNECTOR_CHAIN_DEPTH: usize = 64;
const CONNECTOR_INDEX_SALT: u64 = 0x9e37_79b1_85eb_ca87;
const CONNECTOR_DEPTH_SALT: u64 = 0xc2b2_ae3d_27d4_eb4f;
const CONNECTOR_ROTATION_SALT: u64 = 0x1656_67b1_9e37_79f9;

#[derive(Clone, Copy)]
pub(super) struct ResolvedConnectedPiece<'a> {
    pub(super) structure: &'a StructureDefinition,
    pub(super) rotation: StructureRotation,
    pub(super) origin: IVec3,
}

#[derive(Clone, Copy)]
struct PendingPiece {
    index: usize,
    remaining_strength: Option<f32>,
    depth: usize,
}

pub(super) fn resolve_connected_pieces<'a>(
    world_seed: u64,
    root: &'a StructureDefinition,
    root_rotation: StructureRotation,
    root_origin: IVec3,
    structures: &'a StructureRegistry,
) -> Vec<ResolvedConnectedPiece<'a>> {
    let root_piece = ResolvedConnectedPiece {
        structure: root,
        rotation: root_rotation,
        origin: root_origin,
    };
    let root_connectors = root.connector_points();
    if !root_connectors
        .iter()
        .any(|connector| connector.target.is_some())
    {
        return vec![root_piece];
    }

    let mut pieces = vec![root_piece];
    let mut occupied = HashSet::new();
    occupy_piece(root_piece, &mut occupied);

    let mut pending = VecDeque::from([PendingPiece {
        index: 0,
        remaining_strength: None,
        depth: 0,
    }]);

    while let Some(state) = pending.pop_front() {
        if state.depth >= MAX_CONNECTOR_CHAIN_DEPTH {
            continue;
        }

        let parent = pieces[state.index];
        for (connector_index, output) in parent
            .structure
            .connector_points()
            .into_iter()
            .enumerate()
            .filter(|(_, connector)| connector.target.is_some())
        {
            let effective_strength = state
                .remaining_strength
                .map_or(output.strength, |remaining| remaining.min(output.strength));
            if effective_strength <= 0.0 {
                continue;
            }

            let hash = connector_hash(
                world_seed,
                parent,
                connector_index,
                state.depth,
            );
            let target = output
                .target
                .as_deref()
                .expect("filtered output connector target must exist");
            let Some(child) = structures.select_for_reference(target, hash) else {
                continue;
            };

            let world_position =
                parent.origin + parent.rotation.rotate_offset(output.offset);
            let required_input_face =
                opposite_connector_face(parent.rotation.rotate_face(output.face));
            let Some((child_rotation, child_origin)) = child.resolve_input_attachment(
                world_position,
                required_input_face,
                hash.rotate_left(23),
            ) else {
                continue;
            };

            let child_piece = ResolvedConnectedPiece {
                structure: child,
                rotation: child_rotation,
                origin: child_origin,
            };
            let child_positions = transformed_voxel_positions(child_piece);
            if child_positions
                .iter()
                .any(|position| occupied.contains(position))
            {
                continue;
            }

            occupied.extend(child_positions);
            let child_index = pieces.len();
            pieces.push(child_piece);

            let next_strength =
                effective_strength - output.strength_loss_on_each_loop;
            if next_strength > 0.0 && state.depth + 1 < MAX_CONNECTOR_CHAIN_DEPTH {
                pending.push_back(PendingPiece {
                    index: child_index,
                    remaining_strength: Some(next_strength),
                    depth: state.depth + 1,
                });
            }
        }
    }

    pieces
}

fn occupy_piece(piece: ResolvedConnectedPiece<'_>, occupied: &mut HashSet<IVec3>) {
    occupied.extend(transformed_voxel_positions(piece));
}

fn transformed_voxel_positions(piece: ResolvedConnectedPiece<'_>) -> Vec<IVec3> {
    piece
        .structure
        .voxels()
        .iter()
        .map(|voxel| piece.origin + piece.rotation.rotate_offset(voxel.offset))
        .collect()
}

fn connector_hash(
    world_seed: u64,
    parent: ResolvedConnectedPiece<'_>,
    connector_index: usize,
    depth: usize,
) -> u64 {
    let mut hash = world_seed ^ string_hash(&parent.structure.id).rotate_left(17);
    hash ^= (parent.origin.x as i64 as u64).wrapping_mul(CONNECTOR_INDEX_SALT);
    hash ^= (parent.origin.y as i64 as u64).wrapping_mul(CONNECTOR_DEPTH_SALT);
    hash ^= (parent.origin.z as i64 as u64).wrapping_mul(CONNECTOR_ROTATION_SALT);
    hash ^= u64::try_from(connector_index + 1)
        .expect("connector index must fit in u64")
        .wrapping_mul(CONNECTOR_INDEX_SALT.rotate_left(7));
    hash ^= u64::try_from(depth + 1)
        .expect("connector depth must fit in u64")
        .wrapping_mul(CONNECTOR_DEPTH_SALT.rotate_left(13));
    hash ^= u64::from(rotation_index(parent.rotation) + 1)
        .wrapping_mul(CONNECTOR_ROTATION_SALT.rotate_left(29));
    avalanche(hash)
}

fn rotation_index(rotation: StructureRotation) -> u8 {
    match rotation {
        StructureRotation::Degrees0 => 0,
        StructureRotation::Degrees90 => 1,
        StructureRotation::Degrees180 => 2,
        StructureRotation::Degrees270 => 3,
    }
}
