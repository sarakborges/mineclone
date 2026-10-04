use std::{collections::HashMap, sync::Arc};

use smallvec::SmallVec;

use crate::content::biome::{BiomeDefinition, BiomeRegistry};

use super::foundation::{
    GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationSnapshot,
};

const MIN_CONSTRUCTION_SPAN: u32 = 64;
const MAX_CONSTRUCTION_SPAN: u32 = 512;
const BLEND_FRACTION: f64 = 0.24;
const FORMATION_TARGET_DOMAIN_PREFIX: &str = "biome-layout/formation-target/v1/";
const CELL_PICK_DOMAIN: &str = "biome-layout/cell-pick/v2";
const WARP_COARSE_X_DOMAIN: &str = "biome-layout/warp-coarse-x/v2";
const WARP_COARSE_Z_DOMAIN: &str = "biome-layout/warp-coarse-z/v2";
const WARP_FINE_X_DOMAIN: &str = "biome-layout/warp-fine-x/v2";
const WARP_FINE_Z_DOMAIN: &str = "biome-layout/warp-fine-z/v2";

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct BiomeId(Arc<str>);

impl BiomeId {
    fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BiomeInfluence {
    biome: BiomeId,
    weight: f32,
}

impl BiomeInfluence {
    pub(crate) fn biome(&self) -> &BiomeId {
        &self.biome
    }

