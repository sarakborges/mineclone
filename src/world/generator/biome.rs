use std::{
    collections::HashMap,
    sync::Arc,
};

use smallvec::SmallVec;

use crate::content::biome::{BiomeDefinition, BiomeRegistry};

use super::foundation::{
    GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationSnapshot,
};

const MIN_LAYOUT_CELL_SPAN: u32 = 64;
const MAX_CANDIDATE_RADIUS: i32 = 12;
const BLEND_SCORE_WIDTH: f64 = 0.18;
const CELL_PICK_DOMAIN: &str = "biome-layout/cell-pick/v1";
const SITE_JITTER_X_DOMAIN: &str = "biome-layout/site-jitter-x/v1";
const SITE_JITTER_Z_DOMAIN: &str = "biome-layout/site-jitter-z/v1";
const SITE_SPAN_DOMAIN: &str = "biome-layout/site-span/v1";
const SITE_ANGLE_DOMAIN: &str = "biome-layout/site-angle/v1";
const SITE_ASPECT_DOMAIN: &str = "biome-layout/site-aspect/v1";
const SITE_BIAS_DOMAIN: &str = "biome-layout/site-bias/v1";
const WARP_X_DOMAIN: &str = "biome-layout/warp-x/v1";
const WARP_Z_DOMAIN: &str = "biome-layout/warp-z/v1";

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

    pub(crate) fn get(&self, x: i32, z: i32) -> Option<&BiomeSample> {
        let dx = i64::from(x) - i64::from(self.origin_x);
        let dz = i64::from(z) - i64::from(self.origin_z);
        if dx < 0 || dz < 0 || dx >= i64::from(self.width) || dz >= i64::from(self.depth) {
            return None;
        }
        self.samples
            .get(dz as usize * self.width as usize + dx as usize)
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
            .sample_surface_area(origin_x, origin_z, width, depth)
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

    pub(crate) fn suppressed_biomes(&self) -> &[BiomeId] {
        &self.layout.suppressed_biomes
    }

    pub(crate) const fn layout_cell_span(&self) -> u32 {
        self.layout.cell_span
    }
}

#[derive(Clone, Debug)]
pub(super) struct BiomeLayout {
    entropy: GenerationEntropy,
    rules: Arc<[BiomeRule]>,
    suppressed_biomes: Arc<[BiomeId]>,
    cell_span: u32,
    candidate_radius: i32,
    fallback_rule: usize,
}

#[derive(Clone, Debug)]
struct BiomeRule {
    id: BiomeId,
    weight_units: u64,
    region_min: u32,
    region_max: u32,
    cannot_border: Arc<[BiomeId]>,
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
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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

#[derive(Clone, Copy, Debug)]
struct FormationSite {
    cell: LayoutCell,
    rule: usize,
    x: f64,
    z: f64,
    target_span: f64,
    angle: f64,
    aspect: f64,
    bias: f64,
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
        let fallback_id = all_rules[fallback_original].id.clone();
        let mut suppressed_biomes = Vec::new();
        let mut active_rules = Vec::new();
        for rule in all_rules {
            if compatible_rules_by_id(&rule, &fallback_id) {
                active_rules.push(rule);
            } else {
                suppressed_biomes.push(rule.id);
            }
        }
        active_rules.sort_by(|left, right| left.id.cmp(&right.id));
        let fallback_rule = active_rules
            .iter()
            .position(|rule| rule.id == fallback_id)
            .expect("fallback biome must remain active");

        let min_span = active_rules
            .iter()
            .map(|rule| rule.region_min)
            .min()
            .expect("active biome layout cannot be empty");
        let max_span = active_rules
            .iter()
            .map(|rule| rule.region_max)
            .max()
            .expect("active biome layout cannot be empty");
        let span_for_bounded_search = max_span.div_ceil(8);
        let cell_span = min_span
            .max(span_for_bounded_search)
            .max(MIN_LAYOUT_CELL_SPAN);
        let candidate_radius = i32::try_from(max_span.div_ceil(cell_span) + 2)
            .expect("validated biome span ratio must fit i32")
            .min(MAX_CANDIDATE_RADIUS);

        Self {
            entropy: GenerationEntropy::new(snapshot),
            rules: active_rules.into(),
            suppressed_biomes: suppressed_biomes.into(),
            cell_span,
            candidate_radius,
            fallback_rule,
        }
    }

