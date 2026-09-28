use bevy::{platform::collections::{HashMap, HashSet}, prelude::IVec3};

/// Tracks render meshes evicted only because the mesh-memory high watermark was
/// exceeded. Logical residency is unaffected; entries are eligible for
/// presentation recovery when pressure drops.
#[derive(Default)]
pub(super) struct MeshPressureState {
    evicted: HashMap<IVec3, usize>,
}

impl MeshPressureState {
    pub(super) fn suppress(&mut self, coord: IVec3, bytes: usize) {
        self.evicted.insert(coord, bytes);
    }

    pub(super) fn recover(&mut self, coord: IVec3) -> bool {
        self.evicted.remove(&coord).is_some()
    }

    pub(super) fn coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.evicted.keys().copied()
    }

    pub(super) fn bytes(&self, coord: IVec3) -> Option<usize> {
        self.evicted.get(&coord).copied()
    }

    pub(super) fn contains(&self, coord: IVec3) -> bool {
        self.evicted.contains_key(&coord)
    }

    pub(super) fn len(&self) -> usize {
        self.evicted.len()
    }

    pub(super) fn retain_for_selection(
        &mut self,
        desired: &HashSet<IVec3>,
        mut keep: impl FnMut(IVec3) -> bool,
    ) {
        self.evicted
            .retain(|coord, _| desired.contains(coord) && keep(*coord));
    }
}
