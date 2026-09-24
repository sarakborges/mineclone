use std::collections::{HashMap, HashSet, VecDeque};

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
pub(crate) struct ResolvedConnectedPiece<'a> {
    pub(crate) structure: &'a StructureDefinition,
    pub(crate) rotation: StructureRotation,
    pub(crate) origin: IVec3,
}

#[derive(Clone, Copy)]
struct PendingPiece {
    index: usize,
    remaining_strength: Option<f32>,
    depth: usize,
}

pub(super) fn connected_horizontal_bounds_for_reference(
    structures: &StructureRegistry,
    reference: &str,
) -> Option<(IVec2, IVec2)> {
    let members = structures.reference_members(reference)?;
    let mut memo = HashMap::new();
    let mut bounds = None;

    for structure in members {
        for &rotation in structure.supported_rotations() {
            let piece_bounds = connected_horizontal_bounds_for_piece(
                structure,
                rotation,
                None,
                0,
                structures,
                &mut memo,
            );
            extend_bounds(&mut bounds, piece_bounds);
        }
    }

    bounds
}

fn connected_horizontal_bounds_for_piece<'a>(
    structure: &'a StructureDefinition,
    rotation: StructureRotation,
    remaining_strength: Option<f32>,
    depth: usize,
    structures: &'a StructureRegistry,
    memo: &mut HashMap<(&'a str, StructureRotation, u32, usize), (IVec2, IVec2)>,
) -> (IVec2, IVec2) {
    let remaining_key = remaining_strength.map_or(u32::MAX, f32::to_bits);
    let key = (structure.id.as_str(), rotation, remaining_key, depth);
    if let Some(bounds) = memo.get(&key) {
        return *bounds;
    }

    let mut bounds = structure.horizontal_bounds_for_rotation(rotation);
    if depth >= MAX_CONNECTOR_CHAIN_DEPTH {
        memo.insert(key, bounds);
        return bounds;
    }

    for output in structure
        .connector_points()
        .iter()
        .filter(|connector| connector.target.is_some())
    {
        let effective_strength = remaining_strength
            .map_or(output.strength, |remaining| remaining.min(output.strength));
        if effective_strength <= 0.0 {
            continue;
        }

        let target = output
            .target
            .as_deref()
            .expect("filtered output connector target must exist");
        let Some(members) = structures.reference_members(target) else {
            continue;
        };
        let connector_position = rotation.rotate_offset(output.offset);
        let required_input_face =
            opposite_connector_face(rotation.rotate_face(output.face));
        let next_strength =
            (effective_strength - output.strength_loss_on_each_loop).max(0.0);

        for child in members {
            for (child_rotation, child_origin) in
                child.compatible_input_attachments(connector_position, required_input_face)
            {
                let child_bounds = connected_horizontal_bounds_for_piece(
                    child,
                    child_rotation,
                    Some(next_strength),
                    depth + 1,
                    structures,
                    memo,
                );
                let translated = (
                    child_origin.xz() + child_bounds.0,
                    child_origin.xz() + child_bounds.1,
                );
                bounds.0 = bounds.0.min(translated.0);
                bounds.1 = bounds.1.max(translated.1);
            }
        }
    }

    memo.insert(key, bounds);
    bounds
}

fn extend_bounds(
    bounds: &mut Option<(IVec2, IVec2)>,
    candidate: (IVec2, IVec2),
) {
    *bounds = Some(match *bounds {
        Some((minimum, maximum)) => (
            minimum.min(candidate.0),
            maximum.max(candidate.1),
        ),
        None => candidate,
    });
}

pub(crate) fn resolve_connected_pieces<'a>(
    world_seed: u64,
    root: &'a StructureDefinition,
    root_rotation: StructureRotation,
    root_origin: IVec3,
    structures: &'a StructureRegistry,
) -> Vec<ResolvedConnectedPiece<'a>> {
    resolve_connected_piece_forest(
        world_seed,
        [ResolvedConnectedPiece {
            structure: root,
            rotation: root_rotation,
            origin: root_origin,
        }],
        structures,
    )
}