    fn queries(&self) -> BiomeQueries<'_> {
        BiomeQueries { layout: self }
    }

    fn sample_surface(&self, x: i32, z: i32) -> BiomeSample {
        let mut cell_cache = HashMap::new();
        self.sample_surface_cached(x, z, &mut cell_cache)
    }

    fn sample_surface_area(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
    ) -> BiomeAreaSample {
        assert!(width > 0 && depth > 0, "biome sample area must be non-empty");
        validate_area_extent(origin_x, width, "X");
        validate_area_extent(origin_z, depth, "Z");
        let sample_count = u64::from(width)
            .checked_mul(u64::from(depth))
            .and_then(|count| usize::try_from(count).ok())
            .expect("biome sample area is too large");
        let mut samples = Vec::with_capacity(sample_count);
        let mut cell_cache = HashMap::new();
        for z_offset in 0..depth {
            let z = offset_axis(origin_z, z_offset);
            for x_offset in 0..width {
                let x = offset_axis(origin_x, x_offset);
                samples.push(self.sample_surface_cached(x, z, &mut cell_cache));
            }
        }
        BiomeAreaSample {
            origin_x,
            origin_z,
            width,
            depth,
            samples,
        }
    }

    fn sample_surface_cached(
        &self,
        x: i32,
        z: i32,
        cell_cache: &mut HashMap<LayoutCell, usize>,
    ) -> BiomeSample {
        let (warped_x, warped_z) = self.warped_position(x, z);
        let base_cell = self.cell_at(warped_x, warped_z);
        let mut best_scores = vec![f64::INFINITY; self.rules.len()];

        for dz in -self.candidate_radius..=self.candidate_radius {
            for dx in -self.candidate_radius..=self.candidate_radius {
                let Some(cell_x) = base_cell.x.checked_add(dx) else {
                    continue;
                };
                let Some(cell_z) = base_cell.z.checked_add(dz) else {
                    continue;
                };
                let site = self.formation_site(LayoutCell::new(cell_x, cell_z), cell_cache);
                let score = self.site_score(warped_x, warped_z, site);
                if score < best_scores[site.rule] {
                    best_scores[site.rule] = score;
                }
            }
        }

        let mut ranked = (0..self.rules.len()).collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            best_scores[*left]
                .total_cmp(&best_scores[*right])
                .then_with(|| self.rules[*left].id.cmp(&self.rules[*right].id))
        });
        let primary_rule = ranked[0];
        let primary_score = best_scores[primary_rule];
        let mut raw_influences = SmallVec::<[(usize, f64); 4]>::new();
        for rule in ranked {
            let delta = best_scores[rule] - primary_score;
            if delta > BLEND_SCORE_WIDTH {
                break;
            }
            let normalized = (1.0 - delta / BLEND_SCORE_WIDTH).clamp(0.0, 1.0);
            let raw = normalized * normalized;
            if raw > 0.0 {
                raw_influences.push((rule, raw));
            }
        }
        let total = raw_influences.iter().map(|(_, weight)| *weight).sum::<f64>();
        let influences = raw_influences
            .into_iter()
            .map(|(rule, weight)| BiomeInfluence {
                biome: self.rules[rule].id.clone(),
                weight: (weight / total) as f32,
            })
            .collect::<SmallVec<_>>();

        BiomeSample {
            primary: self.rules[primary_rule].id.clone(),
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
        let origin_cell = self.cell_at(f64::from(origin_x), f64::from(origin_z));
        let cell_radius = max_distance.div_ceil(self.cell_span) as i32 + 2;
        let max_distance_sq = i128::from(max_distance) * i128::from(max_distance);
        let mut cell_cache = HashMap::new();
        let mut best: Option<(i128, BiomeSearchResult)> = None;

        for ring in 0..=cell_radius {
            for cell in ring_cells(origin_cell, ring) {
                if self.cell_rule(cell, &mut cell_cache) != target_rule {
                    continue;
                }
                let site = self.formation_site(cell, &mut cell_cache);
                for (x, z) in self.search_probe_points(site) {
                    let dx = i128::from(x) - i128::from(origin_x);
                    let dz = i128::from(z) - i128::from(origin_z);
                    let distance_sq = dx * dx + dz * dz;
                    if distance_sq > max_distance_sq
                        || best.as_ref().is_some_and(|(best_sq, _)| distance_sq >= *best_sq)
                    {
                        continue;
                    }
                    let sample = self.sample_surface_cached(x, z, &mut cell_cache);
                    if sample.primary().as_str() == biome_id {
                        best = Some((distance_sq, BiomeSearchResult { x, z, sample }));
                    }
                }
            }

            if let Some((best_sq, _)) = &best {
                let next_ring_min = i128::from(ring.saturating_sub(1))
                    * i128::from(self.cell_span);
                if next_ring_min * next_ring_min > *best_sq {
                    break;
                }
            }
        }
        best.map(|(_, result)| result)
    }

    fn search_probe_points(&self, site: FormationSite) -> [(i32, i32); 5] {
        let half = i64::from(self.cell_span) / 4;
        let center_x = site.x.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        let center_z = site.z.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        [
            (center_x, center_z),
            (saturating_offset(center_x, half), center_z),
            (saturating_offset(center_x, -half), center_z),
            (center_x, saturating_offset(center_z, half)),
            (center_x, saturating_offset(center_z, -half)),
        ]
    }

    fn formation_site(
        &self,
        cell: LayoutCell,
        cell_cache: &mut HashMap<LayoutCell, usize>,
    ) -> FormationSite {
        let rule = self.cell_rule(cell, cell_cache);
        let point = cell.as_point();
        let cell_span = f64::from(self.cell_span);
        let center_x = (f64::from(cell.x) + 0.5) * cell_span;
        let center_z = (f64::from(cell.z) + 0.5) * cell_span;
        let jitter_x = signed_unit(self.entropy.sample_2d(
            GenerationDomain::named(SITE_JITTER_X_DOMAIN),
            point,
        )) * cell_span
            * 0.32;
        let jitter_z = signed_unit(self.entropy.sample_2d(
            GenerationDomain::named(SITE_JITTER_Z_DOMAIN),
            point,
        )) * cell_span
            * 0.32;
        let span_hash = self
            .entropy
            .sample_2d(GenerationDomain::named(SITE_SPAN_DOMAIN), point);
        let target_span = random_span(
            span_hash,
            self.rules[rule].region_min,
            self.rules[rule].region_max,
        );
        let angle = unit(self.entropy.sample_2d(
            GenerationDomain::named(SITE_ANGLE_DOMAIN),
            point,
        )) * std::f64::consts::TAU;
        let aspect = 0.72
            + unit(self.entropy.sample_2d(
                GenerationDomain::named(SITE_ASPECT_DOMAIN),
                point,
            )) * 0.68;
        let bias = signed_unit(self.entropy.sample_2d(
            GenerationDomain::named(SITE_BIAS_DOMAIN),
            point,
        )) * 0.045;
        FormationSite {
            cell,
            rule,
            x: center_x + jitter_x,
            z: center_z + jitter_z,
            target_span,
            angle,
            aspect,
            bias,
        }
    }

    fn site_score(&self, x: f64, z: f64, site: FormationSite) -> f64 {
        let dx = x - site.x;
        let dz = z - site.z;
        let cos = site.angle.cos();
        let sin = site.angle.sin();
        let along = dx * cos + dz * sin;
        let across = -dx * sin + dz * cos;
        let elliptical = ((along / site.aspect).powi(2)
            + (across * site.aspect).powi(2))
        .sqrt();
        elliptical / site.target_span + site.bias
    }

    fn cell_rule(
        &self,
        cell: LayoutCell,
        cache: &mut HashMap<LayoutCell, usize>,
    ) -> usize {
        if let Some(rule) = cache.get(&cell) {
            return *rule;
        }
        let class = cell.class();
        let mut established = SmallVec::<[usize; 8]>::new();
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
                        established.push(self.cell_rule(neighbor, cache));
                    }
                }
            }
        }

        let eligible = (0..self.rules.len())
            .filter(|candidate| {
                established
                    .iter()
                    .all(|neighbor| self.compatible_indices(*candidate, *neighbor))
            })
            .collect::<SmallVec<[usize; 16]>>();
        let eligible = if eligible.is_empty() {
            debug_assert!(
                established
                    .iter()
                    .all(|neighbor| self.compatible_indices(self.fallback_rule, *neighbor)),
                "fallback biome must be compatible with every active biome"
            );
            SmallVec::from_slice(&[self.fallback_rule])
        } else {
            eligible
        };

        let mut weighted = SmallVec::<[(usize, u64); 16]>::new();
        let mut total = 0_u64;
        for candidate in eligible {
            let same_neighbors = established
                .iter()
                .filter(|neighbor| **neighbor == candidate)
                .count() as u64;
            let target_cells = u64::from(
                ((self.rules[candidate].region_min + self.rules[candidate].region_max) / 2)
                    .div_ceil(self.cell_span)
                    .max(1),
            );
            let growth = 1_u64.saturating_add(same_neighbors.saturating_mul(target_cells));
            let weight = self.rules[candidate].weight_units.saturating_mul(growth);
            total = total.saturating_add(weight);
            weighted.push((candidate, weight));
        }
        let pick = self.entropy.sample_2d(
            GenerationDomain::named(CELL_PICK_DOMAIN),
            cell.as_point(),
        ) % total.max(1);
        let mut cursor = 0_u64;
        let selected = weighted
            .iter()
            .find_map(|(candidate, weight)| {
                cursor = cursor.saturating_add(*weight);
                (pick < cursor).then_some(*candidate)
            })
            .unwrap_or_else(|| weighted.last().expect("eligible biome pool cannot be empty").0);
        cache.insert(cell, selected);
        selected
    }

    fn compatible_indices(&self, left: usize, right: usize) -> bool {
        compatible_rules(&self.rules[left], &self.rules[right])
    }

    fn cell_at(&self, x: f64, z: f64) -> LayoutCell {
        let span = f64::from(self.cell_span);
        LayoutCell::new(
            (x / span).floor().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32,
            (z / span).floor().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32,
        )
    }

    fn warped_position(&self, x: i32, z: i32) -> (f64, f64) {
        let period = f64::from(self.cell_span) * 3.0;
        let amplitude = f64::from(self.cell_span) * 0.36;
        let x_f = f64::from(x);
        let z_f = f64::from(z);
        let warp_x = self.smooth_noise(
            GenerationDomain::named(WARP_X_DOMAIN),
            x_f,
            z_f,
            period,
        );
        let warp_z = self.smooth_noise(
            GenerationDomain::named(WARP_Z_DOMAIN),
            x_f + period * 0.37,
            z_f - period * 0.61,
            period,
        );
        (x_f + warp_x * amplitude, z_f + warp_z * amplitude)
    }

    fn smooth_noise(
        &self,
        domain: GenerationDomain,
        x: f64,
        z: f64,
        period: f64,
    ) -> f64 {
        let grid_x = (x / period).floor() as i32;
        let grid_z = (z / period).floor() as i32;
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

fn compatible_rules_by_id(rule: &BiomeRule, other_id: &BiomeId) -> bool {
    !rule.cannot_border.iter().any(|id| id == other_id)
}

fn compatible_rules(left: &BiomeRule, right: &BiomeRule) -> bool {
    left.id == right.id
        || (!left.cannot_border.iter().any(|id| id == &right.id)
            && !right.cannot_border.iter().any(|id| id == &left.id))
}

fn quantize_weight(weight: f32) -> u64 {
    (f64::from(weight) * 4096.0).round().max(1.0) as u64
}

fn random_span(hash: u64, min: u32, max: u32) -> f64 {
    if min == max {
        return f64::from(min);
    }
    let range = u64::from(max - min) + 1;
    f64::from(min + (hash % range) as u32)
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

fn validate_area_extent(origin: i32, extent: u32, axis: &str) {
    let last = i64::from(origin) + i64::from(extent) - 1;
    assert!(
        last <= i64::from(i32::MAX),
        "biome sample {axis} extent exceeds world coordinate range"
    );
}

fn offset_axis(origin: i32, offset: u32) -> i32 {
    i32::try_from(i64::from(origin) + i64::from(offset))
        .expect("validated biome sample coordinate must fit i32")
}

fn saturating_offset(value: i32, offset: i64) -> i32 {
    (i64::from(value) + offset).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
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
    use crate::content::biome::BiomeRegistry;
    use super::super::foundation::{GenerationDimension, GenerationSeed};

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

    fn standard_layout(seed: u64) -> BiomeLayout {
        layout(
            seed,
            vec![
                definition("asteria:test/a", 1.0, 192, 384, &["asteria:test/b"]),
                definition("asteria:test/b", 1.0, 192, 384, &["asteria:test/a"]),
                definition("asteria:test/c", 1.0, 192, 384, &[]),
                definition("asteria:test/d", 0.5, 256, 512, &[]),
            ],
        )
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
    fn scalar_and_area_sampling_are_equivalent() {
        let layout = standard_layout(77);
        let area = layout.sample_surface_area(-23, 41, 17, 13);
        for z in 41..54 {
            for x in -23..-6 {
                assert_eq!(area.get(x, z), Some(&layout.sample_surface(x, z)));
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
    fn authored_cannot_border_is_enforced_between_neighboring_layout_cells() {
        let layout = standard_layout(123);
        let mut cache = HashMap::new();
        for z in -12..=12 {
            for x in -12..=12 {
                let cell = LayoutCell::new(x, z);
                let rule = layout.cell_rule(cell, &mut cache);
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let neighbor = LayoutCell::new(x + dx, z + dz);
                        let neighbor_rule = layout.cell_rule(neighbor, &mut cache);
                        assert!(layout.compatible_indices(rule, neighbor_rule));
                    }
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
                let site = layout.formation_site(LayoutCell::new(x, z), &mut cache);
                let rule = &layout.rules[site.rule];
                assert!(site.target_span >= f64::from(rule.region_min));
                assert!(site.target_span <= f64::from(rule.region_max));
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
