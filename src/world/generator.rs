mod biome;
mod biome_map;
mod chunk;
mod foundation;
mod generated_fluid;
mod material;
mod structure;
mod structure_debug;
mod structure_set;
mod terrain;
mod terrain_debug;

use std::sync::Arc;

use bevy::prelude::{IVec3, Resource};

use crate::content::{
    biome::BiomeRegistry,
    block::BlockRegistry,
    dimension::{DimensionDefinition, GeneratedOceanDefinition, GeneratedSurfaceFluidDefinition},
    fluid::FluidRegistry,
    structure::StructureRegistry,
    structure_set::StructureSetRegistry,
};
pub(crate) use biome::BiomeQueries;
use biome::BiomeLayout;
pub(crate) use biome_map::{BiomeMapConfig, render_biome_map};
pub(crate) use chunk::{GeneratedFluidFrontierTarget, MaterializedChunk};
use chunk::{ChunkMaterializer, ChunkRuntimeContent};
use foundation::{GenerationDimension, GenerationEntropy, GenerationSeed, GenerationSnapshot};
use generated_fluid::GeneratedFluidField;
pub(crate) use material::MaterialQueries;
use material::MaterialField;
pub(crate) use structure::StructureQueries;
use structure::{StructureAuthoring, StructureField};
pub(crate) use structure_debug::{StructureDebugConfig, validate_structure_debug};
pub(crate) use terrain::TerrainQueries;
use terrain::TerrainField;
pub(crate) use terrain_debug::{TerrainDebugConfig, render_terrain_debug};

/// Immutable entry point for deterministic generated-world queries.
///
/// `WorldGenerator` owns frozen generation inputs and composes specialized
/// capability owners. Consumers receive semantic query capabilities rather
/// than seed, entropy, authored registries, or cache/layout internals.
#[derive(Resource, Clone, Debug)]
pub(crate) struct WorldGenerator {
    snapshot: Arc<GenerationSnapshot>,
    biomes: Arc<BiomeLayout>,
    terrain: Arc<TerrainField>,
    materials: Arc<MaterialField>,
    structures: Arc<StructureField>,
    chunk_materializer: Option<Arc<ChunkMaterializer>>,
}

