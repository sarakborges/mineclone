use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::{
    structure::{StructureDefinition, StructureRegistry, StructureRotation},
    structure_set::StructureSetDefinition,
};

use super::{geometry::rectangles_overlap, hash::{avalanche, string_hash, unit_interval}};

const ELEMENT_HASH_SALT: u64 = 0x9e37_79b1_85eb_ca87;
const INSTANCE_HASH_SALT: u64 = 0xc2b2_ae3d_27d4_eb4f;
const ATTEMPT_HASH_SALT: u64 = 0x1656_67b1_9e37_79f9;

#[derive(Clone, Copy)]
pub(crate) struct ResolvedSetPiece<'a> {
    pub(crate) structure: &'a StructureDefinition,
    pub(crate) rotation: StructureRotation,
    pub(crate) anchor: IVec2,
    pub(crate) origin_y: i32,
    pub(crate) minimum: IVec2,
    pub(crate) maximum: IVec2,
}

pub(crate) fn resolve_set_pieces<'a>(
    world_seed: u64,
    set: &StructureSetDefinition,
    set_anchor: IVec2,
    structures: &'a StructureRegistry,
    mut resolve_origin_y: impl FnMut(
        &StructureDefinition,
        StructureRotation,
        IVec2,
    ) -> Option<i32>,
) -> Option<Vec<ResolvedSetPiece<'a>>> {
    let occurrence_hash = set_occurrence_hash(world_seed, &set.id, set_anchor);
    let mut resolved: Vec<ResolvedSetPiece<'a>> = Vec::new();
    let mut anchors_by_element = HashMap::<&str, Vec<IVec2>>::new();
    let mut all_anchors = Vec::<IVec2>::new();

    for (element_index, element) in set.elements.iter().enumerate() {
        let element_hash = avalanche(
            occurrence_hash
                ^ string_hash(&element.id).rotate_left(17)
                ^ (element_index as u64).wrapping_mul(ELEMENT_HASH_SALT),
        );

        if unit_interval(element_hash) >= element.chance {
            if element.required && element.count.min > 0 {
                return None;
            }
            anchors_by_element.insert(element.id.as_str(), Vec::new());
            continue;
        }

        let target_count = choose_count(element.count.min, element.count.max, element_hash);
        let mut placed_for_element = Vec::new();

        for instance in 0..target_count {
            let instance_hash = avalanche(
                element_hash
                    ^ u64::from(instance + 1).wrapping_mul(INSTANCE_HASH_SALT),
            );
            let structure = structures
                .select_for_reference(&element.structure, instance_hash)
                .expect("validated structure set element reference must resolve");
            let rotation = structure.rotation_for_hash(instance_hash.rotate_left(23));
            let (minimum_offset, maximum_offset) =
                structure.horizontal_bounds_for_rotation(rotation);

            let mut placed = None;
            for attempt in 0..element.placement.attempts {
                let attempt_hash = avalanche(
                    instance_hash
                        ^ u64::from(attempt + 1).wrapping_mul(ATTEMPT_HASH_SALT),
                );
                let Some(reference_anchor) = reference_anchor(
                    &element.placement.relative_to,
                    set_anchor,
                    &all_anchors,
                    &anchors_by_element,
                    attempt_hash,
                ) else {
                    continue;
                };
                let Some(offset) = annulus_offset(
                    attempt_hash.rotate_left(13),
                    element.placement.min_distance,
                    element.placement.max_distance,
                ) else {
                    continue;
                };
                let Some(anchor) = checked_add(reference_anchor, offset) else {
                    continue;
                };

                if !separation_satisfied(
                    anchor,
                    &all_anchors,
                    element.placement.min_separation,
                ) {
                    continue;
                }

                let Some(minimum) = checked_add(anchor, minimum_offset) else {
                    continue;
                };
                let Some(maximum) = checked_add(anchor, maximum_offset) else {
                    continue;
                };
                if !element.placement.allow_overlap
                    && resolved.iter().any(|piece| {
                        rectangles_overlap(minimum, maximum, piece.minimum, piece.maximum)
                    })
                {
                    continue;
                }

                let Some(origin_y) = resolve_origin_y(structure, rotation, anchor) else {
                    continue;
                };
                placed = Some(ResolvedSetPiece {
                    structure,
                    rotation,
                    anchor,
                    origin_y,
                    minimum,
                    maximum,
                });
                break;
            }

            let Some(piece) = placed else {
                continue;
            };
            placed_for_element.push(piece.anchor);
            all_anchors.push(piece.anchor);
            resolved.push(piece);
        }

        if element.required && placed_for_element.len() < element.count.min as usize {
            return None;
        }
        anchors_by_element.insert(element.id.as_str(), placed_for_element);
    }

    (!resolved.is_empty()).then_some(resolved)
}

