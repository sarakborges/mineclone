//! Sparse storage and edit primitives for the Chisel's four cut sizes.
//!
//! This module is intentionally not wired into the voxel pipeline yet: a microblock
//! must not be player-editable until meshing, targeting, collision and save/load
//! all consume the same data. Ordinary, unmodified blocks remain ordinary cells.

use super::cell::VoxelCell;

/// Maximum subdivisions along each edge. A fully detailed block has 8^3 cells,
/// not 64^3: the player's "64" refers to the 8x8 grid on one face.
pub(crate) const MICROBLOCK_EDGE: usize = 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MicroblockResolution {
    #[default]
    Whole,
    Half,
    Quarter,
    Eighth,
}

impl MicroblockResolution {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Whole => Self::Half,
            Self::Half => Self::Quarter,
            Self::Quarter => Self::Eighth,
            Self::Eighth => Self::Whole,
        }
    }

    /// Width of one cut in units of the finest 8x8x8 grid.
    const fn cell_width(self) -> usize {
        match self {
            Self::Whole => 8,
            Self::Half => 4,
            Self::Quarter => 2,
            Self::Eighth => 1,
        }
    }
}

/// A uniform region uses one cell; only modified regions allocate children.
/// Eight children partition a parent into 2x2x2 octants. Uniform children
/// automatically collapse back into their parent after edits.
#[derive(Clone, Debug)]
pub(crate) enum MicroblockStorage {
    Uniform(Option<VoxelCell>),
    Split(Box<[MicroblockStorage; 8]>),
}

impl MicroblockStorage {
    pub(crate) const fn uniform(cell: Option<VoxelCell>) -> Self {
        Self::Uniform(cell)
    }

    /// `None` means out of bounds; `Some(None)` means empty microcell.
    pub(crate) fn cell_at(&self, position: [usize; 3]) -> Option<Option<VoxelCell>> {
        if !position.iter().all(|&coordinate| coordinate < MICROBLOCK_EDGE) {
            return None;
        }
        Some(self.cell_in_region(position, MICROBLOCK_EDGE))
    }

    /// Returns true only for real changes. Position is in the fine 8x8x8 grid,
    /// and edits snap down to the selected cut size on all three axes.
    pub(crate) fn edit(
        &mut self,
        position: [usize; 3],
        resolution: MicroblockResolution,
        replacement: Option<VoxelCell>,
    ) -> bool {
        if !position.iter().all(|&coordinate| coordinate < MICROBLOCK_EDGE) {
            return false;
        }

        let width = resolution.cell_width();
        let start = position.map(|coordinate| coordinate / width * width);
        self.edit_region(start, MICROBLOCK_EDGE, width, replacement)
    }

    /// Only a wholly uniform region can be represented as an ordinary cell.
    pub(crate) fn uniform_cell(&self) -> Option<Option<VoxelCell>> {
        match self {
            Self::Uniform(cell) => Some(*cell),
            Self::Split(_) => None,
        }
    }

    fn cell_in_region(&self, position: [usize; 3], size: usize) -> Option<VoxelCell> {
        match self {
            Self::Uniform(cell) => *cell,
            Self::Split(children) => {
                let half = size / 2;
                let index = octant_index(position, half);
                let local = position.map(|coordinate| coordinate % half);
                children[index].cell_in_region(local, half)
            }
        }
    }

    fn edit_region(
        &mut self,
        position: [usize; 3],
        size: usize,
        width: usize,
        replacement: Option<VoxelCell>,
    ) -> bool {
        if let Self::Uniform(cell) = self {
            if *cell == replacement {
                return false;
            }
        }
        if size == width {
            *self = Self::Uniform(replacement);
            return true;
        }

        if let Self::Uniform(cell) = self {
            let original = *cell;
            *self = Self::Split(Box::new(std::array::from_fn(|_| Self::Uniform(original))));
        }

        let half = size / 2;
        let index = octant_index(position, half);
        let local = position.map(|coordinate| coordinate % half);
        let Self::Split(children) = self else {
            unreachable!("non-leaf edit must have subdivided its parent")
        };
        let changed = children[index].edit_region(local, half, width, replacement);
        if changed {
            self.compact();
        }
        changed
    }

    fn compact(&mut self) {
        if let Self::Split(children) = self {
            if let Some(first) = children[0].uniform_cell() {
                if children.iter().all(|child| child.uniform_cell() == Some(first)) {
                    *self = Self::Uniform(first);
                }
            }
        }
    }
}

const fn octant_index([x, y, z]: [usize; 3], half: usize) -> usize {
    usize::from(x >= half) | (usize::from(y >= half) << 1) | (usize::from(z >= half) << 2)
}
