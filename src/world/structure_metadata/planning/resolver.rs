use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use bevy::prelude::*;

use crate::content::structure::{StructureDefinition, StructureRotation};

use super::geometry::rectangles_overlap;
use super::super::{
    ResolvedStructurePlacement, ResolvedStructurePlan, ResolvedStructurePlanPiece,
};

#[derive(Clone, Copy)]
pub(crate) struct StructureCandidate<'a> {
    pub(crate) biome_id: &'a str,
    pub(crate) placement_id: &'a str,
    pub(crate) structure: &'a StructureDefinition,
    pub(crate) rotation: StructureRotation,
    pub(crate) placement_anchor: IVec2,
    pub(crate) placement_y: i32,
    pub(crate) anchor: IVec2,
    pub(crate) origin_y: i32,
    pub(crate) primary_placement_piece: bool,
    pub(crate) priority: i32,
    pub(crate) reserve_space: bool,
    pub(crate) conflict_groups: &'a [String],
    pub(crate) minimum: IVec2,
    pub(crate) maximum: IVec2,
    pub(crate) minimum_y: i32,
    pub(crate) maximum_y: i32,
}

pub(crate) fn resolve_structure_placements<'a>(
    target_minimum: IVec2,
    target_maximum: IVec2,
    mut collect_candidates: impl FnMut(IVec2, IVec2, &mut Vec<StructureCandidate<'a>>),
) -> Vec<ResolvedStructurePlacement> {
    let mut direct_candidates = Vec::new();
    collect_candidates(target_minimum, target_maximum, &mut direct_candidates);

    if direct_candidates.is_empty() {
        return Vec::new();
    }

    // Conflict resolution must see a higher-priority placement even when that
    // placement itself lies outside the target chunk. The collector is free to
    // use deterministic metadata to cheaply narrow which roots can intersect
    // the requested bounds before materializing terrain-dependent details.
    let mut competitors = direct_candidates.clone();
    let mut seen = competitors
        .iter()
        .map(candidate_identity)
        .collect::<HashSet<_>>();
    for direct in direct_candidates.iter().copied() {
        let mut overlapping = Vec::new();
        collect_candidates(direct.minimum, direct.maximum, &mut overlapping);
        for candidate in overlapping {
            if candidate.priority < direct.priority
                || !candidates_may_conflict(&candidate, &direct)
            {
                continue;
            }
            if seen.insert(candidate_identity(&candidate)) {
                competitors.push(candidate);
            }
        }
    }

    let mut accepted = direct_candidates
        .into_iter()
        .filter(|candidate| {
            !competitors.iter().any(|other| {
                !same_candidate(other, candidate)
                    && candidate_outranks(other, candidate)
                    && candidates_conflict(other, candidate)
            })
        })
        .collect::<Vec<_>>();
    accepted.sort_by(candidate_order);

    group_accepted_candidates(accepted)
}

fn group_accepted_candidates(
    accepted: Vec<StructureCandidate<'_>>,
) -> Vec<ResolvedStructurePlacement> {
    let mut placement_indices = HashMap::<(&str, &str, IVec2, i32), usize>::new();
    let mut placements = Vec::<ResolvedStructurePlacement>::new();

    for candidate in accepted {
        let key = (
            candidate.biome_id,
            candidate.placement_id,
            candidate.placement_anchor,
            candidate.placement_y,
        );
        let piece = ResolvedStructurePlanPiece {
            structure_id: candidate.structure.id.clone(),
            rotation: candidate.rotation,
            anchor: candidate.anchor,
            origin_y: candidate.origin_y,
            primary_placement_piece: candidate.primary_placement_piece,
        };

        if let Some(&index) = placement_indices.get(&key) {
            let placement = &mut placements[index];
            debug_assert_eq!(placement.plan.minimum, candidate.minimum);
            debug_assert_eq!(placement.plan.maximum, candidate.maximum);
            debug_assert_eq!(placement.plan.minimum_y, candidate.minimum_y);
            debug_assert_eq!(placement.plan.maximum_y, candidate.maximum_y);
            placement.plan.pieces.push(piece);
            continue;
        }

        placement_indices.insert(key, placements.len());
        placements.push(ResolvedStructurePlacement {
            placement_id: candidate.placement_id.to_owned(),
            placement_anchor: candidate.placement_anchor,
            placement_y: candidate.placement_y,
            plan: ResolvedStructurePlan {
                pieces: vec![piece],
                minimum: candidate.minimum,
                maximum: candidate.maximum,
                minimum_y: candidate.minimum_y,
                maximum_y: candidate.maximum_y,
            },
        });
    }

    placements
}

fn candidate_identity<'a>(
    candidate: &StructureCandidate<'a>,
) -> (
    &'a str,
    &'a str,
    IVec2,
    i32,
    &'a str,
    IVec2,
    StructureRotation,
) {
    (
        candidate.biome_id,
        candidate.placement_id,
        candidate.placement_anchor,
        candidate.placement_y,
        candidate.structure.id.as_str(),
        candidate.anchor,
        candidate.rotation,
    )
}

fn candidates_may_conflict(
    higher: &StructureCandidate<'_>,
    lower: &StructureCandidate<'_>,
) -> bool {
    higher.reserve_space
        || higher.conflict_groups.iter().any(|group| {
            lower
                .conflict_groups
                .iter()
                .any(|candidate| candidate == group)
        })
}

fn candidate_order(
    left: &StructureCandidate<'_>,
    right: &StructureCandidate<'_>,
) -> Ordering {
    right
        .priority
        .cmp(&left.priority)
        .then_with(|| left.placement_id.cmp(right.placement_id))
        .then_with(|| left.biome_id.cmp(right.biome_id))
        .then_with(|| left.placement_anchor.x.cmp(&right.placement_anchor.x))
        .then_with(|| left.placement_anchor.y.cmp(&right.placement_anchor.y))
        .then_with(|| left.placement_y.cmp(&right.placement_y))
}

fn candidate_outranks(
    left: &StructureCandidate<'_>,
    right: &StructureCandidate<'_>,
) -> bool {
    candidate_order(left, right) == Ordering::Less
}

fn same_candidate(
    left: &StructureCandidate<'_>,
    right: &StructureCandidate<'_>,
) -> bool {
    left.placement_id == right.placement_id
        && left.biome_id == right.biome_id
        && left.placement_anchor == right.placement_anchor
        && left.placement_y == right.placement_y
}

fn candidates_conflict(
    higher: &StructureCandidate<'_>,
    lower: &StructureCandidate<'_>,
) -> bool {
    rectangles_overlap(
        higher.minimum,
        higher.maximum,
        lower.minimum,
        lower.maximum,
    ) && higher.maximum_y >= lower.minimum_y
        && higher.minimum_y <= lower.maximum_y
        && candidates_may_conflict(higher, lower)
}
