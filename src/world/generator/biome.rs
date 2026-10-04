use std::{collections::HashMap, f64::consts::TAU, sync::Arc};

use smallvec::SmallVec;

use crate::content::biome::{BiomeDefinition, BiomeRegistry};

use super::foundation::{
    GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationSnapshot,
};

const MIN_SEED_SPACING: u32 = 64;
const MAX_SEED_SPACING: u32 = 256;
const CANDIDATE_RADIUS_BUCKETS: i32 = 2;
const COMPATIBILITY_RADIUS_BUCKETS: i32 = CANDIDATE_RADIUS_BUCKETS * 2;
const COMPATIBILITY_CLASS_PERIOD: i32 = COMPATIBILITY_RADIUS_BUCKETS * 2 + 1;
const JITTER_FRACTION: f64 = 0.32;
const BLEND_SCORE_BAND: f64 = 0.18;
const FORMATION_TARGET_DOMAIN_PREFIX: &str = "biome-layout/formation-target/v2/";
const SEED_PICK_DOMAIN: &str = "biome-layout/seed-pick/v3";
const SEED_JITTER_X_DOMAIN: &str = "biome-layout/seed-jitter-x/v3";
const SEED_JITTER_Z_DOMAIN: &str = "biome-layout/seed-jitter-z/v3";
const SEED_SHAPE_A_DOMAIN: &str = "biome-layout/seed-shape-a/v3";
const SEED_SHAPE_B_DOMAIN: &str = "biome-layout/seed-shape-b/v3";
const SEED_BIAS_DOMAIN: &str = "biome-layout/seed-bias/v3";
const WARP_COARSE_X_DOMAIN: &str = "biome-layout/warp-coarse-x/v3";
const WARP_COARSE_Z_DOMAIN: &str = "biome-layout/warp-coarse-z/v3";
const WARP_FINE_X_DOMAIN: &str = "biome-layout/warp-fine-x/v3";
const WARP_FINE_Z_DOMAIN: &str = "biome-layout/warp-fine-z/v3";

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

    pub(crate) fn biome_region_ranges(&self) -> impl Iterator<Item = (&BiomeId, u32, u32)> {
        self.layout
            .rules
            .iter()
            .map(|rule| (&rule.id, rule.region_min, rule.region_max))
    }

    pub(crate) fn suppressed_biomes(&self) -> &[BiomeId] {
        &self.layout.suppressed_biomes
    }

    pub(crate) const fn formation_seed_spacing(&self) -> u32 {
        self.layout.seed_spacing
    }
}

#[derive(Clone, Debug)]
pub(super) struct BiomeLayout {
    entropy: GenerationEntropy,
    rules: Arc<[BiomeRule]>,
    suppressed_biomes: Arc<[BiomeId]>,
    seed_spacing: u32,
    fallback_rule: usize,
    seed_pick_domain: GenerationDomain,
    jitter_x_domain: GenerationDomain,
    jitter_z_domain: GenerationDomain,
    shape_a_domain: GenerationDomain,
    shape_b_domain: GenerationDomain,
    seed_bias_domain: GenerationDomain,
    warp_coarse_x_domain: GenerationDomain,
    warp_coarse_z_domain: GenerationDomain,
    warp_fine_x_domain: GenerationDomain,
    warp_fine_z_domain: GenerationDomain,
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

/// Spatial bucket used only to enumerate deterministic formation seeds.
///
/// It is deliberately not a biome region: final ownership comes from the
/// continuous organic score field produced by nearby seeds, so no biome border
/// is tied to bucket edges.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SeedBucket {
    x: i32,
    z: i32,
}

impl SeedBucket {
    const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    fn class(self) -> i32 {
        self.x.rem_euclid(COMPATIBILITY_CLASS_PERIOD) * COMPATIBILITY_CLASS_PERIOD
            + self.z.rem_euclid(COMPATIBILITY_CLASS_PERIOD)
    }

