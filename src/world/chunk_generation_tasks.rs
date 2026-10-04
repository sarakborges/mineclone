use std::fmt;

use bevy::prelude::*;

/// Placeholder diagnostic retained for runtime diagnostics while the chunk
/// materializer is absent. The replacement scheduler is introduced only after
/// the Phase-2 generation/query foundation exists.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GenerationPipelineDiagnostic;

impl fmt::Display for GenerationPipelineDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("worldgen_rebuild_pending")
    }
}

#[derive(Resource, Default)]
pub(crate) struct GenerationScheduler;

impl GenerationScheduler {
    pub(crate) fn pending_count(&self) -> usize {
        0
    }

    pub(crate) fn diagnostics(&self) -> GenerationPipelineDiagnostic {
        GenerationPipelineDiagnostic
    }
}
