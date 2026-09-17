//! Storage and edit primitives for the Chisel's two resolutions.
//!
//! This module is intentionally not wired into the voxel pipeline yet: a microblock
//! must not be player-editable until meshing, targeting, collision and save/load
//! all consume the same data. Ordinary, unmodified blocks remain ordinary cells.

use super::cell::VoxelCell;

pub(crate) const MICROBLOCK_EDGE: usize = 4;
pub(crate) const MICROBLOCK_COUNT: usize = MICROBLOCK_EDGE * MICROBLOCK_EDGE * MICROBLOCK_EDGE;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MicroblockResolution {
    #[default]
    Eighth,
    SixtyFourth,
}

impl MicroblockResolution {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Eighth => Self::SixtyFourth,
            Self::SixtyFourth => Self::Eighth,
        }
    }

    const fn cell_width(self) -> usize {
        match self {
            Self::Eighth => 2,
            Self::SixtyFourth => 1,
        }
    }
}

/// A 4 x 4 x 4 logical grid allocated only after a change actually needs it.
/// A completely empty or uniform result is represented without an array.
#[derive(Clone, Debug)]
pub(crate) enum MicroblockStorage {
    Uniform(Option<VoxelCell>),
    Detailed(Box<[Option<VoxelCell>; MICROBLOCK_COUNT]>),
}

impl MicroblockStorage {
    pub(crate) const fn uniform(cell: Option<VoxelCell>) -> Self {
        Self::Uniform(cell)
    }

    pub(crate) fn cell_at(&self, position: [usize; 3]) -> Option<Option<VoxelCell>> {
        let index = cell_index(position)?;
        Some(match self {
            Self::Uniform(cell) => *cell,
            Self::Detailed(cells) => cells[index],
        })
    }

    /// Returns `true` only when one or more cells change. Coordinates are in
    /// the shared fine grid (0..4 on each axis); an eighth affects a 2^3 cube.
    pub(crate) fn edit(
        &mut self,
        position: [usize; 3],
        resolution: MicroblockResolution,
        replacement: Option<VoxelCell>,
    ) -> bool {
        if cell_index(position).is_none() {
            return false;
        }

        let width = resolution.cell_width();
        let start = position.map(|coordinate| coordinate / width * width);
        let mut changed = false;

        for z in start[2]..start[2] + width {
            for y in start[1]..start[1] + width {
                for x in start[0]..start[0] + width {
                    // Every index in the snapped region is within the 4^3 grid.
                    let index = x + MICROBLOCK_EDGE * (y + MICROBLOCK_EDGE * z);
                    let current = match self {
                        Self::Uniform(cell) => *cell,
                        Self::Detailed(cells) => cells[index],
                    };
                    if current == replacement {
                        continue;
                    }

                    if let Self::Uniform(cell) = self {
                        *self = Self::Detailed(Box::new([*cell; MICROBLOCK_COUNT]));
                    }
                    if let Self::Detailed(cells) = self {
                        cells[index] = replacement;
                    }
                    changed = true;
                }
            }
        }

        if changed {
            self.compact();
        }
        changed
    }

    pub(crate) fn uniform_cell(&self) -> Option<Option<VoxelCell>> {
        match self {
            Self::Uniform(cell) => Some(*cell),
            Self::Detailed(_) => None,
        }
    }

    fn compact(&mut self) {
        if let Self::Detailed(cells) = self {
            let first = cells[0];
            if cells.iter().all(|cell| *cell == first) {
                *self = Self::Uniform(first);
            }
        }
    }
}

const fn cell_index([x, y, z]: [usize; 3]) -> Option<usize> {
    if x < MICROBLOCK_EDGE && y < MICROBLOCK_EDGE && z < MICROBLOCK_EDGE {
        Some(x + MICROBLOCK_EDGE * (y + MICROBLOCK_EDGE * z))
    } else {
        None
    }
}
