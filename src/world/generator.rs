mod biome;
mod biome_map;
mod foundation;

use std::sync::Arc;

use bevy::prelude::Resource;

use crate::content::{biome::BiomeRegistry, dimension::DimensionDefinition};
pub(crate) use biome::BiomeQueries;
use biome::BiomeLayout;
pub(crate) use biome_map::{BiomeMapConfig, render_biome_map};
use foundation::{GenerationDimension, GenerationEntropy, GenerationSeed, GenerationSnapshot};

/// Immutable entry point for deterministic generated-world queries.
///
/// `WorldGenerator` owns frozen generation inputs and composes specialized
/// capability owners. Consumers receive semantic query capabilities rather
/// than seed, entropy, authored registries, or cache/layout internals.
#[derive(Resource, Clone, Debug)]
pub(crate) struct WorldGenerator {
    snapshot: Arc<GenerationSnapshot>,
    biomes: Arc<BiomeLayout>,
}

impl WorldGenerator {
    pub(crate) fn new(
        seed: u64,
        dimension: &DimensionDefinition,
        biome_registry: &BiomeRegistry,
    ) -> Self {
        let snapshot = GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::from_definition(dimension),
        );
        Self::from_snapshot(snapshot, biome_registry)
    }

    fn from_snapshot(snapshot: GenerationSnapshot, biome_registry: &BiomeRegistry) -> Self {
        let biomes = BiomeLayout::new(&snapshot, biome_registry);
        Self {
            snapshot: Arc::new(snapshot),
            biomes: Arc::new(biomes),
        }
    }

    pub(crate) fn biomes(&self) -> BiomeQueries<'_> {
        self.biomes.queries()
    }

    /// Internal-only generated-world read boundary.
    ///
    /// Semantic owners use this to read immutable definitions and deterministic
    /// entropy without depending on runtime world state or chunk materialization.
    fn read_context(&self) -> GenerationReadContext<'_> {
        GenerationReadContext {
            snapshot: &self.snapshot,
            entropy: GenerationEntropy::new(&self.snapshot),
        }
    }
}

#[derive(Clone, Copy)]
struct GenerationReadContext<'a> {
    snapshot: &'a GenerationSnapshot,
    entropy: GenerationEntropy,
}

impl GenerationReadContext<'_> {
    fn snapshot(&self) -> &GenerationSnapshot {
        self.snapshot
    }

    fn entropy(self) -> GenerationEntropy {
        self.entropy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::foundation::{GenerationDomain, GenerationPoint2, GenerationPoint3};
    use crate::content::biome::BiomeDefinition;

    fn test_generator(seed: u64, dimension_id: &str) -> WorldGenerator {
        let snapshot = GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::new(dimension_id, 64, 1.0),
        );
        let registry = test_biome_registry(dimension_id);
        WorldGenerator::from_snapshot(snapshot, &registry)
    }

    fn test_biome_registry(dimension_id: &str) -> BiomeRegistry {
        let local_dimension = dimension_id
            .split_once(':')
            .map(|(_, local)| local)
            .expect("test dimension id must be namespaced");
        let definition: BiomeDefinition = serde_json::from_value(serde_json::json!({
            "id": format!("asteria:{local_dimension}/base"),
            "name": {
                "english": "Base",
                "portuguese_brazil": "Base",
                "spanish": "Base"
            }
        }))
        .expect("test biome definition must deserialize");
        let mut registry = BiomeRegistry::default();
        registry.insert(definition);
        registry
    }

    fn test_dimension_definition(
        id: &str,
        sea_level: i32,
        gravity_strength: f32,
    ) -> DimensionDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": {
                "english": "Test Dimension",
                "portuguese_brazil": "Test Dimension",
                "spanish": "Test Dimension"
            },
            "dayNightCycle": "asteria:test/cycle",
            "sky": "asteria:test/sky",
            "seaLevel": sea_level,
            "gravityStrength": gravity_strength
        }))
        .expect("test dimension definition must deserialize")
    }

    #[test]
    fn world_generator_is_immutable_send_sync_query_state() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WorldGenerator>();

        let generator = test_generator(7, "asteria:test");
        let clone = generator.clone();
        assert_eq!(
            generator.read_context().snapshot().seed(),
            clone.read_context().snapshot().seed()
        );
        assert_eq!(
            generator.read_context().snapshot().dimension().id(),
            clone.read_context().snapshot().dimension().id()
        );
        assert_eq!(
            generator.biomes().surface_biome_at(12, -34),
            clone.biomes().surface_biome_at(12, -34)
        );
    }

    #[test]
    fn cloned_generators_share_identical_foundation_results() {
        let generator = test_generator(9_001, "asteria:test");
        let clone = generator.clone();
        let domain = GenerationDomain::named("phase2-clone-equivalence");
        let point_2d = GenerationPoint2::new(-987_654, 1_234_567);
        let point_3d = GenerationPoint3::new(451_002, -77, -901_221);

        assert_eq!(
            generator.read_context().entropy().sample_2d(domain, point_2d),
            clone.read_context().entropy().sample_2d(domain, point_2d)
        );
        assert_eq!(
            generator.read_context().entropy().sample_3d(domain, point_3d),
            clone.read_context().entropy().sample_3d(domain, point_3d)
        );
    }

    #[test]
    fn generation_snapshot_freezes_dimension_inputs() {
        let generator = test_generator(123, "asteria:frozen");
        let context = generator.read_context();
        let snapshot = context.snapshot();

        assert_eq!(snapshot.dimension().id(), "asteria:frozen");
        assert_eq!(snapshot.dimension().sea_level(), 64);
        assert_eq!(snapshot.dimension().gravity_strength(), 1.0);
    }

    #[test]
    fn world_generator_copies_authored_dimension_inputs() {
        let mut definition = test_dimension_definition("asteria:authored", 72, 0.85);
        let registry = test_biome_registry("asteria:authored");
        let generator = WorldGenerator::new(991, &definition, &registry);

        definition.id = "asteria:mutated".to_owned();
        definition.sea_level = -10;
        definition.gravity_strength = 3.0;

        let context = generator.read_context();
        let snapshot = context.snapshot();
        assert_eq!(snapshot.seed(), GenerationSeed::new(991));
        assert_eq!(snapshot.dimension().id(), "asteria:authored");
        assert_eq!(snapshot.dimension().sea_level(), 72);
        assert_eq!(snapshot.dimension().gravity_strength(), 0.85);
    }
}
