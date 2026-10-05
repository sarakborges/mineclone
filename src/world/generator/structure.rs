mod connector;

use std::{
    cmp::Ordering,
    collections::HashSet,
    sync::Arc,
};

use bevy::{
    math::Vec3Swizzles,
    prelude::{IVec2, IVec3},
};

use crate::content::{
    biome::BiomeRegistry,
    dimension::{GeneratedSurfaceStructureDefinition, GeneratedSurfaceStructurePlacement},
    structure::{StructureDefinition, StructureRegistry, StructureRotation},
    structure_rules::{StructureFluidPolicy, StructureProximityMode, StructureProximityTarget},
    structure_set::StructureSetRegistry,
};

use self::connector::ConnectorGraph;
use super::{
    biome::BiomeLayout,
    foundation::{GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationSnapshot},
    material::MaterialField,
    structure_set::CompiledStructureSet,
    terrain::TerrainField,
};

const PRESENCE_DOMAIN_PREFIX: &str = "structure/root/presence/v1/";
const JITTER_X_DOMAIN_PREFIX: &str = "structure/root/jitter-x/v1/";
const JITTER_Z_DOMAIN_PREFIX: &str = "structure/root/jitter-z/v1/";
const VARIANT_DOMAIN_PREFIX: &str = "structure/root/variant/v1/";
const ROTATION_DOMAIN_PREFIX: &str = "structure/root/rotation/v1/";
const MARGIN_SAMPLE_DIVISIONS: i32 = 16;

#[derive(Clone, Debug)]
pub(crate) struct StructurePlacement {
    reference: Arc<str>,
    placement_anchor: IVec2,
    structure: Arc<StructureDefinition>,
    rotation: StructureRotation,
    origin: IVec3,
}

impl StructurePlacement {
    pub(crate) fn reference(&self) -> &str {
        &self.reference
    }

    /// Logical root anchor shared by every piece of one placement. For a
    /// single Structure, StructureSet, or connector-expanded graph this remains
    /// the root placement anchor rather than an individual piece origin.
    pub(crate) const fn placement_anchor(&self) -> IVec2 {
        self.placement_anchor
    }

    pub(crate) fn structure(&self) -> &StructureDefinition {
        &self.structure
    }

    pub(crate) fn structure_id(&self) -> &str {
        &self.structure.id
    }

    pub(crate) const fn rotation(&self) -> StructureRotation {
        self.rotation
    }

    pub(crate) const fn origin(&self) -> IVec3 {
        self.origin
    }

    pub(crate) fn horizontal_bounds(&self) -> (IVec2, IVec2) {
        let (minimum, maximum) = self.structure.horizontal_bounds_for_rotation(self.rotation);
        (self.origin.xz() + minimum, self.origin.xz() + maximum)
    }

    pub(crate) fn vertical_bounds(&self) -> (i32, i32) {
        (
            self.origin.y.saturating_add(self.structure.min_y_offset()),
            self.origin
                .y
                .saturating_add(self.structure.effective_max_y_offset()),
        )
    }
}

#[derive(Clone, Debug)]
struct StructureCandidate {
    rule_index: usize,
    cell: GenerationPoint2,
    placement_anchor: IVec2,
    pieces: Arc<[StructurePlacement]>,
    horizontal_minimum: IVec2,
    horizontal_maximum: IVec2,
    minimum_y: i32,
    maximum_y: i32,
    priority: i32,
    reserve_space: bool,
    conflict_groups: Arc<[String]>,
}

impl StructureCandidate {
    const fn horizontal_bounds(&self) -> (IVec2, IVec2) {
        (self.horizontal_minimum, self.horizontal_maximum)
    }

    const fn vertical_bounds(&self) -> (i32, i32) {
        (self.minimum_y, self.maximum_y)
    }

    fn representative(&self) -> Option<StructurePlacement> {
        self.pieces.first().cloned()
    }
}

#[derive(Clone, Copy, Debug)]
struct RootAnchor {
    position: IVec2,
    forced_rotation: Option<StructureRotation>,
    margin_inside: Option<IVec2>,
}

#[derive(Clone, Copy)]
pub(crate) struct StructureQueries<'a> {
    field: &'a StructureField,
}

impl StructureQueries<'_> {
    /// Returns authoritative Structure pieces whose horizontal bounds intersect
    /// the requested world-space rectangle. StructureSet and connector graph
    /// planning plus conflict resolution happen before request-level filtering,
    /// so chunk/request boundaries never become logical placement boundaries.
    pub(crate) fn placements_intersecting(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
    ) -> Vec<StructurePlacement> {
        self.field
            .placements_intersecting(origin_x, origin_z, width, depth)
    }

    /// Finds the nearest authoritative logical root for an authored root
    /// reference. The returned representative piece carries the root reference
    /// and logical `placement_anchor`; callers must not treat its piece origin
    /// as the logical anchor for a multi-piece graph.
    pub(crate) fn find_nearest(
        &self,
        reference: &str,
        origin_x: i32,
        origin_z: i32,
        max_distance: u32,
    ) -> Option<StructurePlacement> {
        self.field
            .find_nearest(reference, origin_x, origin_z, max_distance)
    }
}

