use std::{ops::Deref, sync::Arc};

use bevy::prelude::IVec2;

use super::structure_field::StructureField;

/// Deterministic authored structure intent for a world.
///
/// This owner is deliberately separate from disposable generation caches.
/// Clearing or replacing performance caches must never replace the world-level
/// decision about where authored structures exist.
#[derive(Clone)]
pub(crate) struct StructureMetadata {
    field: Arc<StructureField>,
}

impl StructureMetadata {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            field: Arc::new(StructureField::empty(seed)),
        }
    }

    pub(crate) fn with_field(self, field: StructureField) -> Self {
        Self {
            field: Arc::new(field),
        }
    }

    pub(crate) fn field(&self) -> &StructureField {
        &self.field
    }

    pub(crate) fn reference_bounds(&self, reference: &str) -> Option<(IVec2, IVec2)> {
        self.field.reference_bounds(reference)
    }

    #[cfg(test)]
    pub(crate) fn shares_field_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.field, &other.field)
    }
}

impl Deref for StructureMetadata {
    type Target = StructureField;

    fn deref(&self) -> &Self::Target {
        self.field()
    }
}
