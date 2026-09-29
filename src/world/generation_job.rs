use std::sync::Arc;

use crate::voxel::{chunk::VoxelChunk, coordinates::ChunkCoord};

use super::{generation::generate_chunk, generation_snapshot::GenerationSnapshot};

/// Immutable, scheduler-independent unit of chunk generation work.
///
/// Scheduling policy owns when and where this job executes. The job owns only
/// the deterministic calculation inputs required to produce authoritative
/// chunk data; it has no ECS, rendering, UI, or publication side effects.
pub(crate) struct ChunkGenerationJob {
    coord: ChunkCoord,
    snapshot: Arc<GenerationSnapshot>,
}

impl ChunkGenerationJob {
    pub(crate) fn new(coord: ChunkCoord, snapshot: Arc<GenerationSnapshot>) -> Self {
        Self { coord, snapshot }
    }

    pub(crate) fn run(self) -> VoxelChunk {
        let context = self.snapshot.context();
        generate_chunk(self.coord.as_ivec3(), &context)
    }
}