fn choose_count(minimum: u32, maximum: u32, hash: u64) -> u32 {
    if minimum == maximum {
        return minimum;
    }
    let span = u64::from(maximum - minimum) + 1;
    minimum + (avalanche(hash.rotate_left(31)) % span) as u32
}

fn reference_anchor(
    relative_to: &str,
    origin: IVec2,
    all_anchors: &[IVec2],
    anchors_by_element: &HashMap<&str, Vec<IVec2>>,
    hash: u64,
) -> Option<IVec2> {
    match relative_to {
        "origin" => Some(origin),
        "any" if all_anchors.is_empty() => Some(origin),
        "any" => Some(all_anchors[(hash as usize) % all_anchors.len()]),
        element => {
            let anchors = anchors_by_element.get(element)?;
            (!anchors.is_empty()).then(|| anchors[(hash as usize) % anchors.len()])
        }
    }
}

fn annulus_offset(hash: u64, minimum_distance: u32, maximum_distance: u32) -> Option<IVec2> {
    if maximum_distance == 0 {
        return (minimum_distance == 0).then_some(IVec2::ZERO);
    }

    let radius = i32::try_from(maximum_distance).ok()?;
    let diameter = i64::from(radius) * 2 + 1;
    let x = (hash % diameter as u64) as i64 - i64::from(radius);
    let z_hash = avalanche(hash.rotate_left(29));
    let z = (z_hash % diameter as u64) as i64 - i64::from(radius);
    let distance_squared = x * x + z * z;
    let minimum_squared = i64::from(minimum_distance) * i64::from(minimum_distance);
    let maximum_squared = i64::from(maximum_distance) * i64::from(maximum_distance);

    if distance_squared < minimum_squared || distance_squared > maximum_squared {
        return None;
    }

    Some(IVec2::new(x as i32, z as i32))
}

fn separation_satisfied(anchor: IVec2, existing: &[IVec2], minimum_separation: u32) -> bool {
    if minimum_separation == 0 {
        return true;
    }
    let minimum_squared = i64::from(minimum_separation) * i64::from(minimum_separation);
    existing.iter().all(|other| {
        let dx = i64::from(anchor.x) - i64::from(other.x);
        let dz = i64::from(anchor.y) - i64::from(other.y);
        dx * dx + dz * dz >= minimum_squared
    })
}

fn checked_add(left: IVec2, right: IVec2) -> Option<IVec2> {
    Some(IVec2::new(
        left.x.checked_add(right.x)?,
        left.y.checked_add(right.y)?,
    ))
}

fn set_occurrence_hash(world_seed: u64, set_id: &str, anchor: IVec2) -> u64 {
    let mut hash = world_seed ^ string_hash(set_id).rotate_left(11);
    hash ^= (anchor.x as i64 as u64).wrapping_mul(ELEMENT_HASH_SALT);
    hash ^= (anchor.y as i64 as u64).wrapping_mul(INSTANCE_HASH_SALT);
    avalanche(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annulus_offset_respects_requested_distance() {
        for hash in 0..128 {
            if let Some(offset) = annulus_offset(hash, 3, 7) {
                let distance_squared =
                    i64::from(offset.x) * i64::from(offset.x)
                        + i64::from(offset.y) * i64::from(offset.y);
                assert!((9..=49).contains(&distance_squared));
            }
        }
    }

    #[test]
    fn zero_radius_only_resolves_origin() {
        assert_eq!(annulus_offset(42, 0, 0), Some(IVec2::ZERO));
        assert_eq!(annulus_offset(42, 1, 0), None);
    }
}