#[derive(Clone, Debug)]
pub(super) struct StructureField {
    entropy: GenerationEntropy,
    sea_level: i32,
    biomes: Arc<BiomeLayout>,
    terrain: Arc<TerrainField>,
    materials: Arc<MaterialField>,
    connectors: Arc<ConnectorGraph>,
    rules: Arc<[RootStructureRule]>,
}

#[derive(Clone, Debug)]
struct RootStructureRule {
    biome: Arc<str>,
    reference: Arc<str>,
    spacing: i32,
    jitter: u32,
    chance: f32,
    placement: GeneratedSurfaceStructurePlacement,
    source: RootStructureSource,
    maximum_horizontal_extent: i32,
    presence_domain: GenerationDomain,
    jitter_x_domain: GenerationDomain,
    jitter_z_domain: GenerationDomain,
}

#[derive(Clone, Debug)]
enum RootStructureSource {
    Direct {
        variants: Arc<[Arc<StructureDefinition>]>,
        variant_domain: GenerationDomain,
        rotation_domain: GenerationDomain,
    },
    Set(CompiledStructureSet),
}

pub(super) struct StructureAuthoring<'a> {
    pub(super) biomes: &'a BiomeRegistry,
    pub(super) structures: &'a StructureRegistry,
    pub(super) structure_sets: &'a StructureSetRegistry,
    pub(super) roots: &'a [GeneratedSurfaceStructureDefinition],
}

impl StructureField {
    pub(super) fn new(
        snapshot: &GenerationSnapshot,
        authoring: StructureAuthoring<'_>,
        biomes: Arc<BiomeLayout>,
        terrain: Arc<TerrainField>,
        materials: Arc<MaterialField>,
    ) -> Self {
        let dimension_id = snapshot.dimension().id();
        let connectors = Arc::new(ConnectorGraph::new(authoring.structures));
        let rules = authoring
            .roots
            .iter()
            .map(|definition| {
                let biome = authoring.biomes.get(&definition.biome).unwrap_or_else(|| {
                    panic!(
                        "dimension {dimension_id} generatedSurfaceStructures references missing biome {}",
                        definition.biome
                    )
                });
                assert!(
                    biome.belongs_to_dimension(dimension_id) && biome.surface_layout.is_some(),
                    "dimension {dimension_id} generatedSurfaceStructures biome {} must be a surface biome in this dimension",
                    definition.biome
                );
                RootStructureRule::new(
                    definition,
                    authoring.structures,
                    authoring.structure_sets,
                    &connectors,
                )
            })
            .collect::<Vec<_>>()
            .into();

        Self {
            entropy: GenerationEntropy::new(snapshot),
            sea_level: snapshot.dimension().sea_level(),
            biomes,
            terrain,
            materials,
            connectors,
            rules,
        }
    }

