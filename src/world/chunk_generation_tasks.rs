use bevy::prelude::*;

/// Transitional scheduler handle retained only because runtime diagnostics and
/// lifecycle reset code need a generation-work owner while the Phase-2
/// materializer is absent. The replacement scheduler will replace this type.
#[derive(Resource, Default)]
pub(crate) struct GenerationScheduler;

impl GenerationScheduler {
    pub(crate) fn pending_count(&self) -> usize {
        0
    }
}
