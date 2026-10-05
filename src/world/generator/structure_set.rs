use std::{collections::HashMap, sync::Arc};

use bevy::prelude::IVec2;

use crate::content::{
    structure::{StructureDefinition, StructureRegistry, StructureRotation},
    structure_set::{
        StructureSetCount, StructureSetDefinition, StructureSetElementPlacement,
    },
};

use super::foundation::{GenerationDomain, GenerationEntropy, GenerationPoint2};

const ELEMENT_HASH_SALT: u64 = 0x9e37_79b1_85eb_ca87;
const INSTANCE_HASH_SALT: u64 = 0xc2b2_ae3d_27d4_eb4f;
const ATTEMPT_HASH_SALT: u64 = 0x1656_67b1_9e37_79f9;
const SET_ELEMENT_DOMAIN_PREFIX: &str = "structure/set/element/v1/";

#[derive(Clone, Debug)]
pub(super) struct CompiledStructureSet {
    id: Arc<str>,
    priority: i32,
    conflict_groups: Arc<[String]>,
    reserve_space: bool,
    elements: Arc<[CompiledSetElement]>,
    horizontal_minimum: IVec2,
    horizontal_maximum: IVec2,
}

#[derive(Clone, Debug)]
struct CompiledSetElement {
    id: Arc<str>,
    count: StructureSetCount,
    chance: f32,
    required: bool,
    placement: StructureSetElementPlacement,
    variants: Arc<[Arc<StructureDefinition>]>,
    entropy_domain: GenerationDomain,
}

#[derive(Clone, Debug)]
pub(super) struct ResolvedSetPiece {
    pub(super) structure: Arc<StructureDefinition>,
    pub(super) rotation: StructureRotation,
    pub(super) anchor: IVec2,
    pub(super) origin_y: i32,
    minimum: IVec2,
    maximum: IVec2,
}

impl CompiledStructureSet {
    pub(super) fn new(
        definition: &StructureSetDefinition,
        structures: &StructureRegistry,
        domain_suffix: &str,
    ) -> Self {
        let elements = definition
            .elements
            .iter()
            .map(|element| {
                let variants = structures
                    .reference_members(&element.structure)
                    .expect("validated StructureSet element reference must resolve")
                    .into_iter()
                    .map(|structure| Arc::new(structure.clone()))
                    .collect::<Vec<_>>();
                assert!(
                    !variants.is_empty(),
                    "StructureSet {} element {} must resolve at least one Structure",
                    definition.id,
                    element.id
                );
                CompiledSetElement {
                    id: Arc::from(element.id.as_str()),
                    count: element.count,
                    chance: element.chance,
                    required: element.required,
                    placement: element.placement.clone(),
                    variants: variants.into(),
                    entropy_domain: GenerationDomain::named(&format!(
                        "{SET_ELEMENT_DOMAIN_PREFIX}{domain_suffix}/{}",
                        element.id
                    )),
                }
            })
            .collect::<Vec<_>>()
            .into();

        let (horizontal_minimum, horizontal_maximum) = definition
            .horizontal_bounds_with(|reference| reference_bounds(structures, reference))
            .expect("validated StructureSet references must have horizontal bounds");

        Self {
            id: Arc::from(definition.id.as_str()),
            priority: definition.priority,
            conflict_groups: definition.conflict_groups.clone().into(),
            reserve_space: definition.reserve_space,
            elements,
            horizontal_minimum,
            horizontal_maximum,
        }
    }

    pub(super) fn priority(&self) -> i32 {
        self.priority
    }

    pub(super) fn conflict_groups(&self) -> Arc<[String]> {
        Arc::clone(&self.conflict_groups)
    }

    pub(super) const fn reserve_space(&self) -> bool {
        self.reserve_space
    }

    pub(super) const fn horizontal_bounds(&self) -> (IVec2, IVec2) {
        (self.horizontal_minimum, self.horizontal_maximum)
    }

    pub(super) fn resolve(
        &self,
        entropy: GenerationEntropy,
        cell: GenerationPoint2,
        set_anchor: IVec2,
        mut resolve_origin_y: impl FnMut(
            &StructureDefinition,
            StructureRotation,
            IVec2,
        ) -> Option<i32>,
    ) -> Option<Vec<ResolvedSetPiece>> {
        let mut resolved = Vec::<ResolvedSetPiece>::new();
        let mut anchors_by_element = HashMap::<&str, Vec<IVec2>>::new();
        let mut all_anchors = Vec::<IVec2>::new();

        for (element_index, element) in self.elements.iter().enumerate() {
            let element_hash = avalanche(
                entropy.sample_2d(element.entropy_domain, cell)
                    ^ (element_index as u64).wrapping_mul(ELEMENT_HASH_SALT),
            );

            if !chance_selects(element.chance, element_hash) {
                if element.required && element.count.min > 0 {
                    return None;
                }
                anchors_by_element.insert(element.id.as_ref(), Vec::new());
                continue;
            }

            let target_count = choose_count(element.count, element_hash);
            let mut placed_for_element = Vec::new();

            for instance in 0..target_count {
                let instance_hash = avalanche(
                    element_hash
                        ^ u64::from(instance + 1).wrapping_mul(INSTANCE_HASH_SALT),
                );
                let variant_count = u64::try_from(element.variants.len())
                    .expect("StructureSet variant count must fit u64");
                let variant_index = usize::try_from(instance_hash % variant_count).ok()?;
                let structure = Arc::clone(element.variants.get(variant_index)?);
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

                    let Some(origin_y) = resolve_origin_y(&structure, rotation, anchor) else {
                        continue;
                    };
                    placed = Some(ResolvedSetPiece {
                        structure: Arc::clone(&structure),
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
            anchors_by_element.insert(element.id.as_ref(), placed_for_element);
        }

        (!resolved.is_empty()).then_some(resolved)
    }
}

fn reference_bounds(
    structures: &StructureRegistry,
    reference: &str,
) -> Option<(IVec2, IVec2)> {
    let members = structures.reference_members(reference)?;
    let mut minimum = IVec2::splat(i32::MAX);
    let mut maximum = IVec2::splat(i32::MIN);
    let mut found = false;

    for structure in members {
        for rotation in structure.supported_rotations() {
            let (candidate_minimum, candidate_maximum) =
                structure.horizontal_bounds_for_rotation(*rotation);
            minimum = minimum.min(candidate_minimum);
            maximum = maximum.max(candidate_maximum);
            found = true;
        }
    }

    found.then_some((minimum, maximum))
}

fn choose_count(count: StructureSetCount, hash: u64) -> u32 {
    if count.min == count.max {
        return count.min;
    }
    let span = u64::from(count.max - count.min) + 1;
    count.min + (avalanche(hash.rotate_left(31)) % span) as u32
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

fn rectangles_overlap(
    left_minimum: IVec2,
    left_maximum: IVec2,
    right_minimum: IVec2,
    right_maximum: IVec2,
) -> bool {
    left_minimum.x <= right_maximum.x
        && left_maximum.x >= right_minimum.x
        && left_minimum.y <= right_maximum.y
        && left_maximum.y >= right_minimum.y
}

fn chance_selects(chance: f32, value: u64) -> bool {
    chance >= 1.0 || (chance > 0.0 && unit_probability(value) < f64::from(chance))
}

fn unit_probability(value: u64) -> f64 {
    value as f64 / u64::MAX as f64
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