    pub(super) fn queries(&self) -> StructureQueries<'_> {
        StructureQueries { field: self }
    }

    fn placements_intersecting(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
    ) -> Vec<StructurePlacement> {
        assert!(width > 0 && depth > 0, "structure query area must be non-empty");
        let maximum_x = checked_area_max(origin_x, width, "X");
        let maximum_z = checked_area_max(origin_z, depth, "Z");
        let query_minimum = IVec2::new(origin_x, origin_z);
        let query_maximum = IVec2::new(maximum_x, maximum_z);
        let direct_candidates =
            self.collect_candidates_intersecting_bounds(query_minimum, query_maximum);
        let mut placements = Vec::new();

        for candidate in self.resolve_conflicts(direct_candidates) {
            placements.extend(candidate.pieces.iter().filter_map(|placement| {
                let (minimum, maximum) = placement.horizontal_bounds();
                rectangles_overlap(minimum, maximum, query_minimum, query_maximum)
                    .then(|| placement.clone())
            }));
        }

        placements.sort_by(|left, right| placement_sort_key(left).cmp(&placement_sort_key(right)));
        placements
    }

    fn collect_candidates_intersecting_bounds(
        &self,
        query_minimum: IVec2,
        query_maximum: IVec2,
    ) -> Vec<StructureCandidate> {
        debug_assert!(
            query_minimum.x <= query_maximum.x && query_minimum.y <= query_maximum.y,
            "structure query bounds must be ordered"
        );
        let mut candidates = Vec::new();

        for (rule_index, rule) in self.rules.iter().enumerate() {
            let padding = rule
                .maximum_horizontal_extent
                .saturating_add(i32::try_from(rule.jitter).unwrap_or(i32::MAX));
            let minimum = IVec2::new(
                query_minimum.x.saturating_sub(padding),
                query_minimum.y.saturating_sub(padding),
            );
            let maximum = IVec2::new(
                query_maximum.x.saturating_add(padding),
                query_maximum.y.saturating_add(padding),
            );
            let (minimum_cell, maximum_cell) = candidate_cell_bounds(rule.spacing, minimum, maximum);

            for cell_z in minimum_cell.y..=maximum_cell.y {
                for cell_x in minimum_cell.x..=maximum_cell.x {
                    let Some(candidate) = self.resolve_candidate(
                        rule_index,
                        rule,
                        GenerationPoint2::new(cell_x, cell_z),
                    ) else {
                        continue;
                    };
                    let (placement_minimum, placement_maximum) = candidate.horizontal_bounds();
                    if rectangles_overlap(
                        placement_minimum,
                        placement_maximum,
                        query_minimum,
                        query_maximum,
                    ) {
                        candidates.push(candidate);
                    }
                }
            }
        }

        candidates
    }

    fn resolve_conflicts(
        &self,
        direct_candidates: Vec<StructureCandidate>,
    ) -> Vec<StructureCandidate> {
        if direct_candidates.is_empty() {
            return direct_candidates;
        }

        let mut competitors = direct_candidates.clone();
        let mut seen = competitors
            .iter()
            .map(candidate_identity)
            .collect::<HashSet<_>>();

        for direct in &direct_candidates {
            let (minimum, maximum) = direct.horizontal_bounds();
            for candidate in self.collect_candidates_intersecting_bounds(minimum, maximum) {
                if same_candidate(&candidate, direct)
                    || !self.candidate_outranks(&candidate, direct)
                    || !candidates_conflict(&candidate, direct)
                {
                    continue;
                }
                if seen.insert(candidate_identity(&candidate)) {
                    competitors.push(candidate);
                }
            }
        }

        direct_candidates
            .into_iter()
            .filter(|candidate| {
                !competitors.iter().any(|other| {
                    !same_candidate(other, candidate)
                        && self.candidate_outranks(other, candidate)
                        && candidates_conflict(other, candidate)
                })
            })
            .collect()
    }

    fn find_nearest(
        &self,
        reference: &str,
        origin_x: i32,
        origin_z: i32,
        max_distance: u32,
    ) -> Option<StructurePlacement> {
        let origin = IVec2::new(origin_x, origin_z);
        let maximum_squared = i128::from(max_distance) * i128::from(max_distance);
        let radius = i32::try_from(max_distance).unwrap_or(i32::MAX);
        let minimum = IVec2::new(
            origin.x.saturating_sub(radius),
            origin.y.saturating_sub(radius),
        );
        let maximum = IVec2::new(
            origin.x.saturating_add(radius),
            origin.y.saturating_add(radius),
        );
        let mut best: Option<(i128, StructurePlacement)> = None;

        for (rule_index, rule) in self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.reference.as_ref() == reference)
        {
            let (minimum_cell, maximum_cell) = candidate_cell_bounds(rule.spacing, minimum, maximum);
            for cell_z in minimum_cell.y..=maximum_cell.y {
                for cell_x in minimum_cell.x..=maximum_cell.x {
                    let Some(candidate) = self.resolve_candidate(
                        rule_index,
                        rule,
                        GenerationPoint2::new(cell_x, cell_z),
                    ) else {
                        continue;
                    };
                    let delta = candidate.placement_anchor - origin;
                    let distance_squared = i128::from(delta.x) * i128::from(delta.x)
                        + i128::from(delta.y) * i128::from(delta.y);
                    if distance_squared > maximum_squared || !self.candidate_is_accepted(&candidate) {
                        continue;
                    }
                    let Some(placement) = candidate.representative() else {
                        continue;
                    };
                    let replace = best.as_ref().is_none_or(|(current_distance, current)| {
                        distance_squared < *current_distance
                            || (distance_squared == *current_distance
                                && placement_sort_key(&placement) < placement_sort_key(current))
                    });
                    if replace {
                        best = Some((distance_squared, placement));
                    }
                }
            }
        }

        best.map(|(_, placement)| placement)
    }

    fn candidate_is_accepted(&self, candidate: &StructureCandidate) -> bool {
        let (minimum, maximum) = candidate.horizontal_bounds();
        !self
            .collect_candidates_intersecting_bounds(minimum, maximum)
            .into_iter()
            .any(|other| {
                !same_candidate(&other, candidate)
                    && self.candidate_outranks(&other, candidate)
                    && candidates_conflict(&other, candidate)
            })
    }

    fn candidate_outranks(
        &self,
        left: &StructureCandidate,
        right: &StructureCandidate,
    ) -> bool {
        self.candidate_order(left, right) == Ordering::Less
    }

    fn candidate_order(
        &self,
        left: &StructureCandidate,
        right: &StructureCandidate,
    ) -> Ordering {
        let left_rule = &self.rules[left.rule_index];
        let right_rule = &self.rules[right.rule_index];
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| {
                left_rule
                    .reference
                    .as_ref()
                    .cmp(right_rule.reference.as_ref())
            })
            .then_with(|| left_rule.biome.as_ref().cmp(right_rule.biome.as_ref()))
            .then_with(|| left.placement_anchor.x.cmp(&right.placement_anchor.x))
            .then_with(|| left.placement_anchor.y.cmp(&right.placement_anchor.y))
            .then_with(|| left.minimum_y.cmp(&right.minimum_y))
    }

    fn resolve_candidate(
        &self,
        rule_index: usize,
        rule: &RootStructureRule,
        cell: GenerationPoint2,
    ) -> Option<StructureCandidate> {
        if unit_probability(self.entropy.sample_2d(rule.presence_domain, cell))
            > f64::from(rule.chance)
        {
            return None;
        }

        let hint = self.lattice_hint(rule, cell)?;
        let root = match rule.placement {
            GeneratedSurfaceStructurePlacement::BiomeInterior => {
                let biome_sample = self
                    .biomes
                    .queries()
                    .surface_biome_at(hint.x, hint.y);
                if biome_sample.primary().as_str() != rule.biome.as_ref() {
                    return None;
                }
                RootAnchor {
                    position: hint,
                    forced_rotation: None,
                    margin_inside: None,
                }
            }
            GeneratedSurfaceStructurePlacement::BiomeMargin => {
                self.find_biome_margin_anchor(rule, cell, hint)?
            }
        };
        let anchor = root.position;

        let (pieces, priority, reserve_space, conflict_groups) = match &rule.source {
            RootStructureSource::Direct {
                variants,
                variant_domain,
                rotation_domain,
            } => {
                let variant_hash = self.entropy.sample_2d(*variant_domain, cell);
                let variant_count =
                    u64::try_from(variants.len()).expect("structure variant count must fit u64");
                let variant_index = usize::try_from(variant_hash % variant_count).ok()?;
                let structure = Arc::clone(variants.get(variant_index)?);
                let rotation = root.forced_rotation.unwrap_or_else(|| {
                    structure.rotation_for_hash(self.entropy.sample_2d(*rotation_domain, cell))
                });
                let origin_y = self.fit_root_origin_y(rule, &structure, rotation, root)?;
                if !self.satisfies_restrictions(rule, &structure, rotation, anchor, origin_y) {
                    return None;
                }
                let priority = structure.priority;
                let reserve_space = structure.generation.reserve_space;
                let conflict_groups: Arc<[String]> = structure.conflict_groups.clone().into();
                let placement = StructurePlacement {
                    reference: Arc::clone(&rule.reference),
                    placement_anchor: anchor,
                    structure,
                    rotation,
                    origin: IVec3::new(anchor.x, origin_y, anchor.y),
                };
                (
                    vec![placement],
                    priority,
                    reserve_space,
                    conflict_groups,
                )
            }
            RootStructureSource::Set(set) => {
                debug_assert!(root.forced_rotation.is_none());
                let resolved = set.resolve(self.entropy, cell, anchor, |structure, rotation, piece_anchor| {
                    let origin_y = self.fit_origin_y(structure, rotation, piece_anchor)?;
                    self.satisfies_restrictions(
                        rule,
                        structure,
                        rotation,
                        piece_anchor,
                        origin_y,
                    )
                    .then_some(origin_y)
                })?;
                let pieces = resolved
                    .into_iter()
                    .map(|piece| StructurePlacement {
                        reference: Arc::clone(&rule.reference),
                        placement_anchor: anchor,
                        structure: piece.structure,
                        rotation: piece.rotation,
                        origin: IVec3::new(piece.anchor.x, piece.origin_y, piece.anchor.y),
                    })
                    .collect::<Vec<_>>();
                (
                    pieces,
                    set.priority(),
                    set.reserve_space(),
                    set.conflict_groups(),
                )
            }
        };

        let pieces = self.connectors.expand(
            self.entropy,
            pieces,
            |structure, rotation, geometric_origin| {
                let origin = if structure.requires_ground_fit_when_connected() {
                    let origin_y = self.fit_origin_y(structure, rotation, geometric_origin.xz())?;
                    IVec3::new(geometric_origin.x, origin_y, geometric_origin.z)
                } else {
                    geometric_origin
                };
                self.satisfies_restrictions(rule, structure, rotation, origin.xz(), origin.y)
                    .then_some(origin)
            },
        );

        let (horizontal_minimum, horizontal_maximum, minimum_y, maximum_y) =
            placement_bounds(&pieces)?;

        Some(StructureCandidate {
            rule_index,
            cell,
            placement_anchor: anchor,
            pieces: pieces.into(),
            horizontal_minimum,
            horizontal_maximum,
            minimum_y,
            maximum_y,
            priority,
            reserve_space,
            conflict_groups,
        })
    }

    fn lattice_hint(&self, rule: &RootStructureRule, cell: GenerationPoint2) -> Option<IVec2> {
        let spacing = i64::from(rule.spacing);
        let anchor_x = i64::from(cell.x()) * spacing
            + spacing / 2
            + jitter_offset(
                self.entropy.sample_2d(rule.jitter_x_domain, cell),
                rule.jitter,
            );
        let anchor_z = i64::from(cell.z()) * spacing
            + spacing / 2
            + jitter_offset(
                self.entropy.sample_2d(rule.jitter_z_domain, cell),
                rule.jitter,
            );
        Some(IVec2::new(
            i32::try_from(anchor_x).ok()?,
            i32::try_from(anchor_z).ok()?,
        ))
    }

    fn find_biome_margin_anchor(
        &self,
        rule: &RootStructureRule,
        cell: GenerationPoint2,
        hint: IVec2,
    ) -> Option<RootAnchor> {
        let (minimum, maximum) = cell_world_bounds(cell, rule.spacing)?;
        let side = usize::try_from(MARGIN_SAMPLE_DIVISIONS + 1).ok()?;
        let mut positions = Vec::with_capacity(side * side);
        let mut target = Vec::with_capacity(side * side);
        let queries = self.biomes.queries();

        for z_index in 0..=MARGIN_SAMPLE_DIVISIONS {
            let z = sample_cell_axis(minimum.y, maximum.y, z_index);
            for x_index in 0..=MARGIN_SAMPLE_DIVISIONS {
                let x = sample_cell_axis(minimum.x, maximum.x, x_index);
                let position = IVec2::new(x, z);
                positions.push(position);
                target.push(
                    queries.surface_biome_at(position.x, position.y).primary().as_str()
                        == rule.biome.as_ref(),
                );
            }
        }

        let mut best: Option<(i64, IVec2, IVec2)> = None;
        for z_index in 0..=MARGIN_SAMPLE_DIVISIONS {
            for x_index in 0..MARGIN_SAMPLE_DIVISIONS {
                let left = grid_index(side, x_index, z_index)?;
                let right = grid_index(side, x_index + 1, z_index)?;
                if target[left] == target[right] || positions[left] == positions[right] {
                    continue;
                }
                if let Some((inside, outside)) =
                    self.refine_biome_margin(rule.biome.as_ref(), positions[left], positions[right])
                {
                    select_margin_candidate(&mut best, hint, inside, outside);
                }
            }
        }
        for z_index in 0..MARGIN_SAMPLE_DIVISIONS {
            for x_index in 0..=MARGIN_SAMPLE_DIVISIONS {
                let top = grid_index(side, x_index, z_index)?;
                let bottom = grid_index(side, x_index, z_index + 1)?;
                if target[top] == target[bottom] || positions[top] == positions[bottom] {
                    continue;
                }
                if let Some((inside, outside)) =
                    self.refine_biome_margin(rule.biome.as_ref(), positions[top], positions[bottom])
                {
                    select_margin_candidate(&mut best, hint, inside, outside);
                }
            }
        }

        let (_, outside, outward) = best?;
        Some(RootAnchor {
            position: outside,
            forced_rotation: rotation_for_outward_normal(outward),
            margin_inside: Some(outside - outward),
        })
    }

    fn refine_biome_margin(
        &self,
        biome: &str,
        mut left: IVec2,
        mut right: IVec2,
    ) -> Option<(IVec2, IVec2)> {
        let mut left_is_target = self
            .biomes
            .queries()
            .surface_biome_at(left.x, left.y)
            .primary()
            .as_str()
            == biome;
        let right_is_target = self
            .biomes
            .queries()
            .surface_biome_at(right.x, right.y)
            .primary()
            .as_str()
            == biome;
        if left_is_target == right_is_target {
            return None;
        }

        while (right - left).abs().max_element() > 1 {
            let middle = IVec2::new(
                i32::try_from((i64::from(left.x) + i64::from(right.x)) / 2).ok()?,
                i32::try_from((i64::from(left.y) + i64::from(right.y)) / 2).ok()?,
            );
            if middle == left || middle == right {
                break;
            }
            let middle_is_target = self
                .biomes
                .queries()
                .surface_biome_at(middle.x, middle.y)
                .primary()
                .as_str()
                == biome;
            if middle_is_target == left_is_target {
                left = middle;
                left_is_target = middle_is_target;
            } else {
                right = middle;
            }
        }

        let (inside, outside) = if left_is_target {
            (left, right)
        } else {
            (right, left)
        };
        let outward = outside - inside;
        ((outward.x.unsigned_abs() + outward.y.unsigned_abs()) == 1).then_some((inside, outside))
    }

    fn fit_root_origin_y(
        &self,
        rule: &RootStructureRule,
        structure: &StructureDefinition,
        rotation: StructureRotation,
        root: RootAnchor,
    ) -> Option<i32> {
        if rule.placement == GeneratedSurfaceStructurePlacement::BiomeMargin
            && let Some(inside) = root.margin_inside
            && self
                .materials
                .queries()
                .generated_fluid_at(inside.x, self.sea_level, inside.y)
                .is_some()
        {
            return self
                .sea_level
                .checked_sub(structure.ground_anchor_y_offset());
        }
        self.fit_origin_y(structure, rotation, root.position)
    }

    fn fit_origin_y(
        &self,
        structure: &StructureDefinition,
        rotation: StructureRotation,
        anchor: IVec2,
    ) -> Option<i32> {
        let supports = if structure.ground_anchor_y.is_some() {
            structure.horizontal_footprint_for_rotation(rotation)
        } else {
            structure.support_offsets_for_rotation(rotation)
        };
        let mut minimum_ground_y = i32::MAX;
        let mut maximum_ground_y = i32::MIN;

        for offset in supports {
            let position = checked_horizontal_add(anchor, offset)?;
            let ground_y = self.terrain.queries().surface_at(position.x, position.y);
            if structure.restrictions.requires_dry_ground
                && self
                    .materials
                    .queries()
                    .generated_fluid_at(position.x, ground_y.saturating_add(1), position.y)
                    .is_some()
            {
                return None;
            }
            minimum_ground_y = minimum_ground_y.min(ground_y);
            maximum_ground_y = maximum_ground_y.max(ground_y);
        }

        if minimum_ground_y == i32::MAX {
            return None;
        }
        let slope = maximum_ground_y.saturating_sub(minimum_ground_y);
        if slope < structure.restrictions.min_slope || slope > structure.restrictions.max_slope {
            return None;
        }
        minimum_ground_y.checked_sub(structure.ground_anchor_y_offset())
    }

    fn satisfies_restrictions(
        &self,
        rule: &RootStructureRule,
        structure: &StructureDefinition,
        rotation: StructureRotation,
        anchor: IVec2,
        origin_y: i32,
    ) -> bool {
        let restrictions = &structure.restrictions;
        let base_y = origin_y.saturating_add(structure.min_y_offset());
        if restrictions.min_y.is_some_and(|minimum| base_y < minimum)
            || restrictions.max_y.is_some_and(|maximum| base_y > maximum)
        {
            return false;
        }

        if !restrictions.ground_blocks.is_empty()
            && structure
                .support_offsets_for_rotation(rotation)
                .into_iter()
                .any(|offset| {
                    checked_horizontal_add(anchor, offset).is_none_or(|position| {
                        let ground_y = self.terrain.queries().surface_at(position.x, position.y);
                        !self
                            .materials
                            .queries()
                            .solid_block_at(position.x, ground_y, position.y)
                            .is_some_and(|block| {
                                restrictions
                                    .ground_blocks
                                    .iter()
                                    .any(|allowed| allowed == block.as_str())
                            })
                    })
                })
        {
            return false;
        }

        if restrictions.required_biome_coverage > 0.0 {
            let footprint = structure.horizontal_footprint_for_rotation(rotation);
            let matching = footprint
                .iter()
                .filter(|offset| {
                    checked_horizontal_add(anchor, **offset).is_some_and(|position| {
                        self.biomes
                            .queries()
                            .surface_biome_at(position.x, position.y)
                            .primary()
                            .as_str()
                            == rule.biome.as_ref()
                    })
                })
                .count();
            let coverage = matching as f32 / footprint.len().max(1) as f32;
            if coverage + f32::EPSILON < restrictions.required_biome_coverage {
                return false;
            }
        }

        if structure.generation.fluid_policy == StructureFluidPolicy::Forbid
            && structure.voxels().iter().any(|voxel| {
                let offset = rotation.rotate_offset(voxel.offset);
                self.materials
                    .queries()
                    .generated_fluid_at(
                        anchor.x.saturating_add(offset.x),
                        origin_y.saturating_add(offset.y),
                        anchor.y.saturating_add(offset.z),
                    )
                    .is_some()
            })
        {
            return false;
        }

        restrictions
            .proximity
            .iter()
            .all(|restriction| self.proximity_rule_satisfied(anchor, restriction))
    }

    fn proximity_rule_satisfied(
        &self,
        anchor: IVec2,
        restriction: &crate::content::structure_rules::StructureProximityRestriction,
    ) -> bool {
        let minimum = restriction.min_distance.unwrap_or(0);
        match restriction.mode {
            StructureProximityMode::Required => self
                .nearest_target_distance_squared(anchor, restriction.max_distance, &restriction.target)
                .is_some_and(|distance_squared| {
                    distance_squared >= i64::from(minimum) * i64::from(minimum)
                }),
            StructureProximityMode::Forbidden => !self.target_exists_in_annulus(
                anchor,
                minimum,
                restriction.max_distance,
                &restriction.target,
            ),
        }
    }

    fn nearest_target_distance_squared(
        &self,
        anchor: IVec2,
        maximum_distance: u32,
        target: &StructureProximityTarget,
    ) -> Option<i64> {
        let radius = i32::try_from(maximum_distance)
            .expect("validated structure proximity maxDistance must fit i32");
        let maximum_squared = i64::from(maximum_distance) * i64::from(maximum_distance);
        let mut nearest = None;
        for z in -radius..=radius {
            for x in -radius..=radius {
                let distance_squared = i64::from(x) * i64::from(x) + i64::from(z) * i64::from(z);
                if distance_squared > maximum_squared
                    || nearest.is_some_and(|current| distance_squared >= current)
                {
                    continue;
                }
                let Some(position) = checked_horizontal_add(anchor, IVec2::new(x, z)) else {
                    continue;
                };
                if self.proximity_target_matches(position, target) {
                    nearest = Some(distance_squared);
                }
            }
        }
        nearest
    }

    fn target_exists_in_annulus(
        &self,
        anchor: IVec2,
        minimum_distance: u32,
        maximum_distance: u32,
        target: &StructureProximityTarget,
    ) -> bool {
        let radius = i32::try_from(maximum_distance)
            .expect("validated structure proximity maxDistance must fit i32");
        let minimum_squared = i64::from(minimum_distance) * i64::from(minimum_distance);
        let maximum_squared = i64::from(maximum_distance) * i64::from(maximum_distance);
        for z in -radius..=radius {
            for x in -radius..=radius {
                let distance_squared = i64::from(x) * i64::from(x) + i64::from(z) * i64::from(z);
                if distance_squared < minimum_squared || distance_squared > maximum_squared {
                    continue;
                }
                let Some(position) = checked_horizontal_add(anchor, IVec2::new(x, z)) else {
                    continue;
                };
                if self.proximity_target_matches(position, target) {
                    return true;
                }
            }
        }
        false
    }

    fn proximity_target_matches(&self, position: IVec2, target: &StructureProximityTarget) -> bool {
        let ground_y = self.terrain.queries().surface_at(position.x, position.y);
        if let Some(block) = target.block.as_deref() {
            return self
                .materials
                .queries()
                .solid_block_at(position.x, ground_y, position.y)
                .is_some_and(|candidate| candidate.as_str() == block);
        }
        if let Some(fluid) = target.fluid.as_deref() {
            return self
                .materials
                .queries()
                .generated_fluid_at(position.x, ground_y.saturating_add(1), position.y)
                .is_some_and(|candidate| candidate.as_str() == fluid);
        }
        unreachable!("validated proximity target must define block or fluid")
    }
}

