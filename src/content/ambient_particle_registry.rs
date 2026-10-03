use bevy::prelude::*;
use serde::Deserialize;

use super::{
    ambient_particle::AmbientParticleDefinition, biome::BiomeRegistry, dimension::DimensionRegistry,
    fluid::FluidRegistry, registry::DefinitionMap,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AmbientParticleSource {
    Dimension { id: String },
    Biome { id: String },
    FluidSurface { id: String },
}

impl AmbientParticleSource {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Dimension { id } | Self::Biome { id } | Self::FluidSurface { id } => id,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbientParticleRule {
    pub id: String,
    pub source: AmbientParticleSource,
    #[serde(flatten)]
    pub particle: AmbientParticleDefinition,
}

impl AmbientParticleRule {
    pub(crate) fn validate_references(
        &self,
        biomes: &BiomeRegistry,
        dimensions: &DimensionRegistry,
        fluids: &FluidRegistry,
    ) {
        let exists = match &self.source {
            AmbientParticleSource::Dimension { id } => dimensions.get(id).is_some(),
            AmbientParticleSource::Biome { id } => biomes.get(id).is_some(),
            AmbientParticleSource::FluidSurface { id } => fluids.id_of(id).is_some(),
        };
        assert!(
            exists,
            "ambient particle {} references missing {:?} source {}",
            self.id,
            self.source,
            self.source.id()
        );
    }
}

#[derive(Resource, Default)]
pub struct AmbientParticleRegistry {
    definitions: DefinitionMap<AmbientParticleRule>,
}

impl AmbientParticleRegistry {
    pub fn insert(&mut self, rule: AmbientParticleRule) {
        assert!(!rule.id.trim().is_empty(), "ambient particle id cannot be empty");
        assert!(
            !rule.source.id().trim().is_empty(),
            "ambient particle {} source id cannot be empty",
            rule.id
        );
        rule.particle.validate(&format!("ambient particle {}", rule.id));
        self.definitions.insert(rule.id.clone(), rule);
    }

    pub fn iter(&self) -> impl Iterator<Item = &AmbientParticleRule> {
        self.definitions.values()
    }
}
