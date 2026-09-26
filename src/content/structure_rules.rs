use serde::Deserialize;

use super::{block::BlockRegistry, fluid::FluidRegistry};

pub const MAX_STRUCTURE_PROXIMITY_DISTANCE: u32 = 64;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructureReplacePolicy {
    #[default]
    Any,
    AirOnly,
    Terrain,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructureFluidPolicy {
    #[default]
    Displace,
    Preserve,
    Forbid,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructureProximityMode {
    Required,
    Forbidden,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructureProximityTarget {
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub fluid: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructureProximityRestriction {
    pub target: StructureProximityTarget,
    pub mode: StructureProximityMode,
    #[serde(default)]
    pub min_distance: Option<u32>,
    pub max_distance: u32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureGenerationRules {
    #[serde(default)]
    pub replace_policy: StructureReplacePolicy,
    #[serde(default)]
    pub fluid_policy: StructureFluidPolicy,
    #[serde(default)]
    pub reserve_space: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureRestrictions {
    #[serde(default)]
    pub ground_blocks: Vec<String>,
    #[serde(default = "default_max_slope")]
    pub max_slope: i32,
    #[serde(default = "default_requires_dry_ground")]
    pub requires_dry_ground: bool,
    #[serde(default)]
    pub required_biome_coverage: f32,
    #[serde(default)]
    pub min_y: Option<i32>,
    #[serde(default)]
    pub max_y: Option<i32>,
    #[serde(default)]
    pub proximity: Vec<StructureProximityRestriction>,
}

impl Default for StructureRestrictions {
    fn default() -> Self {
        Self {
            ground_blocks: Vec::new(),
            max_slope: default_max_slope(),
            requires_dry_ground: default_requires_dry_ground(),
            required_biome_coverage: 0.0,
            min_y: None,
            max_y: None,
            proximity: Vec::new(),
        }
    }
}

impl StructureRestrictions {
    pub(crate) fn validate(&self, structure_id: &str) {
        assert!(
            self.max_slope >= 0,
            "structure {structure_id} restrictions.maxSlope cannot be negative"
        );
        assert!(
            self.required_biome_coverage.is_finite()
                && (0.0..=1.0).contains(&self.required_biome_coverage),
            "structure {structure_id} restrictions.requiredBiomeCoverage must be between 0 and 1"
        );
        if let (Some(min_y), Some(max_y)) = (self.min_y, self.max_y) {
            assert!(
                max_y >= min_y,
                "structure {structure_id} restrictions.maxY must be greater than or equal to minY"
            );
        }

        for (index, block) in self.ground_blocks.iter().enumerate() {
            assert!(
                !block.trim().is_empty(),
                "structure {structure_id} restrictions.groundBlocks cannot contain empty ids"
            );
            assert!(
                !self.ground_blocks[..index].contains(block),
                "structure {structure_id} restrictions.groundBlocks cannot contain duplicates"
            );
        }

        for (index, proximity) in self.proximity.iter().enumerate() {
            let target_count =
                usize::from(proximity.target.block.is_some())
                    + usize::from(proximity.target.fluid.is_some());
            assert_eq!(
                target_count, 1,
                "structure {structure_id} restrictions.proximity[{index}].target must define exactly one of block or fluid"
            );
            if let Some(block) = proximity.target.block.as_deref() {
                assert!(
                    !block.trim().is_empty(),
                    "structure {structure_id} restrictions.proximity[{index}].target.block cannot be empty"
                );
            }
            if let Some(fluid) = proximity.target.fluid.as_deref() {
                assert!(
                    !fluid.trim().is_empty(),
                    "structure {structure_id} restrictions.proximity[{index}].target.fluid cannot be empty"
                );
            }
            if let Some(minimum) = proximity.min_distance {
                assert!(
                    minimum <= proximity.max_distance,
                    "structure {structure_id} restrictions.proximity[{index}].minDistance cannot exceed maxDistance"
                );
            }
            assert!(
                proximity.max_distance <= MAX_STRUCTURE_PROXIMITY_DISTANCE,
                "structure {structure_id} restrictions.proximity[{index}].maxDistance cannot exceed {MAX_STRUCTURE_PROXIMITY_DISTANCE}"
            );
            assert!(
                !self.proximity[..index].contains(proximity),
                "structure {structure_id} restrictions.proximity cannot contain duplicate rules"
            );
            assert!(
                !self.proximity[..index].iter().any(|previous| {
                    previous.target == proximity.target
                        && previous.min_distance == proximity.min_distance
                        && previous.max_distance == proximity.max_distance
                        && previous.mode != proximity.mode
                }),
                "structure {structure_id} restrictions.proximity cannot require and forbid the same target over the same distance range"
            );
        }
    }

    pub(crate) fn validate_references(
        &self,
        structure_id: &str,
        blocks: &BlockRegistry,
        fluids: &FluidRegistry,
    ) {
        for block in &self.ground_blocks {
            assert!(
                blocks.get(block).is_some(),
                "structure {structure_id} restrictions.groundBlocks references missing block: {block}"
            );
        }

        for (index, proximity) in self.proximity.iter().enumerate() {
            if let Some(block) = proximity.target.block.as_deref() {
                assert!(
                    blocks.get(block).is_some(),
                    "structure {structure_id} restrictions.proximity[{index}] references missing block: {block}"
                );
            }
            if let Some(fluid) = proximity.target.fluid.as_deref() {
                assert!(
                    fluids.id_of(fluid).is_some(),
                    "structure {structure_id} restrictions.proximity[{index}] references missing fluid: {fluid}"
                );
            }
        }
    }
}

fn default_max_slope() -> i32 {
    1
}

fn default_requires_dry_ground() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_legacy_structure_placement_behavior() {
        let restrictions = StructureRestrictions::default();
        let generation = StructureGenerationRules::default();

        assert_eq!(restrictions.max_slope, 1);
        assert!(restrictions.requires_dry_ground);
        assert_eq!(restrictions.required_biome_coverage, 0.0);
        assert_eq!(generation.replace_policy, StructureReplacePolicy::Any);
        assert_eq!(generation.fluid_policy, StructureFluidPolicy::Displace);
        assert!(!generation.reserve_space);
    }
}