    pub(crate) const fn weight(&self) -> f32 {
        self.weight
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BiomeSample {
    primary: BiomeId,
    influences: SmallVec<[BiomeInfluence; 4]>,
}

impl BiomeSample {
    pub(crate) fn primary(&self) -> &BiomeId {
        &self.primary
    }

    pub(crate) fn influences(&self) -> &[BiomeInfluence] {
        &self.influences
    }

    pub(crate) fn primary_weight(&self) -> f32 {
        self.influences
            .iter()
            .find(|influence| influence.biome == self.primary)
            .map_or(1.0, BiomeInfluence::weight)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BiomeAreaSample {
    origin_x: i32,
    origin_z: i32,
    width: u32,
    depth: u32,
    step: u32,
    samples: Vec<BiomeSample>,
}

impl BiomeAreaSample {
    pub(crate) const fn origin(&self) -> (i32, i32) {
        (self.origin_x, self.origin_z)
    }

    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    pub(crate) const fn depth(&self) -> u32 {
        self.depth
    }

    pub(crate) const fn step(&self) -> u32 {
        self.step
    }

    pub(crate) fn sample_at(&self, x_index: u32, z_index: u32) -> Option<&BiomeSample> {
        if x_index >= self.width || z_index >= self.depth {
            return None;
        }
        self.samples
            .get(z_index as usize * self.width as usize + x_index as usize)
    }

    pub(crate) fn get(&self, x: i32, z: i32) -> Option<&BiomeSample> {
        let dx = i64::from(x) - i64::from(self.origin_x);
        let dz = i64::from(z) - i64::from(self.origin_z);
        if dx < 0 || dz < 0 {
            return None;
        }
        let step = i64::from(self.step);
        if dx % step != 0 || dz % step != 0 {
            return None;
        }
        let x_index = u32::try_from(dx / step).ok()?;
        let z_index = u32::try_from(dz / step).ok()?;
        self.sample_at(x_index, z_index)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BiomeSearchResult {
    x: i32,
    z: i32,
    sample: BiomeSample,
}

impl BiomeSearchResult {
    pub(crate) const fn position(&self) -> (i32, i32) {
        (self.x, self.z)
    }

    pub(crate) fn sample(&self) -> &BiomeSample {
        &self.sample
    }
}

#[derive(Clone, Copy)]
pub(crate) struct BiomeQueries<'a> {
    layout: &'a BiomeLayout,
}

impl BiomeQueries<'_> {
    pub(crate) fn surface_biome_at(&self, x: i32, z: i32) -> BiomeSample {
        self.layout.sample_surface(x, z)
    }

    pub(crate) fn volume_biome_at(&self, _x: i32, _y: i32, _z: i32) -> Option<BiomeSample> {
        None
    }

    pub(crate) fn effective_biome_at(&self, x: i32, y: i32, z: i32) -> BiomeSample {
        self.volume_biome_at(x, y, z)
            .unwrap_or_else(|| self.surface_biome_at(x, z))
    }

    pub(crate) fn sample_surface_area(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
    ) -> BiomeAreaSample {
        self.layout
            .sample_surface_grid(origin_x, origin_z, width, depth, 1)
    }

    pub(crate) fn sample_surface_grid(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
        step: u32,
    ) -> BiomeAreaSample {
        self.layout
            .sample_surface_grid(origin_x, origin_z, width, depth, step)
    }

    pub(crate) fn find_surface_biome(
        &self,
        biome_id: &str,
        origin_x: i32,
        origin_z: i32,
        max_distance: u32,
    ) -> Option<BiomeSearchResult> {
        self.layout
            .find_surface_biome(biome_id, origin_x, origin_z, max_distance)
    }

    pub(crate) fn biome_ids(&self) -> impl Iterator<Item = &BiomeId> {
        self.layout.rules.iter().map(|rule| &rule.id)
    }

    pub(crate) fn biome_region_ranges(&self) -> impl Iterator<Item = (&BiomeId, u32, u32)> {
        self.layout
            .rules
            .iter()
            .map(|rule| (&rule.id, rule.region_min, rule.region_max))
    }

    pub(crate) fn suppressed_biomes(&self) -> &[BiomeId] {
        &self.layout.suppressed_biomes
    }
}

#[derive(Clone, Debug)]
pub(super) struct BiomeLayout {
    entropy: GenerationEntropy,
    rules: Arc<[BiomeRule]>,
    suppressed_biomes: Arc<[BiomeId]>,
    construction_span: u32,
    fallback_rule: usize,
}

#[derive(Clone, Debug)]
struct BiomeRule {
    id: BiomeId,
    weight_units: u64,
    region_min: u32,
    region_max: u32,
    cannot_border: Arc<[BiomeId]>,
    target_domain: GenerationDomain,
}

impl BiomeRule {
    fn from_definition(definition: &BiomeDefinition) -> Self {
        Self {
            id: BiomeId::new(Arc::<str>::from(definition.id.as_str())),
            weight_units: quantize_weight(definition.weight),
            region_min: definition.region_size.min,
            region_max: definition.region_size.max,
            cannot_border: definition
                .cannot_border
                .iter()
                .map(|id| BiomeId::new(Arc::<str>::from(id.as_str())))
                .collect::<Vec<_>>()
                .into(),
            target_domain: GenerationDomain::named(&format!(
                "{FORMATION_TARGET_DOMAIN_PREFIX}{}",
                definition.id
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LayoutCell {
    x: i32,
    z: i32,
}

impl LayoutCell {
    const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    fn class(self) -> u8 {
        ((self.x.rem_euclid(2) as u8) << 1) | self.z.rem_euclid(2) as u8
    }

    fn as_point(self) -> GenerationPoint2 {
        GenerationPoint2::new(self.x, self.z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CellAssignment {
    rule: usize,
    root: LayoutCell,
    target_span: u32,
}

impl BiomeLayout {
    pub(super) fn new(snapshot: &GenerationSnapshot, registry: &BiomeRegistry) -> Self {
        let dimension_id = snapshot.dimension().id();
        let mut definitions = registry.for_dimension(dimension_id).collect::<Vec<_>>();
        definitions.sort_by(|left, right| left.id.cmp(&right.id));
        assert!(
            !definitions.is_empty(),
            "dimension {dimension_id} has no authored biomes"
        );
        for definition in &definitions {
            definition.validate_references(registry);
        }

        let all_rules = definitions
            .into_iter()
            .map(BiomeRule::from_definition)
            .collect::<Vec<_>>();
        let fallback_original = choose_fallback_rule(&all_rules);
        let fallback = all_rules[fallback_original].clone();
        let fallback_id = fallback.id.clone();
        let mut suppressed_biomes = Vec::new();
        let mut active_rules = Vec::new();
        for rule in all_rules {
            if compatible_rules(&rule, &fallback) {
                active_rules.push(rule);
            } else {
                suppressed_biomes.push(rule.id);
            }
        }
        active_rules.sort_by(|left, right| left.id.cmp(&right.id));
        suppressed_biomes.sort();
        let fallback_rule = active_rules
            .iter()
            .position(|rule| rule.id == fallback_id)
            .expect("fallback biome must remain active");

        let min_span = active_rules
            .iter()
            .map(|rule| rule.region_min)
            .min()
            .expect("active biome layout cannot be empty");
        let construction_span = min_span
            .div_ceil(2)
            .clamp(MIN_CONSTRUCTION_SPAN, MAX_CONSTRUCTION_SPAN);

        Self {
            entropy: GenerationEntropy::new(snapshot),
            rules: active_rules.into(),
            suppressed_biomes: suppressed_biomes.into(),
            construction_span,
            fallback_rule,
        }
    }

    pub(super) fn queries(&self) -> BiomeQueries<'_> {
        BiomeQueries { layout: self }
    }

    fn sample_surface(&self, x: i32, z: i32) -> BiomeSample {
        let mut assignments = HashMap::new();
        self.sample_surface_cached(x, z, &mut assignments)
    }

    fn sample_surface_grid(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
        step: u32,
    ) -> BiomeAreaSample {
        assert!(width > 0 && depth > 0, "biome sample area must be non-empty");
        assert!(step > 0, "biome sample step must be positive");
        validate_grid_extent(origin_x, width, step, "X");
        validate_grid_extent(origin_z, depth, step, "Z");
        let sample_count = u64::from(width)
            .checked_mul(u64::from(depth))
            .and_then(|count| usize::try_from(count).ok())
            .expect("biome sample area is too large");
        let mut samples = Vec::with_capacity(sample_count);
        let mut assignments = HashMap::new();
        for z_index in 0..depth {
            let z = grid_axis(origin_z, z_index, step);
            for x_index in 0..width {
                let x = grid_axis(origin_x, x_index, step);
                samples.push(self.sample_surface_cached(x, z, &mut assignments));
            }
        }
        BiomeAreaSample {
            origin_x,
            origin_z,
            width,
            depth,
            step,
            samples,
        }
    }

    fn sample_surface_cached(
        &self,
        x: i32,
        z: i32,
        assignments: &mut HashMap<LayoutCell, CellAssignment>,
    ) -> BiomeSample {
        let (cell, local_x, local_z) = self.warped_cell(x, z);
        let primary_assignment = self.cell_assignment(cell, assignments);
        let mut raw_weights = vec![0.0_f64; self.rules.len()];
        raw_weights[primary_assignment.rule] = 1.0;

        for dz in -1..=1 {
            let z_proximity = neighbor_proximity(local_z, dz);
            if z_proximity <= 0.0 {
                continue;
            }
            for dx in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let x_proximity = neighbor_proximity(local_x, dx);
                if x_proximity <= 0.0 {
                    continue;
                }
                let Some(neighbor_x) = cell.x.checked_add(dx) else {
                    continue;
                };
                let Some(neighbor_z) = cell.z.checked_add(dz) else {
                    continue;
                };
                let neighbor = self.cell_assignment(
                    LayoutCell::new(neighbor_x, neighbor_z),
                    assignments,
                );
                raw_weights[neighbor.rule] += x_proximity * z_proximity;
            }
        }

        let total = raw_weights.iter().sum::<f64>();
        let mut influences = SmallVec::<[BiomeInfluence; 4]>::new();
        influences.push(BiomeInfluence {
            biome: self.rules[primary_assignment.rule].id.clone(),
            weight: (raw_weights[primary_assignment.rule] / total) as f32,
        });
        let mut secondary = raw_weights
            .into_iter()
            .enumerate()
            .filter(|(rule, weight)| *rule != primary_assignment.rule && *weight > 0.0)
            .collect::<Vec<_>>();
        secondary.sort_by(|(left_rule, left_weight), (right_rule, right_weight)| {
            right_weight
                .total_cmp(left_weight)
                .then_with(|| self.rules[*left_rule].id.cmp(&self.rules[*right_rule].id))
        });
        influences.extend(secondary.into_iter().map(|(rule, weight)| BiomeInfluence {
            biome: self.rules[rule].id.clone(),
            weight: (weight / total) as f32,
        }));

        BiomeSample {
            primary: self.rules[primary_assignment.rule].id.clone(),
            influences,
        }
    }

    fn find_surface_biome(
        &self,
        biome_id: &str,
        origin_x: i32,
        origin_z: i32,
        max_distance: u32,
    ) -> Option<BiomeSearchResult> {
        let target_rule = self.rules.iter().position(|rule| rule.id.as_str() == biome_id)?;
        let origin_sample = self.sample_surface(origin_x, origin_z);
        if origin_sample.primary().as_str() == biome_id {
            return Some(BiomeSearchResult {
                x: origin_x,
                z: origin_z,
                sample: origin_sample,
            });
        }

        let (origin_cell, _, _) = self.warped_cell(origin_x, origin_z);
        let cell_radius = max_distance.div_ceil(self.construction_span) as i32 + 4;
        let max_distance_sq = i128::from(max_distance) * i128::from(max_distance);
        let mut assignments = HashMap::new();
        let mut best: Option<(i128, BiomeSearchResult)> = None;

        for ring in 0..=cell_radius {
            for cell in ring_cells(origin_cell, ring) {
                if self.cell_assignment(cell, &mut assignments).rule != target_rule {
                    continue;
                }
                for (x, z) in self.search_probe_points(cell) {
                    let dx = i128::from(x) - i128::from(origin_x);
                    let dz = i128::from(z) - i128::from(origin_z);
                    let distance_sq = dx * dx + dz * dz;
                    if distance_sq > max_distance_sq
                        || best.as_ref().is_some_and(|(best_sq, _)| distance_sq >= *best_sq)
                    {
                        continue;
                    }
                    let sample = self.sample_surface_cached(x, z, &mut assignments);
                    if sample.primary().as_str() == biome_id {
                        best = Some((distance_sq, BiomeSearchResult { x, z, sample }));
                    }
                }
            }
            if let Some((best_sq, _)) = &best {
                let conservative_ring = i128::from(ring.saturating_sub(4))
                    * i128::from(self.construction_span);
                if conservative_ring * conservative_ring > *best_sq {
                    break;
                }
            }
        }
        best.map(|(_, result)| result)
    }

    fn search_probe_points(&self, cell: LayoutCell) -> [(i32, i32); 9] {
        let span = i64::from(self.construction_span);
        let base_x = i64::from(cell.x) * span;
        let base_z = i64::from(cell.z) * span;
        let quarter = span / 4;
        let center = span / 2;
        let three_quarters = span - quarter;
        let offsets = [quarter, center, three_quarters];
        let mut points = [(0_i32, 0_i32); 9];
        let mut index = 0;
        for z_offset in offsets {
            for x_offset in offsets {
                points[index] = (
                    clamp_world_axis(base_x + x_offset),
                    clamp_world_axis(base_z + z_offset),
                );
                index += 1;
            }
        }
        points
    }

    fn cell_assignment(
        &self,
        cell: LayoutCell,
        cache: &mut HashMap<LayoutCell, CellAssignment>,
    ) -> CellAssignment {
        if let Some(assignment) = cache.get(&cell) {
            return *assignment;
        }

        let class = cell.class();
        let mut established = SmallVec::<[CellAssignment; 8]>::new();
        if class > 0 {
            for dz in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dz == 0 {
                        continue;
                    }
                    let Some(x) = cell.x.checked_add(dx) else {
                        continue;
                    };
                    let Some(z) = cell.z.checked_add(dz) else {
                        continue;
                    };
                    let neighbor = LayoutCell::new(x, z);
                    if neighbor.class() < class {
                        established.push(self.cell_assignment(neighbor, cache));
                    }
                }
            }
        }

        let eligible = (0..self.rules.len())
            .filter(|candidate| {
                established
                    .iter()
                    .all(|neighbor| self.compatible_indices(*candidate, neighbor.rule))
            })
            .collect::<SmallVec<[usize; 16]>>();
        let eligible = if eligible.is_empty() {
            debug_assert!(
                established
                    .iter()
                    .all(|neighbor| self.compatible_indices(self.fallback_rule, neighbor.rule)),
                "fallback biome must be compatible with every active biome"
            );
            SmallVec::from_slice(&[self.fallback_rule])
        } else {
            eligible
        };

        let mut weighted = SmallVec::<[(usize, u64); 16]>::new();
        let mut total = 0_u64;
        for candidate in eligible {
            let continuation = established
                .iter()
                .filter(|neighbor| neighbor.rule == candidate)
                .map(|neighbor| self.growth_affinity(*neighbor, cell))
                .max()
                .unwrap_or(0);
            let growth_multiplier = 1_u64.saturating_add(continuation.saturating_mul(4));
            let weight = self.rules[candidate]
                .weight_units
                .saturating_mul(growth_multiplier);
            total = total.saturating_add(weight);
            weighted.push((candidate, weight));
        }

        let pick = self.entropy.sample_2d(
            GenerationDomain::named(CELL_PICK_DOMAIN),
            cell.as_point(),
        ) % total.max(1);
        let mut cursor = 0_u64;
        let selected_rule = weighted
            .iter()
            .find_map(|(candidate, weight)| {
                cursor = cursor.saturating_add(*weight);
                (pick < cursor).then_some(*candidate)
            })
            .unwrap_or_else(|| weighted.last().expect("eligible biome pool cannot be empty").0);

        let continued = established
            .iter()
            .filter(|neighbor| neighbor.rule == selected_rule)
            .map(|neighbor| (*neighbor, self.growth_affinity(*neighbor, cell)))
            .filter(|(_, affinity)| *affinity > 0)
            .max_by(|(left, left_affinity), (right, right_affinity)| {
                left_affinity
                    .cmp(right_affinity)
                    .then_with(|| right.root.cmp(&left.root))
            })
            .map(|(assignment, _)| assignment);
        let assignment = continued.unwrap_or_else(|| CellAssignment {
            rule: selected_rule,
            root: cell,
            target_span: self.formation_target_span(selected_rule, cell),
        });
        cache.insert(cell, assignment);
        assignment
    }

    fn formation_target_span(&self, rule: usize, root: LayoutCell) -> u32 {
        let definition = &self.rules[rule];
        random_span_u32(
            self.entropy.sample_2d(definition.target_domain, root.as_point()),
            definition.region_min,
            definition.region_max,
        )
    }

    fn growth_affinity(&self, assignment: CellAssignment, target: LayoutCell) -> u64 {
        let target_cells = assignment
            .target_span
            .div_ceil(self.construction_span)
            .max(1);
        let dx = i64::from(target.x) - i64::from(assignment.root.x);
        let dz = i64::from(target.z) - i64::from(assignment.root.z);
        let distance = dx.unsigned_abs().max(dz.unsigned_abs());
        if distance > u64::from(target_cells) {
            return 0;
        }
        let remaining = u64::from(target_cells) - distance + 1;
        remaining.saturating_mul(remaining)
    }

    fn compatible_indices(&self, left: usize, right: usize) -> bool {
        compatible_rules(&self.rules[left], &self.rules[right])
    }

    fn warped_cell(&self, x: i32, z: i32) -> (LayoutCell, f64, f64) {
        let (warped_x, warped_z) = self.warped_position(x, z);
        let span = f64::from(self.construction_span);
        let scaled_x = warped_x / span;
        let scaled_z = warped_z / span;
        let cell_x = scaled_x.floor();
        let cell_z = scaled_z.floor();
        (
            LayoutCell::new(
                clamp_cell_axis(cell_x),
                clamp_cell_axis(cell_z),
            ),
            scaled_x - cell_x,
            scaled_z - cell_z,
        )
    }

    fn warped_position(&self, x: i32, z: i32) -> (f64, f64) {
        let span = f64::from(self.construction_span);
        let x_f = f64::from(x);
        let z_f = f64::from(z);
        let coarse_period = span * 3.2;
        let fine_period = span * 0.95;
        let coarse_x = self.smooth_noise(
            GenerationDomain::named(WARP_COARSE_X_DOMAIN),
            x_f,
            z_f,
            coarse_period,
        );
        let coarse_z = self.smooth_noise(
            GenerationDomain::named(WARP_COARSE_Z_DOMAIN),
            x_f + coarse_period * 0.37,
            z_f - coarse_period * 0.61,
            coarse_period,
        );
        let fine_x = self.smooth_noise(
            GenerationDomain::named(WARP_FINE_X_DOMAIN),
            x_f - fine_period * 0.43,
            z_f + fine_period * 0.29,
            fine_period,
        );
        let fine_z = self.smooth_noise(
            GenerationDomain::named(WARP_FINE_Z_DOMAIN),
            x_f + fine_period * 0.71,
            z_f + fine_period * 0.53,
            fine_period,
        );
        (
            x_f + coarse_x * span * 0.30 + fine_x * span * 0.11,
            z_f + coarse_z * span * 0.30 + fine_z * span * 0.11,
        )
    }

    fn smooth_noise(
        &self,
        domain: GenerationDomain,
        x: f64,
        z: f64,
        period: f64,
    ) -> f64 {
        let grid_x = (x / period).floor().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        let grid_z = (z / period).floor().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        let tx = fade((x / period) - f64::from(grid_x));
        let tz = fade((z / period) - f64::from(grid_z));
        let x1 = grid_x.saturating_add(1);
        let z1 = grid_z.saturating_add(1);
        let a = signed_unit(
            self.entropy
                .sample_2d(domain, GenerationPoint2::new(grid_x, grid_z)),
        );
        let b = signed_unit(
            self.entropy
                .sample_2d(domain, GenerationPoint2::new(x1, grid_z)),
        );
        let c = signed_unit(
            self.entropy
                .sample_2d(domain, GenerationPoint2::new(grid_x, z1)),
        );
        let d = signed_unit(
            self.entropy
                .sample_2d(domain, GenerationPoint2::new(x1, z1)),
        );
        lerp(lerp(a, b, tx), lerp(c, d, tx), tz)
    }
}

fn choose_fallback_rule(rules: &[BiomeRule]) -> usize {
    rules
        .iter()
        .enumerate()
        .max_by(|(left_index, left), (right_index, right)| {
            let left_weight = compatible_weight(rules, *left_index);
            let right_weight = compatible_weight(rules, *right_index);
            left_weight
                .cmp(&right_weight)
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|(index, _)| index)
        .expect("biome layout needs at least one rule")
}

fn compatible_weight(rules: &[BiomeRule], candidate: usize) -> u128 {
    rules
        .iter()
        .filter(|other| compatible_rules(&rules[candidate], other))
        .map(|rule| u128::from(rule.weight_units))
        .sum()
}

fn compatible_rules(left: &BiomeRule, right: &BiomeRule) -> bool {
    left.id == right.id
        || (!left.cannot_border.iter().any(|id| id == &right.id)
            && !right.cannot_border.iter().any(|id| id == &left.id))
}

fn neighbor_proximity(local: f64, offset: i32) -> f64 {
    match offset {
        -1 => ((BLEND_FRACTION - local) / BLEND_FRACTION).clamp(0.0, 1.0),
        0 => 1.0,
        1 => ((local - (1.0 - BLEND_FRACTION)) / BLEND_FRACTION).clamp(0.0, 1.0),
        _ => 0.0,
    }
}

fn quantize_weight(weight: f32) -> u64 {
    (f64::from(weight) * 4096.0).round().max(1.0) as u64
}

fn random_span_u32(hash: u64, min: u32, max: u32) -> u32 {
    if min == max {
        return min;
    }
    let range = u64::from(max - min) + 1;
    min + (hash % range) as u32
}

fn ring_cells(center: LayoutCell, ring: i32) -> Vec<LayoutCell> {
    if ring == 0 {
        return vec![center];
    }
    let mut cells = Vec::with_capacity((ring * 8) as usize);
    for dx in -ring..=ring {
        if let (Some(x), Some(z)) = (center.x.checked_add(dx), center.z.checked_sub(ring)) {
            cells.push(LayoutCell::new(x, z));
        }
    }
    for dz in (-ring + 1)..=ring {
        if let (Some(x), Some(z)) = (center.x.checked_add(ring), center.z.checked_add(dz)) {
            cells.push(LayoutCell::new(x, z));
        }
    }
    for dx in (-ring..ring).rev() {
        if let (Some(x), Some(z)) = (center.x.checked_add(dx), center.z.checked_add(ring)) {
            cells.push(LayoutCell::new(x, z));
        }
    }
    for dz in ((-ring + 1)..ring).rev() {
        if let (Some(x), Some(z)) = (center.x.checked_sub(ring), center.z.checked_add(dz)) {
            cells.push(LayoutCell::new(x, z));
        }
    }
    cells
}

fn validate_grid_extent(origin: i32, count: u32, step: u32, axis: &str) {
    let last_offset = u64::from(count - 1)
        .checked_mul(u64::from(step))
        .expect("biome sample grid span is too large");
    let last = i128::from(origin) + i128::from(last_offset);
    assert!(
        last <= i128::from(i32::MAX),
        "biome sample {axis} extent exceeds world coordinate range"
    );
}

fn grid_axis(origin: i32, index: u32, step: u32) -> i32 {
    let offset = u64::from(index) * u64::from(step);
    i32::try_from(i128::from(origin) + i128::from(offset))
        .expect("validated biome sample coordinate must fit i32")
}

fn clamp_cell_axis(value: f64) -> i32 {
    value.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

fn clamp_world_axis(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn unit(hash: u64) -> f64 {
    (hash >> 11) as f64 / ((1_u64 << 53) - 1) as f64
}

fn signed_unit(hash: u64) -> f64 {
    unit(hash) * 2.0 - 1.0
}

fn fade(value: f64) -> f64 {
    value * value * value * (value * (value * 6.0 - 15.0) + 10.0)
}

fn lerp(left: f64, right: f64, t: f64) -> f64 {
    left + (right - left) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::foundation::{GenerationDimension, GenerationSeed};
    use crate::content::biome::BiomeRegistry;

    fn definition(
        id: &str,
        weight: f32,
        min: u32,
        max: u32,
        cannot_border: &[&str],
    ) -> BiomeDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": {
                "english": id,
                "portuguese_brazil": id,
                "spanish": id
            },
            "weight": weight,
            "regionSize": { "min": min, "max": max },
            "cannotBorder": cannot_border
        }))
        .expect("test biome definition must deserialize")
    }

    fn layout(seed: u64, definitions: Vec<BiomeDefinition>) -> BiomeLayout {
        let mut registry = BiomeRegistry::default();
        for definition in definitions {
            registry.insert(definition);
        }
        let snapshot = GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::new("asteria:test", 64, 1.0),
        );
        BiomeLayout::new(&snapshot, &registry)
    }

    fn standard_definitions() -> Vec<BiomeDefinition> {
        vec![
            definition("asteria:test/a", 1.0, 192, 384, &["asteria:test/b"]),
            definition("asteria:test/b", 1.0, 192, 384, &["asteria:test/a"]),
            definition("asteria:test/c", 1.0, 192, 384, &[]),
            definition("asteria:test/d", 0.5, 256, 512, &[]),
        ]
    }

    fn standard_layout(seed: u64) -> BiomeLayout {
        layout(seed, standard_definitions())
    }

    #[test]
    fn surface_sampling_is_direct_and_order_independent() {
        let first = standard_layout(42);
        let second = standard_layout(42);
        let target = (1_500_000_000, -1_500_000_000);
        let direct = first.sample_surface(target.0, target.1);

        for position in [(0, 0), (128, -64), (-4096, 8192), (77_777, -88_888)] {
            let _ = first.sample_surface(position.0, position.1);
        }
        assert_eq!(first.sample_surface(target.0, target.1), direct);
        assert_eq!(second.sample_surface(target.0, target.1), direct);
    }

    #[test]
    fn registry_insertion_order_cannot_change_layout() {
        let forward = standard_definitions();
        let mut reverse = standard_definitions();
        reverse.reverse();
        let first = layout(57, forward);
        let second = layout(57, reverse);
        for z in (-700..=700).step_by(37) {
            for x in (-700..=700).step_by(41) {
                assert_eq!(first.sample_surface(x, z), second.sample_surface(x, z));
            }
        }
    }

    #[test]
    fn scalar_and_area_sampling_are_equivalent() {
        let layout = standard_layout(77);
        let area = layout.sample_surface_grid(-23, 41, 17, 13, 1);
        assert_eq!(area.origin(), (-23, 41));
        assert_eq!(area.width(), 17);
        assert_eq!(area.depth(), 13);
        assert_eq!(area.step(), 1);
        for z in 41..54 {
            for x in -23..-6 {
                assert_eq!(area.get(x, z), Some(&layout.sample_surface(x, z)));
            }
        }
    }

    #[test]
    fn scalar_and_strided_grid_sampling_are_equivalent() {
        let layout = standard_layout(78);
        let grid = layout.sample_surface_grid(-211, 83, 19, 11, 7);
        for z_index in 0..grid.depth() {
            for x_index in 0..grid.width() {
                let x = grid_axis(grid.origin().0, x_index, grid.step());
                let z = grid_axis(grid.origin().1, z_index, grid.step());
                assert_eq!(grid.sample_at(x_index, z_index), Some(&layout.sample_surface(x, z)));
                assert_eq!(grid.get(x, z), grid.sample_at(x_index, z_index));
            }
        }
    }

    #[test]
    fn influences_are_normalized_and_include_primary() {
        let layout = standard_layout(91);
        for z in (-512..=512).step_by(31) {
            for x in (-512..=512).step_by(29) {
                let sample = layout.sample_surface(x, z);
                let sum = sample
                    .influences()
                    .iter()
                    .map(BiomeInfluence::weight)
                    .sum::<f32>();
                assert!((sum - 1.0).abs() < 0.0001);
                assert!(
                    sample
                        .influences()
                        .iter()
                        .any(|influence| influence.biome() == sample.primary())
                );
            }
        }
    }

    #[test]
    fn authored_cannot_border_is_enforced_in_final_sampled_layout() {
        let layout = standard_layout(123);
        let grid = layout.sample_surface_grid(-1536, -1536, 193, 193, 16);
        for z in 0..grid.depth() {
            for x in 0..grid.width() {
                let sample = grid.sample_at(x, z).expect("sample must exist");
                for (neighbor_x, neighbor_z) in [(x + 1, z), (x, z + 1)] {
                    let Some(neighbor) = grid.sample_at(neighbor_x, neighbor_z) else {
                        continue;
                    };
                    let left = layout
                        .rules
                        .iter()
                        .position(|rule| rule.id == *sample.primary())
                        .expect("sample biome must be active");
                    let right = layout
                        .rules
                        .iter()
                        .position(|rule| rule.id == *neighbor.primary())
                        .expect("sample biome must be active");
                    assert!(layout.compatible_indices(left, right));
                }
            }
        }
    }

    #[test]
    fn formation_target_span_stays_inside_authored_range() {
        let layout = standard_layout(345);
        let mut cache = HashMap::new();
        for z in -8..=8 {
            for x in -8..=8 {
                let assignment = layout.cell_assignment(LayoutCell::new(x, z), &mut cache);
                let rule = &layout.rules[assignment.rule];
                assert!(assignment.target_span >= rule.region_min);
                assert!(assignment.target_span <= rule.region_max);
            }
        }
    }

    #[test]
    fn biome_search_uses_same_authoritative_samples() {
        let layout = standard_layout(777);
        let target = layout.rules[0].id.as_str().to_owned();
        let found = layout
            .find_surface_biome(&target, 0, 0, 20_000)
            .expect("authored biome should be discoverable");
        assert_eq!(found.sample().primary().as_str(), target);
        assert_eq!(
            layout.sample_surface(found.position().0, found.position().1),
            *found.sample()
        );
    }

    #[test]
    fn volume_field_is_optional_and_effective_biome_falls_back_to_surface() {
        let layout = standard_layout(999);
        let queries = layout.queries();
        assert!(queries.volume_biome_at(10, 500, -20).is_none());
        assert_eq!(
            queries.effective_biome_at(10, 500, -20),
            queries.surface_biome_at(10, -20)
        );
    }
}