    fn as_point(self) -> GenerationPoint2 {
        GenerationPoint2::new(self.x, self.z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SeedAssignment {
    rule: usize,
    root: SeedBucket,
    target_span: u32,
}

#[derive(Clone, Copy, Debug)]
struct FormationSeed {
    bucket: SeedBucket,
    assignment: SeedAssignment,
    center_x: f64,
    center_z: f64,
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
        let seed_spacing = min_span
            .div_ceil(3)
            .clamp(MIN_SEED_SPACING, MAX_SEED_SPACING);

        Self {
            entropy: GenerationEntropy::new(snapshot),
            rules: active_rules.into(),
            suppressed_biomes: suppressed_biomes.into(),
            seed_spacing,
            fallback_rule,
            seed_pick_domain: GenerationDomain::named(SEED_PICK_DOMAIN),
            jitter_x_domain: GenerationDomain::named(SEED_JITTER_X_DOMAIN),
            jitter_z_domain: GenerationDomain::named(SEED_JITTER_Z_DOMAIN),
            shape_a_domain: GenerationDomain::named(SEED_SHAPE_A_DOMAIN),
            shape_b_domain: GenerationDomain::named(SEED_SHAPE_B_DOMAIN),
            seed_bias_domain: GenerationDomain::named(SEED_BIAS_DOMAIN),
            warp_coarse_x_domain: GenerationDomain::named(WARP_COARSE_X_DOMAIN),
            warp_coarse_z_domain: GenerationDomain::named(WARP_COARSE_Z_DOMAIN),
            warp_fine_x_domain: GenerationDomain::named(WARP_FINE_X_DOMAIN),
            warp_fine_z_domain: GenerationDomain::named(WARP_FINE_Z_DOMAIN),
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
        assignments: &mut HashMap<SeedBucket, SeedAssignment>,
    ) -> BiomeSample {
        let (warped_x, warped_z) = self.warped_position(x, z);
        let center_bucket = self.bucket_for_position(warped_x, warped_z);
        let mut best_scores = vec![None::<f64>; self.rules.len()];

        for dz in -CANDIDATE_RADIUS_BUCKETS..=CANDIDATE_RADIUS_BUCKETS {
            for dx in -CANDIDATE_RADIUS_BUCKETS..=CANDIDATE_RADIUS_BUCKETS {
                let Some(bucket_x) = center_bucket.x.checked_add(dx) else {
                    continue;
                };
                let Some(bucket_z) = center_bucket.z.checked_add(dz) else {
                    continue;
                };
                let seed = self.formation_seed(
                    SeedBucket::new(bucket_x, bucket_z),
                    assignments,
                );
                let score = self.seed_score(seed, warped_x, warped_z);
                let slot = &mut best_scores[seed.assignment.rule];
                if slot.is_none_or(|current| score > current) {
                    *slot = Some(score);
                }
            }
        }

        let (primary_rule, primary_score) = best_scores
            .iter()
            .enumerate()
            .filter_map(|(rule, score)| score.map(|score| (rule, score)))
            .max_by(|(left_rule, left_score), (right_rule, right_score)| {
                left_score
                    .total_cmp(right_score)
                    .then_with(|| self.rules[*right_rule].id.cmp(&self.rules[*left_rule].id))
            })
            .expect("biome sampling must have at least one formation seed");

        let mut weighted = best_scores
            .into_iter()
            .enumerate()
            .filter_map(|(rule, score)| {
                let score = score?;
                let delta = primary_score - score;
                if delta > BLEND_SCORE_BAND {
                    return None;
                }
                let proximity = (1.0 - delta / BLEND_SCORE_BAND).clamp(0.0, 1.0);
                Some((rule, smoothstep(proximity)))
            })
            .collect::<Vec<_>>();
        let total = weighted.iter().map(|(_, weight)| *weight).sum::<f64>();
        weighted.sort_by(|(left_rule, left_weight), (right_rule, right_weight)| {
            let left_primary = *left_rule == primary_rule;
            let right_primary = *right_rule == primary_rule;
            right_primary
                .cmp(&left_primary)
                .then_with(|| right_weight.total_cmp(left_weight))
                .then_with(|| self.rules[*left_rule].id.cmp(&self.rules[*right_rule].id))
        });
        let influences = weighted
            .into_iter()
            .map(|(rule, weight)| BiomeInfluence {
                biome: self.rules[rule].id.clone(),
                weight: (weight / total) as f32,
            })
            .collect::<SmallVec<[BiomeInfluence; 4]>>();

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
        let origin_sample = self.sample_surface(origin_x, origin_z);
        if origin_sample.primary().as_str() == biome_id {
            return Some(BiomeSearchResult {
                x: origin_x,
                z: origin_z,
                sample: origin_sample,
            });
        }

        let (warped_x, warped_z) = self.warped_position(origin_x, origin_z);
        let origin_bucket = self.bucket_for_position(warped_x, warped_z);
        let bucket_radius = max_distance.div_ceil(self.seed_spacing) as i32
            + CANDIDATE_RADIUS_BUCKETS
            + 2;
        let max_distance_sq = i128::from(max_distance) * i128::from(max_distance);
        let mut assignments = HashMap::new();
        let mut best: Option<(i128, BiomeSearchResult)> = None;

        for ring in 0..=bucket_radius {
            for bucket in ring_buckets(origin_bucket, ring) {
                if self.seed_assignment(bucket, &mut assignments).rule != target_rule {
                    continue;
                }
                let seed = self.formation_seed(bucket, &mut assignments);
                for (x, z) in self.search_probe_points(seed) {
                    let dx = i128::from(x) - i128::from(origin_x);
                    let dz = i128::from(z) - i128::from(origin_z);
                    let distance_sq = dx * dx + dz * dz;
                    if distance_sq > max_distance_sq
                        || best
                            .as_ref()
                            .is_some_and(|(best_sq, _)| distance_sq >= *best_sq)
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
                let conservative_ring = i128::from(
                    ring.saturating_sub(CANDIDATE_RADIUS_BUCKETS + 2),
                ) * i128::from(self.seed_spacing);
                if conservative_ring * conservative_ring > *best_sq {
                    break;
                }
            }
        }
        best.map(|(_, result)| result)
    }

    fn search_probe_points(&self, seed: FormationSeed) -> [(i32, i32); 9] {
        let center_x = seed.center_x.round() as i64;
        let center_z = seed.center_z.round() as i64;
        let offset = i64::from(self.seed_spacing / 3);
        let offsets = [-offset, 0, offset];
        let mut points = [(0_i32, 0_i32); 9];
        let mut index = 0;
        for z_offset in offsets {
            for x_offset in offsets {
                points[index] = (
                    clamp_world_axis(center_x + x_offset),
                    clamp_world_axis(center_z + z_offset),
                );
                index += 1;
            }
        }
        points
    }

    fn formation_seed(
        &self,
        bucket: SeedBucket,
        cache: &mut HashMap<SeedBucket, SeedAssignment>,
    ) -> FormationSeed {
        let assignment = self.seed_assignment(bucket, cache);
        let (center_x, center_z) = self.seed_center(bucket);
        FormationSeed {
            bucket,
            assignment,
            center_x,
            center_z,
        }
    }

    fn seed_assignment(
        &self,
        bucket: SeedBucket,
        cache: &mut HashMap<SeedBucket, SeedAssignment>,
    ) -> SeedAssignment {
        if let Some(assignment) = cache.get(&bucket) {
            return *assignment;
        }

        let class = bucket.class();
        let mut established = SmallVec::<[SeedAssignment; 64]>::new();
        for dz in -COMPATIBILITY_RADIUS_BUCKETS..=COMPATIBILITY_RADIUS_BUCKETS {
            for dx in -COMPATIBILITY_RADIUS_BUCKETS..=COMPATIBILITY_RADIUS_BUCKETS {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let Some(x) = bucket.x.checked_add(dx) else {
                    continue;
                };
                let Some(z) = bucket.z.checked_add(dz) else {
                    continue;
                };
                let neighbor = SeedBucket::new(x, z);
                if neighbor.class() < class {
                    established.push(self.seed_assignment(neighbor, cache));
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
                .map(|neighbor| self.growth_affinity(*neighbor, bucket))
                .max()
                .unwrap_or(0);
            let growth_multiplier = 1_u64.saturating_add(continuation.saturating_mul(3));
            let weight = self.rules[candidate]
                .weight_units
                .saturating_mul(growth_multiplier);
            total = total.saturating_add(weight);
            weighted.push((candidate, weight));
        }

        let pick = self.entropy.sample_2d(self.seed_pick_domain, bucket.as_point()) % total.max(1);
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
            .map(|neighbor| (*neighbor, self.growth_affinity(*neighbor, bucket)))
            .filter(|(_, affinity)| *affinity > 0)
            .max_by(|(left, left_affinity), (right, right_affinity)| {
                left_affinity
                    .cmp(right_affinity)
                    .then_with(|| right.root.cmp(&left.root))
            })
            .map(|(assignment, _)| assignment);
        let assignment = continued.unwrap_or_else(|| SeedAssignment {
            rule: selected_rule,
            root: bucket,
            target_span: self.formation_target_span(selected_rule, bucket),
        });
        cache.insert(bucket, assignment);
        assignment
    }

    fn formation_target_span(&self, rule: usize, root: SeedBucket) -> u32 {
        let definition = &self.rules[rule];
        random_span_u32(
            self.entropy.sample_2d(definition.target_domain, root.as_point()),
            definition.region_min,
            definition.region_max,
        )
    }

    fn growth_affinity(&self, assignment: SeedAssignment, target: SeedBucket) -> u64 {
        let (root_x, root_z) = self.seed_center(assignment.root);
        let (target_x, target_z) = self.seed_center(target);
        let dx = target_x - root_x;
        let dz = target_z - root_z;
        let distance = dx.hypot(dz);
        let radius = f64::from(assignment.target_span) * 0.5;
        if radius <= 0.0 || distance > radius {
            return 0;
        }
        let remaining = (1.0 - distance / radius).clamp(0.0, 1.0);
        (remaining * 16.0).round() as u64 + 1
    }

    fn compatible_indices(&self, left: usize, right: usize) -> bool {
        compatible_rules(&self.rules[left], &self.rules[right])
    }

    fn bucket_for_position(&self, x: f64, z: f64) -> SeedBucket {
        let spacing = f64::from(self.seed_spacing);
        SeedBucket::new(
            clamp_bucket_axis((x / spacing).floor()),
            clamp_bucket_axis((z / spacing).floor()),
        )
    }

    fn seed_center(&self, bucket: SeedBucket) -> (f64, f64) {
        let spacing = f64::from(self.seed_spacing);
        let base_x = (f64::from(bucket.x) + 0.5) * spacing;
        let base_z = (f64::from(bucket.z) + 0.5) * spacing;
        let jitter_x = signed_unit(
            self.entropy
                .sample_2d(self.jitter_x_domain, bucket.as_point()),
        ) * spacing
            * JITTER_FRACTION;
        let jitter_z = signed_unit(
            self.entropy
                .sample_2d(self.jitter_z_domain, bucket.as_point()),
        ) * spacing
            * JITTER_FRACTION;
        (base_x + jitter_x, base_z + jitter_z)
    }

    fn seed_score(&self, seed: FormationSeed, x: f64, z: f64) -> f64 {
        let spacing = f64::from(self.seed_spacing);
        let dx = x - seed.center_x;
        let dz = z - seed.center_z;
        let distance = dx.hypot(dz) / spacing;
        let angle = dz.atan2(dx);
        let phase_a = unit(
            self.entropy
                .sample_2d(self.shape_a_domain, seed.bucket.as_point()),
        ) * TAU;
        let phase_b = unit(
            self.entropy
                .sample_2d(self.shape_b_domain, seed.bucket.as_point()),
        ) * TAU;
        let shape = 1.0
            + 0.13 * (angle * 3.0 + phase_a).sin()
            + 0.07 * (angle * 5.0 + phase_b).sin();
        let target_ratio = (f64::from(seed.assignment.target_span) / (spacing * 3.0))
            .clamp(0.5, 3.0);
        let size_scale = target_ratio.powf(0.12);
        let bias = signed_unit(
            self.entropy
                .sample_2d(self.seed_bias_domain, seed.bucket.as_point()),
        ) * 0.045;
        let continuation_bonus = (seed.assignment.root != seed.bucket) as u8 as f64 * 0.055;
        -(distance / (shape * size_scale)) + bias + continuation_bonus
    }

    fn warped_position(&self, x: i32, z: i32) -> (f64, f64) {
        let spacing = f64::from(self.seed_spacing);
        let x_f = f64::from(x);
        let z_f = f64::from(z);
        let coarse_period = spacing * 4.0;
        let fine_period = spacing * 1.35;
        let coarse_x = self.smooth_noise(
            self.warp_coarse_x_domain,
            x_f,
            z_f,
            coarse_period,
        );
        let coarse_z = self.smooth_noise(
            self.warp_coarse_z_domain,
            x_f + coarse_period * 0.37,
            z_f - coarse_period * 0.61,
            coarse_period,
        );
        let fine_x = self.smooth_noise(
            self.warp_fine_x_domain,
            x_f - fine_period * 0.43,
            z_f + fine_period * 0.29,
            fine_period,
        );
        let fine_z = self.smooth_noise(
            self.warp_fine_z_domain,
            x_f + fine_period * 0.71,
            z_f + fine_period * 0.53,
            fine_period,
        );
        (
            x_f + coarse_x * spacing * 0.42 + fine_x * spacing * 0.16,
            z_f + coarse_z * spacing * 0.42 + fine_z * spacing * 0.16,
        )
    }

    fn smooth_noise(
        &self,
        domain: GenerationDomain,
        x: f64,
        z: f64,
        period: f64,
    ) -> f64 {
        let grid_x = (x / period)
            .floor()
            .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        let grid_z = (z / period)
            .floor()
            .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
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

fn ring_buckets(center: SeedBucket, ring: i32) -> Vec<SeedBucket> {
    if ring == 0 {
        return vec![center];
    }
    let mut buckets = Vec::with_capacity((ring * 8) as usize);
    for dx in -ring..=ring {
        if let (Some(x), Some(z)) = (center.x.checked_add(dx), center.z.checked_sub(ring)) {
            buckets.push(SeedBucket::new(x, z));
        }
    }
    for dz in (-ring + 1)..=ring {
        if let (Some(x), Some(z)) = (center.x.checked_add(ring), center.z.checked_add(dz)) {
            buckets.push(SeedBucket::new(x, z));
        }
    }
    for dx in (-ring..ring).rev() {
        if let (Some(x), Some(z)) = (center.x.checked_add(dx), center.z.checked_add(ring)) {
            buckets.push(SeedBucket::new(x, z));
        }
    }
    for dz in ((-ring + 1)..ring).rev() {
        if let (Some(x), Some(z)) = (center.x.checked_sub(ring), center.z.checked_add(dz)) {
            buckets.push(SeedBucket::new(x, z));
        }
    }
    buckets
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

fn clamp_bucket_axis(value: f64) -> i32 {
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

fn smoothstep(value: f64) -> f64 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(left: f64, right: f64, t: f64) -> f64 {
    left + (right - left) * t
}

#[cfg(test)]
mod tests {
    use super::*;
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
        for z in (-700..=700).step_by(53) {
            for x in (-700..=700).step_by(47) {
                assert_eq!(first.sample_surface(x, z), second.sample_surface(x, z));
            }
        }
    }

    #[test]
    fn scalar_and_grid_sampling_are_equivalent() {
        let layout = standard_layout(78);
        let grid = layout.sample_surface_grid(-211, 83, 19, 11, 7);
        for z_index in 0..grid.depth() {
            for x_index in 0..grid.width() {
                let x = grid_axis(grid.origin().0, x_index, grid.step());
                let z = grid_axis(grid.origin().1, z_index, grid.step());
                assert_eq!(grid.sample_at(x_index, z_index), Some(&layout.sample_surface(x, z)));
            }
        }
    }

    #[test]
    fn influences_are_normalized_and_include_primary() {
        let layout = standard_layout(91);
        for z in (-512..=512).step_by(41) {
            for x in (-512..=512).step_by(37) {
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
    fn authored_cannot_border_is_enforced_in_sampled_layout() {
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
