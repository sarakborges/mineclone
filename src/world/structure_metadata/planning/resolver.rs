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

// Conflict resolution is intentionally intent-based rather than acceptance-based.
// A target-intersecting candidate loses to any higher-ranked conflicting intent,
// even when that intent lies outside the target bounds and would not itself be
// materialized by this query. This keeps the decision independent of which chunk
// asks first and avoids a recursive/global conflict walk whose result could vary
// with the requested region.
//
// Collector contract: every call must append all candidates whose full placement
// bounds intersect the requested rectangle. The resolver first collects candidates
// intersecting the target, then asks again over each direct candidate's full bounds
// so cross-boundary reservations participate in the decision.
//
// Ranking is deterministic: priority descending, then placement id, biome id,
// placement anchor X/Z, and placement Y ascending. `reserve_space` is directional:
// only a higher-ranked reserving intent blocks a lower-ranked intent solely by
// reservation. Shared conflict groups are symmetric once ranking picks the winner.
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn structure_definition(id: &str) -> StructureDefinition {
        serde_json::from_value(json!({
            "id": id,
            "name": {
                "english": id,
                "portuguese_brazil": id,
                "spanish": id
            },
            "locatable": false,
            "rotation": false,
            "anchor": {"x": 0, "y": 0, "z": 0},
            "palette": {
                "S": {"block": "test:block"}
            },
            "layers": [{"y": 0, "rows": ["S"]}]
        }))
        .expect("test structure definition must deserialize")
    }

    fn candidate<'a>(
        placement_id: &'a str,
        structure: &'a StructureDefinition,
        priority: i32,
        reserve_space: bool,
        conflict_groups: &'a [String],
        bounds: (IVec2, IVec2),
    ) -> StructureCandidate<'a> {
        StructureCandidate {
            biome_id: "test:biome",
            placement_id,
            structure,
            rotation: StructureRotation::Degrees0,
            placement_anchor: bounds.0,
            placement_y: 0,
            anchor: bounds.0,
            origin_y: 0,
            primary_placement_piece: true,
            priority,
            reserve_space,
            conflict_groups,
            minimum: bounds.0,
            maximum: bounds.1,
            minimum_y: 0,
            maximum_y: 3,
        }
    }

    fn resolve_from_candidates<'a>(
        target_minimum: IVec2,
        target_maximum: IVec2,
        candidates: &[StructureCandidate<'a>],
    ) -> Vec<ResolvedStructurePlacement> {
        resolve_structure_placements(
            target_minimum,
            target_maximum,
            |minimum, maximum, found| {
                found.extend(candidates.iter().copied().filter(|candidate| {
                    rectangles_overlap(
                        candidate.minimum,
                        candidate.maximum,
                        minimum,
                        maximum,
                    )
                }));
            },
        )
    }

    #[test]
    fn higher_priority_reservation_outside_target_blocks_cross_boundary_candidate() {
        let lower = structure_definition("test:lower");
        let higher = structure_definition("test:higher");
        let candidates = [
            candidate(
                "test:lower-placement",
                &lower,
                1,
                false,
                &[],
                (IVec2::new(14, 0), IVec2::new(17, 3)),
            ),
            candidate(
                "test:higher-placement",
                &higher,
                10,
                true,
                &[],
                (IVec2::new(16, 0), IVec2::new(19, 3)),
            ),
        ];

        let resolved = resolve_from_candidates(
            IVec2::ZERO,
            IVec2::new(15, 15),
            &candidates,
        );

        assert!(resolved.is_empty());
    }

    #[test]
    fn shared_conflict_group_outside_target_blocks_lower_priority_candidate() {
        let lower = structure_definition("test:lower");
        let higher = structure_definition("test:higher");
        let conflict_groups = vec!["test:landmark".to_owned()];
        let candidates = [
            candidate(
                "test:lower-placement",
                &lower,
                1,
                false,
                &conflict_groups,
                (IVec2::new(14, 0), IVec2::new(17, 3)),
            ),
            candidate(
                "test:higher-placement",
                &higher,
                10,
                false,
                &conflict_groups,
                (IVec2::new(16, 0), IVec2::new(19, 3)),
            ),
        ];

        let resolved = resolve_from_candidates(
            IVec2::ZERO,
            IVec2::new(15, 15),
            &candidates,
        );

        assert!(resolved.is_empty());
    }

    #[test]
    fn unrelated_cross_boundary_intent_does_not_block_candidate() {
        let lower = structure_definition("test:lower");
        let higher = structure_definition("test:higher");
        let lower_groups = vec!["test:foliage".to_owned()];
        let higher_groups = vec!["test:landmark".to_owned()];
        let candidates = [
            candidate(
                "test:lower-placement",
                &lower,
                1,
                false,
                &lower_groups,
                (IVec2::new(14, 0), IVec2::new(17, 3)),
            ),
            candidate(
                "test:higher-placement",
                &higher,
                10,
                false,
                &higher_groups,
                (IVec2::new(16, 0), IVec2::new(19, 3)),
            ),
        ];

        let resolved = resolve_from_candidates(
            IVec2::ZERO,
            IVec2::new(15, 15),
            &candidates,
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].placement_id, "test:lower-placement");
    }

    #[test]
    fn lower_priority_reservation_does_not_block_higher_priority_candidate() {
        let higher = structure_definition("test:higher");
        let lower = structure_definition("test:lower");
        let candidates = [
            candidate(
                "test:higher-placement",
                &higher,
                10,
                false,
                &[],
                (IVec2::new(14, 0), IVec2::new(17, 3)),
            ),
            candidate(
                "test:lower-placement",
                &lower,
                1,
                true,
                &[],
                (IVec2::new(16, 0), IVec2::new(19, 3)),
            ),
        ];

        let resolved = resolve_from_candidates(
            IVec2::ZERO,
            IVec2::new(15, 15),
            &candidates,
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].placement_id, "test:higher-placement");
    }

    #[test]
    fn equal_priority_cross_boundary_conflict_uses_deterministic_tie_breaker() {
        let direct = structure_definition("test:direct");
        let outside = structure_definition("test:outside");
        let candidates = [
            candidate(
                "test:zeta-placement",
                &direct,
                5,
                false,
                &[],
                (IVec2::new(14, 0), IVec2::new(17, 3)),
            ),
            candidate(
                "test:alpha-placement",
                &outside,
                5,
                true,
                &[],
                (IVec2::new(16, 0), IVec2::new(19, 3)),
            ),
        ];

        let resolved = resolve_from_candidates(
            IVec2::ZERO,
            IVec2::new(15, 15),
            &candidates,
        );

        assert!(resolved.is_empty());
    }
}
