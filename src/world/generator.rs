mod foundation;

use std::sync::Arc;

use crate::content::dimension::DimensionDefinition;

pub(crate) use foundation::{
    GenerationDimension, GenerationDomain, GenerationPoint2, GenerationPoint3, GenerationSeed,
    GenerationSnapshot, SampleArea2d, SampleGrid2d,
};
use foundation::GenerationEntropy;

/// Immutable entry point for deterministic generated-world queries.
///
/// The rebuild owns this state completely. It deliberately does not reuse the
/// legacy world seed, dimension runtime identity, coordinate wrappers, hashing,
/// chunk residency, persistence, rendering, or task state.
#[derive(Clone, Debug)]
pub(crate) struct WorldGenerator {
    snapshot: Arc<GenerationSnapshot>,
    entropy: GenerationEntropy,
}

impl WorldGenerator {
    pub(crate) fn new(seed: GenerationSeed, dimension: &DimensionDefinition) -> Self {
        Self::from_snapshot(GenerationSnapshot::new(
            seed,
            GenerationDimension::from_definition(dimension),
        ))
    }

    fn from_snapshot(snapshot: GenerationSnapshot) -> Self {
        let snapshot = Arc::new(snapshot);
        let entropy = GenerationEntropy::new(&snapshot);
        Self { snapshot, entropy }
    }

    pub(crate) fn snapshot(&self) -> &GenerationSnapshot {
        &self.snapshot
    }

    /// Foundation-only deterministic entropy. Later biome/terrain/structure
    /// owners consume this internally; gameplay consumers never do.
    pub(super) fn entropy(&self) -> &GenerationEntropy {
        &self.entropy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_generator(seed: u64, dimension_id: &str) -> WorldGenerator {
        WorldGenerator::from_snapshot(GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::new(dimension_id, 64, 1.0),
        ))
    }

    #[test]
    fn world_generator_is_immutable_send_sync_query_state() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WorldGenerator>();

        let generator = test_generator(7, "asteria:test");
        let clone = generator.clone();
        assert_eq!(generator.snapshot().seed(), clone.snapshot().seed());
        assert_eq!(
            generator.snapshot().dimension().id(),
            clone.snapshot().dimension().id()
        );
    }

    #[test]
    fn cloned_generators_share_identical_foundation_results() {
        let generator = test_generator(9_001, "asteria:test");
        let clone = generator.clone();
        let domain = GenerationDomain::named("phase2-test");
        let point = GenerationPoint2::new(-987_654, 1_234_567);

        assert_eq!(
            generator.entropy().sample_2d(domain, point),
            clone.entropy().sample_2d(domain, point)
        );
    }
}
