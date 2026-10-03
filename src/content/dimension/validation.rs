use std::collections::HashSet;

use crate::content::{
    biome::{BiomeKind, BiomeRegistry},
    fluid::FluidRegistry,
};

use super::types::{DimensionBiomeSizeAxis, DimensionDefinition};

impl DimensionDefinition {
    pub fn validate_biomes(&self, biomes: &BiomeRegistry) {
        assert!(!self.id.trim().is_empty(), "dimension id cannot be empty");
        assert!(
            !self.day_night_cycle.trim().is_empty(),
            "dimension {} dayNightCycle cannot be empty",
            self.id
        );
        assert!(
            !self.sky.trim().is_empty(),
            "dimension {} sky cannot be empty",
            self.id
        );
        assert!(
            self.gravity_strength.is_finite() && self.gravity_strength >= 0.0,
            "dimension {} gravityStrength must be finite and non-negative",
            self.id
        );
        assert!(
            self.max_entities > 0,
            "dimension {} maxEntities must be positive",
            self.id
        );
        assert!(
            !self.biomes.is_empty(),
            "dimension {} must define at least one biome",
            self.id
        );

        let mut ids = HashSet::new();
        let mut has_active_surface = false;
        for entry in &self.biomes {
            assert!(
                ids.insert(entry.id.as_str()),
                "dimension {} defines biome more than once: {}",
                self.id,
                entry.id
            );
            assert!(
                entry.weight.is_finite() && entry.weight >= 0.0,
                "dimension {} biome {} weight must be finite and non-negative",
                self.id,
                entry.id
            );
            assert!(
                entry.spawn_weight.is_finite() && entry.spawn_weight >= 0.0,
                "dimension {} biome {} spawnWeight must be finite and non-negative",
                self.id,
                entry.id
            );
            if let Some(group) = entry.exclusive_neighbor_group.as_deref() {
                assert!(
                    !group.trim().is_empty(),
                    "dimension {} biome {} exclusiveNeighborGroup cannot be empty",
                    self.id,
                    entry.id
                );
            }
            if let Some(selector) = &entry.neighbor_deny {
                selector.validate(&entry.id, "neighborDeny");
            }
            let biome = biomes.get(&entry.id).unwrap_or_else(|| {
                panic!(
                    "dimension {} references missing biome: {}",
                    self.id, entry.id
                )
            });
            let size = entry.size.unwrap_or_else(|| {
                panic!(
                    "dimension {} {:?} biome {} must define size",
                    self.id, biome.kind, entry.id
                )
            });
            validate_size_axis(&self.id, &entry.id, "x", size.x);
            validate_size_axis(&self.id, &entry.id, "z", size.z);
            match biome.kind {
                BiomeKind::Surface => {
                    if let Some(y) = size.y {
                        validate_size_axis(&self.id, &entry.id, "y", y);
                    }
                    if entry.weight > 0.0 {
                        has_active_surface = true;
                    }
                }
                BiomeKind::Volume => {
                    assert!(
                        entry.neighbor_deny.is_none(),
                        "dimension {} volume biome {} cannot define neighborDeny",
                        self.id,
                        entry.id
                    );
                    validate_size_axis(
                        &self.id,
                        &entry.id,
                        "y",
                        size.y.unwrap_or_else(|| {
                            panic!(
                                "dimension {} volume biome {} must define size.y",
                                self.id, entry.id
                            )
                        }),
                    );
                }
            }
        }
        assert!(
            has_active_surface,
            "dimension {} must define at least one surface biome with positive weight",
            self.id
        );

        for entry in &self.biomes {
            let Some(selector) = &entry.neighbor_deny else {
                continue;
            };
            for denied_id in &selector.ids {
                let denied = biomes.get(denied_id).unwrap_or_else(|| {
                    panic!(
                        "dimension {} biome {} neighborDeny references missing biome: {}",
                        self.id, entry.id, denied_id
                    )
                });
                assert_eq!(
                    denied.kind,
                    BiomeKind::Surface,
                    "dimension {} biome {} neighborDeny must reference a surface biome: {}",
                    self.id,
                    entry.id,
                    denied_id
                );
                assert!(
                    self.biomes.iter().any(|candidate| candidate.id == *denied_id),
                    "dimension {} biome {} neighborDeny references biome not listed in the dimension: {}",
                    self.id,
                    entry.id,
                    denied_id
                );
            }
        }

        if let Some(id) = self.ocean_biome.as_deref() {
            let biome = biomes.get(id).unwrap_or_else(|| {
                panic!(
                    "dimension {} oceanBiome references missing biome: {id}",
                    self.id
                )
            });
            assert_eq!(
                biome.kind,
                BiomeKind::Surface,
                "dimension {} oceanBiome must reference a surface biome: {id}",
                self.id
            );
            assert!(
                self.biomes.iter().any(|entry| entry.id == id),
                "dimension {} oceanBiome must also be listed in biomes: {id}",
                self.id
            );
        }
    }

    pub(crate) fn validate_fluid_references(&self, fluids: &FluidRegistry) {
        assert!(
            fluids.id_of(&self.sea_fluid).is_some(),
            "dimension {} seaFluid references missing fluid: {}",
            self.id,
            self.sea_fluid
        );
    }
}

fn validate_size_axis(
    dimension_id: &str,
    biome_id: &str,
    axis: &str,
    size: DimensionBiomeSizeAxis,
) {
    assert!(
        size.min > 0.0,
        "dimension {dimension_id} biome {biome_id} size.{axis}.min must be positive"
    );
    assert!(
        size.max >= size.min,
        "dimension {dimension_id} biome {biome_id} size.{axis}.max must be greater than or equal to min"
    );
}