pub(crate) fn resolve_connected_piece_forest<'a>(
    world_seed: u64,
    roots: impl IntoIterator<Item = ResolvedConnectedPiece<'a>>,
    structures: &'a StructureRegistry,
) -> Vec<ResolvedConnectedPiece<'a>> {
    let roots = roots.into_iter().collect::<Vec<_>>();
    if roots.is_empty() {
        return Vec::new();
    }
    if roots.iter().all(|root| {
        !root
            .structure
            .connector_points()
            .iter()
            .any(|connector| connector.target.is_some())
    }) {
        return roots;
    }

    let mut occupied = HashSet::new();
    for root in &roots {
        occupy_piece(*root, &mut occupied);
    }

    let mut resolved = Vec::new();
    for root in roots {
        resolved.extend(resolve_connected_branch(
            world_seed,
            root,
            structures,
            &mut occupied,
        ));
    }
    resolved
}

fn resolve_connected_branch<'a>(
    world_seed: u64,
    root: ResolvedConnectedPiece<'a>,
    structures: &'a StructureRegistry,
    occupied: &mut HashSet<IVec3>,
) -> Vec<ResolvedConnectedPiece<'a>> {
    let mut pieces = vec![root];
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::{Value, json};

    use super::*;

    fn localized(name: &str) -> Value {
        json!({
            "english": name,
            "portuguese_brazil": name,
            "spanish": name
        })
    }

    fn registry(definitions: Vec<Value>) -> StructureRegistry {
        let mut registry = StructureRegistry::default();
        for definition in definitions {
            registry.insert(
                serde_json::from_value(definition)
                    .expect("test structure definition must deserialize"),
            );
        }
        registry
    }

    fn root_definition(target: &str, loss: f32) -> Value {
        json!({
            "id": "test:root",
            "name": localized("Root"),
            "locatable": false,
            "rotation": false,
            "anchor": {"x": 0, "y": 0, "z": 0},
            "palette": {
                "S": {"block": "test:block"},
                "O": {
                    "connector": {
                        "target": target,
                        "face": "right",
                        "strength": 1.0,
                        "strengthLossOnEachLoop": loss
                    }
                }
            },
            "layers": [{"y": 0, "rows": ["SO"]}]
        })
    }

    fn segment_definition(id: &str, group_id: Option<&str>, chained: bool) -> Value {
        let mut definition = json!({
            "id": id,
            "name": localized("Segment"),
            "locatable": false,
            "rotation": true,
            "anchor": {"x": 1, "y": 0, "z": 0},
            "palette": {
                "I": {"connector": {"face": "left"}},
                "S": {"block": "test:block"}
            },
            "layers": [{"y": 0, "rows": ["IS"]}]
        });
        if let Some(group_id) = group_id {
            definition["group_id"] = json!(group_id);
        }
        if chained {
            definition["palette"]["O"] = json!({
                "connector": {
                    "target": id,
                    "face": "right",
                    "strength": 1.0,
                    "strengthLossOnEachLoop": 0.25
                }
            });
            definition["layers"][0]["rows"] = json!(["ISO"]);
        }
        definition
    }

    #[test]
    fn output_connector_can_decorate_a_persistent_voxel() {
        let definition = json!({
            "id": "test:decorated_output",
            "name": localized("Decorated Output"),
            "locatable": false,
            "rotation": false,
            "anchor": {"x": 0, "y": 0, "z": 0},
            "palette": {
                "P": {
                    "block": "test:block",
                    "connector": {
                        "target": "test:child",
                        "face": "right",
                        "strength": 1.0,
                        "strengthLossOnEachLoop": 1.0
                    }
                }
            },
            "layers": [{"y": 0, "rows": ["P"]}]
        });
        let mut structure: StructureDefinition =
            serde_json::from_value(definition).expect("decorated output must deserialize");
        structure.validate_layout();
        structure.rebuild_runtime();

        assert_eq!(structure.voxels().len(), 1);
        assert_eq!(structure.connector_points().len(), 1);
        assert_eq!(structure.voxels()[0].offset, IVec3::ZERO);
        assert_eq!(structure.connector_points()[0].offset, IVec3::ZERO);
    }

    #[test]
    fn straight_chain_terminates_from_strength_loss() {
        let registry = registry(vec![
            root_definition("test:segment", 0.25),
            segment_definition("test:segment", None, true),
        ]);
        let root = registry.get("test:root").expect("root must exist");

        let pieces = resolve_connected_pieces(
            7,
            root,
            StructureRotation::Degrees0,
            IVec3::ZERO,
            &registry,
        );

        assert_eq!(pieces.len(), 5);
        assert_eq!(
            pieces.iter().map(|piece| piece.origin.x).collect::<Vec<_>>(),
            vec![0, 2, 4, 6, 8]
        );
    }

    #[test]
    fn rotated_parent_aligns_child_input_face_and_position() {
        let registry = registry(vec![
            root_definition("test:segment", 1.0),
            segment_definition("test:segment", None, false),
        ]);
        let root = registry.get("test:root").expect("root must exist");

        let pieces = resolve_connected_pieces(
            11,
            root,
            StructureRotation::Degrees90,
            IVec3::ZERO,
            &registry,
        );

        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[1].rotation, StructureRotation::Degrees90);
        assert_eq!(pieces[1].origin, IVec3::new(0, 0, 2));
    }

    #[test]
    fn connector_bounds_cover_the_full_possible_chain() {
        let registry = registry(vec![
            root_definition("test:segment", 0.25),
            segment_definition("test:segment", None, true),
        ]);

        assert_eq!(
            connected_horizontal_bounds_for_reference(&registry, "test:root"),
            Some((IVec2::ZERO, IVec2::new(8, 0)))
        );
    }

    #[test]
    fn group_targets_are_deterministic_and_can_select_each_member() {
        let registry = registry(vec![
            root_definition("test:segments", 1.0),
            segment_definition("test:segment_a", Some("segments"), false),
            segment_definition("test:segment_b", Some("segments"), false),
        ]);
        let root = registry.get("test:root").expect("root must exist");
        let mut selected = HashSet::new();

        for seed in 0..64 {
            let first = resolve_connected_pieces(
                seed,
                root,
                StructureRotation::Degrees0,
                IVec3::ZERO,
                &registry,
            );
            let second = resolve_connected_pieces(
                seed,
                root,
                StructureRotation::Degrees0,
                IVec3::ZERO,
                &registry,
            );
            assert_eq!(
                first.iter().map(|piece| piece.structure.id.as_str()).collect::<Vec<_>>(),
                second.iter().map(|piece| piece.structure.id.as_str()).collect::<Vec<_>>()
            );
            selected.insert(first[1].structure.id.clone());
        }

        assert_eq!(
            selected,
            HashSet::from([
                "test:segment_a".to_owned(),
                "test:segment_b".to_owned()
            ])
        );
    }

    #[test]
    fn child_that_overlaps_existing_voxels_is_rejected() {
        let overlapping_child = json!({
            "id": "test:overlap",
            "name": localized("Overlap"),
            "locatable": false,
            "rotation": false,
            "anchor": {"x": 0, "y": 0, "z": 0},
            "palette": {
                "S": {"block": "test:block"},
                "I": {"connector": {"face": "left"}}
            },
            "layers": [{"y": 0, "rows": ["SI"]}]
        });
        let registry = registry(vec![
            root_definition("test:overlap", 1.0),
            overlapping_child,
        ]);
        let root = registry.get("test:root").expect("root must exist");

        let pieces = resolve_connected_pieces(
            3,
            root,
            StructureRotation::Degrees0,
            IVec3::ZERO,
            &registry,
        );

        assert_eq!(pieces.len(), 1);
    }
}
