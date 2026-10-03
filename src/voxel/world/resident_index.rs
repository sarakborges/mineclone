use std::collections::BTreeSet;

use bevy::{platform::collections::HashMap, prelude::*};

use super::super::chunk::CHUNK_SIZE;

#[derive(Clone, Default)]
pub(super) struct LoadedChunkColumnIndex {
    columns: HashMap<IVec2, BTreeSet<i32>>,
}

impl LoadedChunkColumnIndex {
    pub(super) fn insert(&mut self, coord: IVec3) {
        self.columns.entry(coord.xz()).or_default().insert(coord.y);
    }

    pub(super) fn remove(&mut self, coord: IVec3) {
        let horizontal = coord.xz();
        let Some(ys) = self.columns.get_mut(&horizontal) else {
            return;
        };

        ys.remove(&coord.y);
        if ys.is_empty() {
            self.columns.remove(&horizontal);
        }
    }

    pub(super) fn coords_below(&self, coord: IVec3) -> impl Iterator<Item = IVec3> + '_ {
        self.columns
            .get(&coord.xz())
            .into_iter()
            .flat_map(move |ys| {
                ys.range(..coord.y)
                    .copied()
                    .map(move |y| IVec3::new(coord.x, y, coord.z))
            })
    }

    pub(super) fn highest_world_y_in_column(&self, horizontal_chunk: IVec2) -> Option<i32> {
        let highest_chunk_y = self.columns.get(&horizontal_chunk)?.last().copied()?;
        Some((highest_chunk_y + 1) * CHUNK_SIZE as i32 - 1)
    }
}