impl RootStructureRule {
    fn new(
        definition: &GeneratedSurfaceStructureDefinition,
        structures: &StructureRegistry,
        structure_sets: &StructureSetRegistry,
        connectors: &ConnectorGraph,
    ) -> Self {
        let suffix = format!("{}/{}", definition.biome, definition.structure);
        if definition.placement == GeneratedSurfaceStructurePlacement::BiomeMargin {
            assert!(
                structure_sets.get(&definition.structure).is_none(),
                "biomeMargin generated surface structure cannot reference a StructureSet: {}",
                definition.structure
            );
        }
        let (source, maximum_horizontal_extent) =
            if let Some(set) = structure_sets.get(&definition.structure) {
                let connected_bounds = set
                    .horizontal_bounds_with(|reference| {
                        connectors.connected_horizontal_bounds_for_reference(reference)
                    })
                    .expect("validated StructureSet references must have connector-expanded bounds");
                let compiled = CompiledStructureSet::new(set, structures, &suffix);
                (
                    RootStructureSource::Set(compiled),
                    maximum_extent_from_bounds(connected_bounds),
                )
            } else {
                let variants = structures
                    .reference_members(&definition.structure)
                    .unwrap_or_else(|| {
                        panic!(
                            "generated surface structure rule references missing Structure, Structure group, or StructureSet: {}",
                            definition.structure
                        )
                    })
                    .into_iter()
                    .map(|structure| Arc::new(structure.clone()))
                    .collect::<Vec<_>>();
                assert!(
                    !variants.is_empty(),
                    "generated surface structure reference {} must resolve at least one Structure",
                    definition.structure
                );
                if definition.placement == GeneratedSurfaceStructurePlacement::BiomeMargin {
                    assert!(
                        variants
                            .iter()
                            .all(|structure| structure.supported_rotations().len() == 4),
                        "biomeMargin generated surface structure reference {} must support all horizontal rotations",
                        definition.structure
                    );
                }
                let connected_bounds = connectors
                    .connected_horizontal_bounds_for_reference(&definition.structure)
                    .expect("validated Structure reference must have connector-expanded bounds");
                (
                    RootStructureSource::Direct {
                        variants: variants.into(),
                        variant_domain: GenerationDomain::named(&format!(
                            "{VARIANT_DOMAIN_PREFIX}{suffix}"
                        )),
                        rotation_domain: GenerationDomain::named(&format!(
                            "{ROTATION_DOMAIN_PREFIX}{suffix}"
                        )),
                    },
                    maximum_extent_from_bounds(connected_bounds),
                )
            };

        Self {
            biome: Arc::from(definition.biome.as_str()),
            reference: Arc::from(definition.structure.as_str()),
            spacing: i32::try_from(definition.spacing)
                .expect("validated structure spacing must fit i32"),
            jitter: definition.jitter,
            chance: definition.chance,
            placement: definition.placement,
            source,
            maximum_horizontal_extent,
            presence_domain: GenerationDomain::named(&format!("{PRESENCE_DOMAIN_PREFIX}{suffix}")),
            jitter_x_domain: GenerationDomain::named(&format!("{JITTER_X_DOMAIN_PREFIX}{suffix}")),
            jitter_z_domain: GenerationDomain::named(&format!("{JITTER_Z_DOMAIN_PREFIX}{suffix}")),
        }
    }
}

