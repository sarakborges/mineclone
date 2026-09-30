use std::{collections::HashMap, sync::Arc};

use bevy::prelude::*;

use crate::content::{
    biome::BiomeRegistry,
    biome_structure::{BiomeStructurePlacementRules, StructurePlacementRules},
    structure::StructureRegistry,
    structure_set::StructureSetRegistry,
};

use super::structure_metadata::planning::{
    connected_horizontal_bounds_for_reference, structure_candidate_anchor,
};

#[derive(Clone, Debug)]
pub(crate) struct SurfaceStructureFieldEntry {
    pub(crate) biome_id: String,
    pub(crate) reference: String,
    pub(crate) placement: StructurePlacementRules,
    pub(crate) horizontal_bounds: (IVec2, IVec2),
}

#[derive(Clone)]
pub(crate) struct StructureField {
    seed: u64,
    surface_entries: Arc<Vec<SurfaceStructureFieldEntry>>,
    reference_bounds: Arc<HashMap<String, (IVec2, IVec2)>>,
}

impl StructureField {
    pub(crate) fn empty(seed: u64) -> Self {
        Self {
            seed,
            surface_entries: Arc::new(Vec::new()),
            reference_bounds: Arc::new(HashMap::new()),
        }
    }

    pub(crate) fn from_content(
        seed: u64,
        biomes: &BiomeRegistry,
        structures: &StructureRegistry,
        structure_sets: &StructureSetRegistry,
    ) -> Self {
        let mut reference_bounds = HashMap::<String, (IVec2, IVec2)>::new();
        let mut surface_entries = Vec::new();

        for biome_structure in biomes.structure_placements() {
            let reference = biome_structure.structure_id.as_str();
            let horizontal_bounds = cached_structure_reference_bounds(
                &mut reference_bounds,
                structures,
                structure_sets,
                reference,
            );

            let BiomeStructurePlacementRules::Surface(placement) = biome_structure.placement else {
                continue;
            };
            let horizontal_bounds = horizontal_bounds.unwrap_or_else(|| {
                panic!(
                    "surface biome {} references missing structure or structure group: {reference}",
                    biome_structure.biome_id
                )
            });

            surface_entries.push(SurfaceStructureFieldEntry {
                biome_id: biome_structure.biome_id.clone(),
                reference: biome_structure.structure_id.clone(),
                placement,
                horizontal_bounds,
            });
        }

        surface_entries.sort_by(|left, right| {
            left.biome_id
                .cmp(&right.biome_id)
                .then_with(|| left.reference.cmp(&right.reference))
        });

        Self {
            seed,
            surface_entries: Arc::new(surface_entries),
            reference_bounds: Arc::new(reference_bounds),
        }
    }

    pub(crate) fn surface_entries(&self) -> &[SurfaceStructureFieldEntry] {
        &self.surface_entries
    }

    pub(crate) fn reference_bounds(&self, reference: &str) -> Option<(IVec2, IVec2)> {
        self.reference_bounds.get(reference).copied()
    }

    pub(crate) fn visit_surface_anchors_intersecting(
        &self,
        entry: &SurfaceStructureFieldEntry,
        target_minimum: IVec2,
        target_maximum: IVec2,
        mut visit: impl FnMut(IVec2),
    ) {
        let (minimum_offset, maximum_offset) = entry.horizontal_bounds;
        let minimum_candidate = target_minimum - maximum_offset;
        let maximum_candidate = target_maximum - minimum_offset;
        let spacing = entry.placement.spacing;
        let minimum_cell = IVec2::new(
            minimum_candidate.x.div_euclid(spacing) - 1,
            minimum_candidate.y.div_euclid(spacing) - 1,
        );
        let maximum_cell = IVec2::new(
            maximum_candidate.x.div_euclid(spacing) + 1,
            maximum_candidate.y.div_euclid(spacing) + 1,
        );

        for cell_z in minimum_cell.y..=maximum_cell.y {
            for cell_x in minimum_cell.x..=maximum_cell.x {
                let cell = IVec2::new(cell_x, cell_z);
                let Some(anchor) = structure_candidate_anchor(
                    self.seed,
                    &entry.biome_id,
                    &entry.reference,
                    entry.placement,
                    cell,
                ) else {
                    continue;
                };
                let minimum = anchor + minimum_offset;
                let maximum = anchor + maximum_offset;
                if rectangles_overlap(minimum, maximum, target_minimum, target_maximum) {
                    visit(anchor);
                }
            }
        }
    }
}

fn cached_structure_reference_bounds(
    cache: &mut HashMap<String, (IVec2, IVec2)>,
    structures: &StructureRegistry,
    structure_sets: &StructureSetRegistry,
    reference: &str,
) -> Option<(IVec2, IVec2)> {
    if let Some(bounds) = cache.get(reference) {
        return Some(*bounds);
    }

    let bounds = if let Some(set) = structure_sets.get(reference) {
        set.horizontal_bounds_with(|member_reference| {
            cached_reference_bounds(cache, structures, member_reference)
        })?
    } else {
        connected_horizontal_bounds_for_reference(structures, reference)?
    };
    cache.insert(reference.to_owned(), bounds);
    Some(bounds)
}

fn cached_reference_bounds(
    cache: &mut HashMap<String, (IVec2, IVec2)>,
    structures: &StructureRegistry,
    reference: &str,
) -> Option<(IVec2, IVec2)> {
    if let Some(bounds) = cache.get(reference) {
        return Some(*bounds);
    }

    let bounds = connected_horizontal_bounds_for_reference(structures, reference)?;
    cache.insert(reference.to_owned(), bounds);
    Some(bounds)
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
