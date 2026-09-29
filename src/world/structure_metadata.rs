pub(crate) mod planning;

use std::{ops::Deref, sync::Arc};

use bevy::prelude::IVec2;

use crate::content::structure::StructureRotation;

use super::structure_field::StructureField;

/// One immutable materialization decision inside a resolved authored structure plan.
///
/// The value belongs to the world-domain planning contract. A cache may retain it,
/// but cache eviction must never change what the same deterministic planning inputs
/// resolve to.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedStructurePlanPiece {
    pub(crate) structure_id: String,
    pub(crate) rotation: StructureRotation,
    pub(crate) anchor: IVec2,
    pub(crate) origin_y: i32,
    pub(crate) primary_placement_piece: bool,
}

/// Immutable resolved structure intent for one authored placement occurrence.
///
/// This is deliberately distinct from both cache ownership and voxel
/// materialization. It can describe pieces that cross unloaded chunk boundaries;
/// generation later applies only the pieces intersecting the chunk being built.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedStructurePlan {
    pub(crate) pieces: Vec<ResolvedStructurePlanPiece>,
    pub(crate) minimum: IVec2,
    pub(crate) maximum: IVec2,
    pub(crate) minimum_y: i32,
    pub(crate) maximum_y: i32,
}

/// One accepted authored placement after deterministic conflict resolution.
///
/// Priority/reservation/conflict-group inputs are intentionally absent here: they
/// belong to planning. Materialization only needs placement identity plus the
/// immutable plan that survived conflict resolution.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedStructurePlacement {
    pub(crate) placement_id: String,
    pub(crate) placement_anchor: IVec2,
    pub(crate) placement_y: i32,
    pub(crate) plan: ResolvedStructurePlan,
}

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