fn cell_world_bounds(cell: GenerationPoint2, spacing: i32) -> Option<(IVec2, IVec2)> {
    let spacing = i64::from(spacing);
    let minimum_x = i64::from(cell.x()) * spacing;
    let minimum_z = i64::from(cell.z()) * spacing;
    let maximum_x = minimum_x + spacing - 1;
    let maximum_z = minimum_z + spacing - 1;
    Some((
        IVec2::new(i32::try_from(minimum_x).ok()?, i32::try_from(minimum_z).ok()?),
        IVec2::new(i32::try_from(maximum_x).ok()?, i32::try_from(maximum_z).ok()?),
    ))
}

fn sample_cell_axis(minimum: i32, maximum: i32, index: i32) -> i32 {
    let span = i64::from(maximum) - i64::from(minimum);
    let offset = span * i64::from(index) / i64::from(MARGIN_SAMPLE_DIVISIONS);
    i32::try_from(i64::from(minimum) + offset).expect("cell sample remains within i32 bounds")
}

fn grid_index(side: usize, x: i32, z: i32) -> Option<usize> {
    let x = usize::try_from(x).ok()?;
    let z = usize::try_from(z).ok()?;
    z.checked_mul(side)?.checked_add(x)
}

fn select_margin_candidate(
    best: &mut Option<(i64, IVec2, IVec2)>,
    hint: IVec2,
    inside: IVec2,
    outside: IVec2,
) {
    let outward = outside - inside;
    let delta = outside - hint;
    let distance_squared = i64::from(delta.x) * i64::from(delta.x)
        + i64::from(delta.y) * i64::from(delta.y);
    let candidate_key = (
        distance_squared,
        outside.y,
        outside.x,
        outward.y,
        outward.x,
    );
    let replace = best.as_ref().is_none_or(|(current_distance, current_outside, current_outward)| {
        candidate_key
            < (
                *current_distance,
                current_outside.y,
                current_outside.x,
                current_outward.y,
                current_outward.x,
            )
    });
    if replace {
        *best = Some((distance_squared, outside, outward));
    }
}

