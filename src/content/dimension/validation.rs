use std::collections::HashSet;

use crate::content::{
    biome::{BiomeKind, BiomeRegistry},
    fluid::FluidRegistry,
};

use super::types::{DimensionBiome, DimensionBiomeSizeAxis, DimensionDefinition};

impl DimensionDefinition {
    pub fn validate_biomes(&self, biomes: &BiomeRegistry) {
        assert!(!self.id.trim().is_empty(), "dimension id cannot be empty");
        assert!(!self.day_night_cycle.trim().is_empty(), "dimension {} dayNightCycle cannot be empty", self.id);
        assert!(!self.sky.trim().is_empty(), "dimension {} sky cannot be empty", self.id);
        assert!(self.gravity_strength.is_finite() && self.gravity_strength >= 0.0, "dimension {} gravityStrength must be finite and non-negative", self.id);
        assert!(self.max_entities > 0, "dimension {} maxEntities must be positive", self.id);
        assert!(!self.biomes.is_empty(), "dimension {} must define at least one biome", self.id);

        let mut ids = HashSet::new();
        let mut has_active_surface = false;
        for entry in &self.biomes {
            assert!(ids.insert(entry.id.as_str()), "dimension {} defines biome more than once: {}", self.id, entry.id);
            assert!(entry.weight.is_finite() && entry.weight >= 0.0, "dimension {} biome {} weight must be finite and non-negative", self.id, entry.id);
            let biome = biomes.get(&entry.id).unwrap_or_else(|| panic!("dimension {} references missing biome: {}", self.id, entry.id));
            let size = entry.size.unwrap_or_else(|| panic!("dimension {} {:?} biome {} must define size", self.id, biome.kind, entry.id));
            validate_size_axis(&self.id, &entry.id, "x", size.x);
            validate_size_axis(&self.id, &entry.id, "z", size.z);
            match biome.kind {
                BiomeKind::Surface => {
                    if let Some(y) = size.y { validate_size_axis(&self.id, &entry.id, "y", y); }
                    if entry.weight > 0.0 { has_active_surface = true; }
                }
                BiomeKind::Volume => {
                    validate_size_axis(&self.id, &entry.id, "y", size.y.unwrap_or_else(|| panic!("dimension {} volume biome {} must define size.y", self.id, entry.id)));
                }
            }
        }
        assert!(has_active_surface, "dimension {} must define at least one surface biome with positive weight", self.id);

        for entry in &self.biomes {
            let biome = biomes.get(&entry.id).expect("biome validated above");
            if !entry.avoid_near.is_empty() {
                assert_eq!(biome.kind, BiomeKind::Surface, "dimension {} biome {} avoidNear is only valid for surface biomes", self.id, entry.id);
            }
            let mut avoided = HashSet::new();
            for id in &entry.avoid_near {
                assert!(avoided.insert(id.as_str()), "dimension {} biome {} avoidNear cannot contain duplicates: {}", self.id, entry.id, id);
                assert!(id != &entry.id, "dimension {} biome {} cannot avoid itself", self.id, entry.id);
                let avoided_entry = self.biomes.iter().find(|candidate| candidate.id == *id).unwrap_or_else(|| panic!("dimension {} biome {} avoidNear references missing biome: {}", self.id, entry.id, id));
                let avoided_biome = biomes.get(id).expect("biome validated above");
                assert_eq!(avoided_biome.kind, BiomeKind::Surface, "dimension {} biome {} avoidNear must reference a surface biome: {}", self.id, entry.id, id);
                assert!(avoided_entry.weight > 0.0, "dimension {} biome {} avoidNear references disabled biome {}", self.id, entry.id, id);
                assert!(!same_exclusive_group(entry, avoided_entry), "dimension {} biome {} avoidNear redundantly references {} from the same exclusiveNeighborGroup", self.id, entry.id, id);
            }
            if let Some(group) = entry.exclusive_neighbor_group.as_deref() {
                assert_eq!(biome.kind, BiomeKind::Surface, "dimension {} biome {} exclusiveNeighborGroup is only valid for surface biomes", self.id, entry.id);
                assert!(!group.trim().is_empty(), "dimension {} biome {} exclusiveNeighborGroup cannot be empty", self.id, entry.id);
                let active_members = self.biomes.iter().filter(|candidate| candidate.weight > 0.0 && candidate.exclusive_neighbor_group.as_deref() == Some(group)).count();
                assert!(active_members >= 2, "dimension {} exclusiveNeighborGroup {} must contain at least two active biomes", self.id, group);
            }
            let mut required = HashSet::new();
            if !entry.require_near.is_empty() {
                assert_eq!(biome.kind, BiomeKind::Surface, "dimension {} biome {} requireNear is only valid for surface biomes", self.id, entry.id);
            }
            for id in &entry.require_near {
                assert!(required.insert(id.as_str()), "dimension {} biome {} requireNear cannot contain duplicates: {}", self.id, entry.id, id);
                assert!(id != &entry.id, "dimension {} biome {} cannot require itself nearby", self.id, entry.id);
                assert!(!avoided.contains(id.as_str()), "dimension {} biome {} cannot both avoidNear and requireNear {}", self.id, entry.id, id);
                let required_biome = biomes.get(id).unwrap_or_else(|| panic!("dimension {} biome {} requireNear references missing biome: {}", self.id, entry.id, id));
                assert_eq!(required_biome.kind, BiomeKind::Surface, "dimension {} biome {} requireNear must reference a surface biome: {}", self.id, entry.id, id);
                let required_entry = self.biomes.iter().find(|candidate| candidate.id == *id).unwrap_or_else(|| panic!("dimension {} biome {} requireNear references biome outside this dimension: {}", self.id, entry.id, id));
                assert!(required_entry.weight > 0.0, "dimension {} biome {} requireNear references disabled biome {}", self.id, entry.id, id);
                assert!(!same_exclusive_group(entry, required_entry), "dimension {} biome {} cannot requireNear {} because both are in the same exclusiveNeighborGroup", self.id, entry.id, id);
            }
        }

        if let Some(id) = self.ocean_biome.as_deref() {
            let biome = biomes.get(id).unwrap_or_else(|| panic!("dimension {} oceanBiome references missing biome: {id}", self.id));
            assert_eq!(biome.kind, BiomeKind::Surface, "dimension {} oceanBiome must reference a surface biome: {id}", self.id);
            assert!(self.biomes.iter().any(|entry| entry.id == id), "dimension {} oceanBiome must also be listed in biomes: {id}", self.id);
        }
    }

    pub(crate) fn validate_fluid_references(&self, fluids: &FluidRegistry) {
        assert!(fluids.id_of(&self.sea_fluid).is_some(), "dimension {} seaFluid references missing fluid: {}", self.id, self.sea_fluid);
    }
}

fn same_exclusive_group(left: &DimensionBiome, right: &DimensionBiome) -> bool {
    left.exclusive_neighbor_group.as_deref().zip(right.exclusive_neighbor_group.as_deref()).is_some_and(|(left, right)| left == right)
}

fn validate_size_axis(dimension_id: &str, biome_id: &str, axis: &str, size: DimensionBiomeSizeAxis) {
    assert!(size.min > 0.0, "dimension {dimension_id} biome {biome_id} size.{axis}.min must be positive");
    assert!(size.max >= size.min, "dimension {dimension_id} biome {biome_id} size.{axis}.max must be greater than or equal to min");
}
