mod foundation;

use std::sync::Arc;

use crate::{content::dimension::DimensionDefinition, world::WorldSeed};

pub(crate) use foundation::{
    GenerationDimension, GenerationDomain, GenerationSnapshot, SampleArea2d, SampleGrid2d,
};
use foundation::GenerationEntropy;

/// Immutable entry point for deterministic generated-world queries.
///
/// This object deliberately owns no runtime world, chunk residency, persistence,
/// rendering, or task state. Later generation layers compose their specialized
/// query owners behind this facade while sharing the same immutable snapshot.
#[derive(Clone, Debug)]
pub(crate) struct WorldGenerator {
    snapshot: Arc<GenerationSnapshot>,
    entropy: GenerationEntropy,
}

impl WorldGenerator {
    pub(crate) fn new(seed: WorldSeed, dimension: &DimensionDefinition) -> Self {
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

    /// Foundation-only deterministic entropy. Domain owners consume this;
    /// external gameplay/query consumers must use the specialized capability
    /// that owns the generated fact instead of reconstructing it from entropy.
    pub(super) fn entropy(&self) -> &GenerationEntropy {
        &self.entropy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::dimension::DimensionId;

    fn test_generator(seed: u64, dimension_id: &str) -> WorldGenerator {
        WorldGenerator::from_snapshot(GenerationSnapshot::new(
            WorldSeed(seed),
            GenerationDimension::new(DimensionId::from(dimension_id), 64, 1.0),
        ))
    }

    #[test]
    fn world_generator_is_immutable_send_sync_query_state() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WorldGenerator>();

        let generator = test_generator(7, "asteria:test");
        let clone = generator.clone();
        assert_eq!(generator.snapshot().seed().0, clone.snapshot().seed().0);
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
        let point = bevy::prelude::IVec2::new(-987_654, 1_234_567);

        assert_eq!(
            generator.entropy().sample_2d(domain, point),
            clone.entropy().sample_2d(domain, point)
        );
    }
}
