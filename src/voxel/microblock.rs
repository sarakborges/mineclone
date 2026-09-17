//! Session-only Chisel geometry. The original macro cell is always the sole
//! source of the material, texture rotation, orientation and visual properties.
//!
//! A modified cell carries one private, fixed-size occupancy mask. The mask is
//! deliberately excluded from disk serialization by SecondaryProperties::iter,
//! while the in-memory chunk archive retains it across streaming unloads.

use bevy::prelude::*;

use super::{cell::VoxelCell, read::VoxelRead};

pub(crate) const MICROBLOCK_EDGE: i32 = 8;
pub(crate) const CHISEL_MASK_PROPERTY: &str = "asteria:chisel_mask";
const LAYERS: usize = MICROBLOCK_EDGE as usize;
const ENCODED_LENGTH: usize = LAYERS * 16;

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ChiselResolution {
    #[default]
    Full,
    Thick,
    Thin,
    ExtraThin,
}

impl ChiselResolution {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Full => Self::Thick,
            Self::Thick => Self::Thin,
            Self::Thin => Self::ExtraThin,
            Self::ExtraThin => Self::Full,
        }
    }

    pub(crate) const fn cell_width(self) -> usize {
        match self {
            Self::Full => 8,
            Self::Thick => 4,
            Self::Thin => 2,
            Self::ExtraThin => 1,
        }
    }
}

/// Each of the eight Z layers stores an 8x8 XY occupancy bitmap. An ordinary
/// block needs no mask at all; a sculpted block needs only 64 occupancy bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MicroblockMask {
    layers: [u64; LAYERS],
}

impl MicroblockMask {
    pub(crate) const EMPTY: Self = Self { layers: [0; LAYERS] };
    pub(crate) const FULL: Self = Self {
        layers: [u64::MAX; LAYERS],
    };

    pub(crate) fn is_modified(cell: VoxelCell) -> bool {
        cell.secondary_property(CHISEL_MASK_PROPERTY).is_some()
    }

    pub(crate) fn from_cell(cell: VoxelCell) -> Self {
        let Some(encoded) = cell.secondary_property(CHISEL_MASK_PROPERTY) else {
            return Self::FULL;
        };
        let Some(mask) = Self::decode(encoded) else {
            // Malformed internal data cannot trigger out-of-bounds geometry.
            return Self::FULL;
        };
        mask
    }

    pub(crate) fn contains(self, [x, y, z]: [usize; 3]) -> bool {
        if x >= LAYERS || y >= LAYERS || z >= LAYERS {
            return false;
        }
        self.layers[z] & (1_u64 << (x + y * LAYERS)) != 0
    }

    /// The selected cut replaces one aligned cube, never an individual
    /// arbitrary microcell at a coarser resolution.
    pub(crate) fn edit(
        &mut self,
        position: [usize; 3],
        resolution: ChiselResolution,
        occupied: bool,
    ) -> bool {
        if position.iter().any(|&axis| axis >= LAYERS) {
            return false;
        }
        let width = resolution.cell_width();
        let origin = position.map(|axis| axis / width * width);
        let mut changed = false;
        for z in origin[2]..origin[2] + width {
            for y in origin[1]..origin[1] + width {
                for x in origin[0]..origin[0] + width {
                    let bit = 1_u64 << (x + y * LAYERS);
                    let previous = self.layers[z];
                    if occupied {
                        self.layers[z] |= bit;
                    } else {
                        self.layers[z] &= !bit;
                    }
                    changed |= previous != self.layers[z];
                }
            }
        }
        changed
    }

    pub(crate) fn apply_to_cell(self, cell: VoxelCell) -> VoxelCell {
        if self == Self::FULL {
            return cell.without_secondary_property(CHISEL_MASK_PROPERTY);
        }
        let mut encoded = String::with_capacity(ENCODED_LENGTH);
        use std::fmt::Write as _;
        for layer in self.layers {
            write!(&mut encoded, "{layer:016x}").expect("writing to a String cannot fail");
        }
        cell.with_secondary_property(CHISEL_MASK_PROPERTY, &encoded)
    }

    pub(crate) fn has_room(cell: VoxelCell) -> bool {
        Self::is_modified(cell) || cell.secondary_properties().iter().count() < 8
    }

    fn decode(encoded: &str) -> Option<Self> {
        if encoded.len() != ENCODED_LENGTH {
            return None;
        }
        let mut layers = [0; LAYERS];
        for (index, layer) in layers.iter_mut().enumerate() {
            *layer = u64::from_str_radix(encoded.get(index * 16..(index + 1) * 16)?, 16).ok()?;
        }
        Some(Self { layers })
    }
}

/// `div_euclid` and `rem_euclid` keep neighboring cells correct even at
/// negative world coordinates and across chunk boundaries.
pub(crate) fn parent_voxel(fine: IVec3) -> IVec3 {
    IVec3::new(
        fine.x.div_euclid(MICROBLOCK_EDGE),
        fine.y.div_euclid(MICROBLOCK_EDGE),
        fine.z.div_euclid(MICROBLOCK_EDGE),
    )
}

pub(crate) fn local_cell(fine: IVec3) -> [usize; 3] {
    [
        fine.x.rem_euclid(MICROBLOCK_EDGE) as usize,
        fine.y.rem_euclid(MICROBLOCK_EDGE) as usize,
        fine.z.rem_euclid(MICROBLOCK_EDGE) as usize,
    ]
}

pub(crate) fn occupied_cell<W: VoxelRead + ?Sized>(world: &W, fine: IVec3) -> Option<VoxelCell> {
    let parent = parent_voxel(fine);
    let cell = world.cell_at(parent)?;
    MicroblockMask::from_cell(cell)
        .contains(local_cell(fine))
        .then_some(cell)
}
