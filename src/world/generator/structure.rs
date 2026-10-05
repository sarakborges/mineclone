use std::sync::Arc;

use bevy::prelude::{IVec2, IVec3};

use crate::content::{
    biome::BiomeRegistry,
    dimension::GeneratedSurfaceStructureDefinition,
    structure::{StructureDefinition, StructureRegistry, StructureRotation},
    structure_rules::{StructureFluidPolicy, StructureProximityMode, StructureProximityTarget},
};

use super::{
    biome::BiomeLayout,
    foundation::{GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationSnapshot},
    material::MaterialField,
    terrain::TerrainField,
};

const PRESENCE_DOMAIN_PREFIX: &str = "structure/root/presence/v1/";
const JITTER_X_DOMAIN_PREFIX: &str = "structure/root/jitter-x/v1/";
const JITTER_Z_DOMAIN_PREFIX: &str = "structure/root/jitter-z/v1/";
const VARIANT_DOMAIN_PREFIX: &str = "structure/root/variant/v1/";
const ROTATION_DOMAIN_PREFIX: &str = "structure/root/rotation/v1/";

#[derive(Clone, Debug)]
pub(crate) struct StructurePlacement {
    reference: Arc<str>,
    structure: Arc<StructureDefinition>,
    rotation: StructureRotation,
    origin: IVec3,
}

impl StructurePlacement {
    pub(crate) fn reference(&self) -> &str {
        &self.reference
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

#[derive(Clone, Copy)]
pub(crate) struct StructureQueries<'a> {
    field: &'a StructureField,
}

impl StructureQueries<'_> {
    /// Returns authoritative root Structure placements whose horizontal bounds
    /// intersect the requested world-space rectangle. The request does not
    /// materialize chunks and does not affect future results.
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

    /// Finds the nearest authoritative root placement for an authored Structure
    /// or Structure-group reference without materializing the searched area.
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
    biomes: Arc<BiomeLayout>,
    terrain: Arc<TerrainField>,
    materials: Arc<MaterialField>,
    rules: Arc<[RootStructureRule]>,
}

#[derive(Clone, Debug)]
struct RootStructureRule {
    biome: Arc<str>,
    reference: Arc<str>,
    spacing: i32,
    jitter: u32,
    chance: f32,
    variants: Arc<[Arc<StructureDefinition>]>,
    maximum_horizontal_extent: i32,
    presence_domain: GenerationDomain,
    jitter_x_domain: GenerationDomain,
    jitter_z_domain: GenerationDomain,
    variant_domain: GenerationDomain,
    rotation_domain: GenerationDomain,
}