fn rotation_for_outward_normal(outward: IVec2) -> Option<StructureRotation> {
    match (outward.x, outward.y) {
        (0, 1) => Some(StructureRotation::Degrees0),
        (-1, 0) => Some(StructureRotation::Degrees90),
        (0, -1) => Some(StructureRotation::Degrees180),
        (1, 0) => Some(StructureRotation::Degrees270),
        _ => None,
    }
}

fn maximum_extent_from_bounds((minimum, maximum): (IVec2, IVec2)) -> i32 {
    minimum
        .x
        .unsigned_abs()
        .max(minimum.y.unsigned_abs())
        .max(maximum.x.unsigned_abs())
        .max(maximum.y.unsigned_abs())
        .try_into()
        .unwrap_or(i32::MAX)
}

fn placement_bounds(pieces: &[StructurePlacement]) -> Option<(IVec2, IVec2, i32, i32)> {
    let mut horizontal_minimum = IVec2::splat(i32::MAX);
    let mut horizontal_maximum = IVec2::splat(i32::MIN);
    let mut minimum_y = i32::MAX;
    let mut maximum_y = i32::MIN;

    for piece in pieces {
        let (piece_minimum, piece_maximum) = piece.horizontal_bounds();
        let (piece_minimum_y, piece_maximum_y) = piece.vertical_bounds();
        horizontal_minimum = horizontal_minimum.min(piece_minimum);
        horizontal_maximum = horizontal_maximum.max(piece_maximum);
        minimum_y = minimum_y.min(piece_minimum_y);
        maximum_y = maximum_y.max(piece_maximum_y);
    }

    (minimum_y != i32::MAX).then_some((
        horizontal_minimum,
        horizontal_maximum,
        minimum_y,
        maximum_y,
    ))
}

