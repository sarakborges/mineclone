use serde::Deserialize;

use super::block::BlockRegistry;

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
    }

    pub(crate) fn validate_references(&self, structure_id: &str, blocks: &BlockRegistry) {
        for block in &self.ground_blocks {
            assert!(
                blocks.get(block).is_some(),
                "structure {structure_id} restrictions.groundBlocks references missing block: {block}"
            );
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