impl WorldGenerator {
    /// Generator used by diagnostic/query paths that do not consume authored
    /// Structures or runtime chunk encoding registries.
    pub(crate) fn new(
        seed: u64,
        dimension: &DimensionDefinition,
        biome_registry: &BiomeRegistry,
    ) -> Self {
        let snapshot = GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::from_definition(dimension),
        );
        let structures = StructureRegistry::default();
        let structure_sets = StructureSetRegistry::default();
        Self::from_snapshot_with_content(
            snapshot,
            biome_registry,
            &structures,
            &structure_sets,
            dimension.generated_ocean.as_ref(),
            &dimension.generated_surface_fluids,
            &[],
            None,
        )
    }

    /// Query/debug generator with authored Structures but no runtime chunk encoder.
    pub(crate) fn new_with_structures(
        seed: u64,
        dimension: &DimensionDefinition,
        biome_registry: &BiomeRegistry,
        structure_registry: &StructureRegistry,
        structure_set_registry: &StructureSetRegistry,
    ) -> Self {
        let snapshot = GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::from_definition(dimension),
        );
        Self::from_snapshot_with_content(
            snapshot,
            biome_registry,
            structure_registry,
            structure_set_registry,
            dimension.generated_ocean.as_ref(),
            &dimension.generated_surface_fluids,
            &dimension.generated_surface_structures,
            None,
        )
    }

    /// Runtime generator with a frozen adapter capable of materializing `VoxelChunk` values.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_runtime(
        seed: u64,
        dimension: &DimensionDefinition,
        biome_registry: &BiomeRegistry,
        block_registry: &BlockRegistry,
        fluid_registry: &FluidRegistry,
        structure_registry: &StructureRegistry,
        structure_set_registry: &StructureSetRegistry,
    ) -> Self {
        let snapshot = GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::from_definition(dimension),
        );
        Self::from_snapshot_with_content(
            snapshot,
            biome_registry,
            structure_registry,
            structure_set_registry,
            dimension.generated_ocean.as_ref(),
            &dimension.generated_surface_fluids,
            &dimension.generated_surface_structures,
            Some(ChunkRuntimeContent {
                blocks: block_registry,
                fluids: fluid_registry,
            }),
        )
    }

    fn from_snapshot(snapshot: GenerationSnapshot, biome_registry: &BiomeRegistry) -> Self {
        let structures = StructureRegistry::default();
        let structure_sets = StructureSetRegistry::default();
        Self::from_snapshot_with_content(
            snapshot,
            biome_registry,
            &structures,
            &structure_sets,
            None,
            &[],
            &[],
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_snapshot_with_content(
        snapshot: GenerationSnapshot,
        biome_registry: &BiomeRegistry,
        structure_registry: &StructureRegistry,
        structure_set_registry: &StructureSetRegistry,
        generated_ocean: Option<&GeneratedOceanDefinition>,
        generated_surface_fluids: &[GeneratedSurfaceFluidDefinition],
        generated_surface_structures: &[crate::content::dimension::GeneratedSurfaceStructureDefinition],
        chunk_runtime: Option<ChunkRuntimeContent<'_>>,
    ) -> Self {
        let biomes = Arc::new(BiomeLayout::new(&snapshot, biome_registry));
        let generated_fluids = Arc::new(GeneratedFluidField::new(
            &snapshot,
            biome_registry,
            generated_ocean,
            generated_surface_fluids,
        ));
        let terrain = Arc::new(TerrainField::new(
            &snapshot,
            biome_registry,
            Arc::clone(&biomes),
            Arc::clone(&generated_fluids),
        ));
        let materials = Arc::new(MaterialField::new(
            &snapshot,
            biome_registry,
            Arc::clone(&biomes),
            Arc::clone(&terrain),
            generated_fluids,
        ));
        let structures = Arc::new(StructureField::new(
            &snapshot,
            StructureAuthoring {
                biomes: biome_registry,
                structures: structure_registry,
                structure_sets: structure_set_registry,
                roots: generated_surface_structures,
            },
            Arc::clone(&biomes),
            Arc::clone(&terrain),
            Arc::clone(&materials),
        ));
        let chunk_materializer = chunk_runtime.map(|content| {
            Arc::new(ChunkMaterializer::new(
                &snapshot,
                Arc::clone(&materials),
                Arc::clone(&structures),
                structure_registry,
                content,
            ))
        });
        Self {
            snapshot: Arc::new(snapshot),
            biomes,
            terrain,
            materials,
            structures,
            chunk_materializer,
        }
    }

    pub(crate) fn biomes(&self) -> BiomeQueries<'_> {
        self.biomes.queries()
    }

    pub(crate) fn terrain(&self) -> TerrainQueries<'_> {
        self.terrain.queries()
    }

    pub(crate) fn materials(&self) -> MaterialQueries<'_> {
        self.materials.queries()
    }

    pub(crate) fn structures(&self) -> StructureQueries<'_> {
        self.structures.queries()
    }

    /// Materializes one 16³ runtime chunk without making the chunk a semantic owner.
    ///
    /// This capability exists only on runtime generators built by `new_runtime`.
    pub(crate) fn materialize_chunk(&self, coord: IVec3) -> MaterializedChunk {
        self.chunk_materializer
            .as_ref()
            .expect("chunk materialization requires a runtime WorldGenerator")
            .materialize(coord)
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
    use super::foundation::{GenerationDomain, GenerationPoint2, GenerationPoint3};
    use super::*;
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
            },
            "surfaceLayout": {},
            "surfaceTerrain": {},
            "surfaceLayers": [
                { "block": "asteria:test_surface", "depth": 1 },
                { "block": "asteria:test_core" }
            ]
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
        assert_eq!(
            generator.terrain().surface_at(12, -34),
            clone.terrain().surface_at(12, -34)
        );
        let surface_y = generator.terrain().surface_at(12, -34);
        assert_eq!(
            generator
                .materials()
                .solid_block_at(12, surface_y, -34)
                .map(|block| block.as_str().to_owned()),
            clone
                .materials()
                .solid_block_at(12, surface_y, -34)
                .map(|block| block.as_str().to_owned())
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