fn candidate_identity(candidate: &StructureCandidate) -> (usize, i32, i32) {
    (
        candidate.rule_index,
        candidate.cell.x(),
        candidate.cell.z(),
    )
}

fn same_candidate(left: &StructureCandidate, right: &StructureCandidate) -> bool {
    candidate_identity(left) == candidate_identity(right)
}

fn candidates_conflict(higher: &StructureCandidate, lower: &StructureCandidate) -> bool {
    let (higher_minimum, higher_maximum) = higher.horizontal_bounds();
    let (lower_minimum, lower_maximum) = lower.horizontal_bounds();
    let (higher_minimum_y, higher_maximum_y) = higher.vertical_bounds();
    let (lower_minimum_y, lower_maximum_y) = lower.vertical_bounds();

    rectangles_overlap(
        higher_minimum,
        higher_maximum,
        lower_minimum,
        lower_maximum,
    ) && higher_maximum_y >= lower_minimum_y
        && higher_minimum_y <= lower_maximum_y
        && (higher.reserve_space
            || higher.conflict_groups.iter().any(|group| {
                lower
                    .conflict_groups
                    .iter()
                    .any(|candidate| candidate == group)
            }))
}

fn candidate_cell_bounds(spacing: i32, minimum: IVec2, maximum: IVec2) -> (IVec2, IVec2) {
    (
        IVec2::new(
            minimum.x.div_euclid(spacing).saturating_sub(1),
            minimum.y.div_euclid(spacing).saturating_sub(1),
        ),
        IVec2::new(
            maximum.x.div_euclid(spacing).saturating_add(1),
            maximum.y.div_euclid(spacing).saturating_add(1),
        ),
    )
}

fn checked_area_max(origin: i32, extent: u32, axis: &str) -> i32 {
    i32::try_from(i64::from(origin) + i64::from(extent) - 1)
        .unwrap_or_else(|_| panic!("structure query {axis} extent exceeds world coordinate range"))
}

fn checked_horizontal_add(origin: IVec2, offset: IVec2) -> Option<IVec2> {
    Some(IVec2::new(
        origin.x.checked_add(offset.x)?,
        origin.y.checked_add(offset.y)?,
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

fn unit_probability(value: u64) -> f64 {
    value as f64 / u64::MAX as f64
}

fn jitter_offset(value: u64, jitter: u32) -> i64 {
    if jitter == 0 {
        return 0;
    }
    let span = u64::from(jitter) * 2 + 1;
    i64::try_from(value % span).expect("structure jitter remainder must fit i64") - i64::from(jitter)
}

fn placement_sort_key(
    placement: &StructurePlacement,
) -> (i32, i32, i32, i32, i32, &str) {
    (
        placement.placement_anchor.y,
        placement.placement_anchor.x,
        placement.origin.z,
        placement.origin.x,
        placement.origin.y,
        placement.structure.id.as_str(),
    )
}