impl StructureField {
    pub(super) fn new(
        snapshot: &GenerationSnapshot,
        biome_registry: &BiomeRegistry,
        structure_registry: &StructureRegistry,
        biomes: Arc<BiomeLayout>,
        terrain: Arc<TerrainField>,
        materials: Arc<MaterialField>,
        definitions: &[GeneratedSurfaceStructureDefinition],
    ) -> Self {
        let dimension_id = snapshot.dimension().id();
        let rules = definitions
            .iter()
            .map(|definition| {
                let biome = biome_registry.get(&definition.biome).unwrap_or_else(|| {
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
                RootStructureRule::new(definition, structure_registry)
            })
            .collect::<Vec<_>>()
            .into();

        Self {
            entropy: GenerationEntropy::new(snapshot),
            biomes,
            terrain,
            materials,
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
        let mut placements = Vec::new();

        for rule in self.rules.iter() {
            let padding = rule
                .maximum_horizontal_extent
                .saturating_add(i32::try_from(rule.jitter).unwrap_or(i32::MAX));
            let minimum = query_minimum - IVec2::splat(padding);
            let maximum = query_maximum + IVec2::splat(padding);
            let (minimum_cell, maximum_cell) = candidate_cell_bounds(rule.spacing, minimum, maximum);

            for cell_z in minimum_cell.y..=maximum_cell.y {
                for cell_x in minimum_cell.x..=maximum_cell.x {
                    let cell = GenerationPoint2::new(cell_x, cell_z);
                    let Some(placement) = self.resolve_candidate(rule, cell) else {
                        continue;
                    };
                    let (placement_minimum, placement_maximum) = placement.horizontal_bounds();
                    if rectangles_overlap(
                        placement_minimum,
                        placement_maximum,
                        query_minimum,
                        query_maximum,
                    ) {
                        placements.push(placement);
                    }
                }
            }
        }

        placements.sort_by(|left, right| {
            left.origin
                .z
                .cmp(&right.origin.z)
                .then_with(|| left.origin.x.cmp(&right.origin.x))
                .then_with(|| left.origin.y.cmp(&right.origin.y))
                .then_with(|| left.structure.id.cmp(&right.structure.id))
        });
        placements
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
        let minimum = origin - IVec2::splat(radius);
        let maximum = origin + IVec2::splat(radius);
        let mut best: Option<(i128, StructurePlacement)> = None;

        for rule in self.rules.iter().filter(|rule| rule.reference.as_ref() == reference) {
            let (minimum_cell, maximum_cell) = candidate_cell_bounds(rule.spacing, minimum, maximum);
            for cell_z in minimum_cell.y..=maximum_cell.y {
                for cell_x in minimum_cell.x..=maximum_cell.x {
                    let Some(placement) =
                        self.resolve_candidate(rule, GenerationPoint2::new(cell_x, cell_z))
                    else {
                        continue;
                    };
                    let delta = placement.origin.xz() - origin;
                    let distance_squared = i128::from(delta.x) * i128::from(delta.x)
                        + i128::from(delta.y) * i128::from(delta.y);
                    if distance_squared > maximum_squared {
                        continue;
                    }
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

    fn resolve_candidate(
        &self,
        rule: &RootStructureRule,
        cell: GenerationPoint2,
    ) -> Option<StructurePlacement> {
        if unit_probability(self.entropy.sample_2d(rule.presence_domain, cell))
            > f64::from(rule.chance)
        {
            return None;
        }

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
        let anchor = IVec2::new(i32::try_from(anchor_x).ok()?, i32::try_from(anchor_z).ok()?);

        let biome_sample = self.biomes.queries().surface_biome_at(anchor.x, anchor.y);
        if biome_sample.primary().as_str() != rule.biome.as_ref() {
            return None;
        }

        let variant_hash = self.entropy.sample_2d(rule.variant_domain, cell);
        let variant_index = usize::try_from(variant_hash % rule.variants.len() as u64).ok()?;
        let structure = Arc::clone(rule.variants.get(variant_index)?);
        let rotation = structure.rotation_for_hash(self.entropy.sample_2d(rule.rotation_domain, cell));
        let origin_y = self.fit_origin_y(&structure, rotation, anchor)?;

        self.satisfies_restrictions(rule, &structure, rotation, anchor, origin_y)
            .then_some(StructurePlacement {
                reference: Arc::clone(&rule.reference),
                structure,
                rotation,
                origin: IVec3::new(anchor.x, origin_y, anchor.y),
            })
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
            let position = anchor + offset;
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
                    let position = anchor + offset;
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
        {
            return false;
        }

        if restrictions.required_biome_coverage > 0.0 {
            let footprint = structure.horizontal_footprint_for_rotation(rotation);
            let matching = footprint
                .iter()
                .filter(|offset| {
                    let position = anchor + **offset;
                    self.biomes
                        .queries()
                        .surface_biome_at(position.x, position.y)
                        .primary()
                        .as_str()
                        == rule.biome.as_ref()
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
        let radius = maximum_distance as i32;
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
                if self.proximity_target_matches(anchor + IVec2::new(x, z), target) {
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
        let radius = maximum_distance as i32;
        let minimum_squared = i64::from(minimum_distance) * i64::from(minimum_distance);
        let maximum_squared = i64::from(maximum_distance) * i64::from(maximum_distance);
        for z in -radius..=radius {
            for x in -radius..=radius {
                let distance_squared = i64::from(x) * i64::from(x) + i64::from(z) * i64::from(z);
                if distance_squared < minimum_squared || distance_squared > maximum_squared {
                    continue;
                }
                if self.proximity_target_matches(anchor + IVec2::new(x, z), target) {
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
    ) -> Self {
        let variants = structures
            .reference_members(&definition.structure)
            .unwrap_or_else(|| {
                panic!(
                    "generated surface structure rule references missing Structure or Structure group: {}",
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
        let maximum_horizontal_extent = variants
            .iter()
            .flat_map(|structure| {
                structure.supported_rotations().iter().map(|rotation| {
                    let (minimum, maximum) = structure.horizontal_bounds_for_rotation(*rotation);
                    minimum
                        .x
                        .unsigned_abs()
                        .max(minimum.y.unsigned_abs())
                        .max(maximum.x.unsigned_abs())
                        .max(maximum.y.unsigned_abs())
                })
            })
            .max()
            .and_then(|extent| i32::try_from(extent).ok())
            .unwrap_or(0);
        let suffix = format!("{}/{}", definition.biome, definition.structure);

        Self {
            biome: Arc::from(definition.biome.as_str()),
            reference: Arc::from(definition.structure.as_str()),
            spacing: i32::try_from(definition.spacing)
                .expect("validated structure spacing must fit i32"),
            jitter: definition.jitter,
            chance: definition.chance,
            variants: variants.into(),
            maximum_horizontal_extent,
            presence_domain: GenerationDomain::named(&format!("{PRESENCE_DOMAIN_PREFIX}{suffix}")),
            jitter_x_domain: GenerationDomain::named(&format!("{JITTER_X_DOMAIN_PREFIX}{suffix}")),
            jitter_z_domain: GenerationDomain::named(&format!("{JITTER_Z_DOMAIN_PREFIX}{suffix}")),
            variant_domain: GenerationDomain::named(&format!("{VARIANT_DOMAIN_PREFIX}{suffix}")),
            rotation_domain: GenerationDomain::named(&format!("{ROTATION_DOMAIN_PREFIX}{suffix}")),
        }
    }
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

fn placement_sort_key(placement: &StructurePlacement) -> (i32, i32, i32, &str) {
    (
        placement.origin.z,
        placement.origin.x,
        placement.origin.y,
        placement.structure.id.as_str(),
    )
}
