//! Persisted Chisel geometry. The original macro cell supplies the material,
//! texture rotation, orientation and visual properties; its occupancy mask
//! preserves the carved shape across saves and archived chunks.
//!
//! A leading `t` is retained for compatibility with legacy session-created
//! parent blocks. New Chisel placement never creates parents in empty space.

use bevy::prelude::*;

use super::{cell::VoxelCell, read::VoxelRead};

pub(crate) const MICROBLOCK_EDGE: i32 = 8;
pub(crate) const CHISEL_MASK_PROPERTY: &str = "asteria:chisel_mask";
const LAYERS: usize = MICROBLOCK_EDGE as usize;
const ENCODED_LENGTH: usize = LAYERS * 16;
const TRANSIENT_PREFIX: char = 't';

/// Full-block interactions belong to ordinary block tools, not the Chisel.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ChiselResolution {
    #[default]
    Thick,
    Thin,
    ExtraThin,
}

impl ChiselResolution {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Thick => Self::Thin,
            Self::Thin => Self::ExtraThin,
            Self::ExtraThin => Self::Thick,
        }
    }

    pub(crate) const fn cell_width(self) -> usize {
        match self {
            Self::Thick => 4,
            Self::Thin => 2,
            Self::ExtraThin => 1,
        }
    }
}

/// Eight Z layers, each holding an 8x8 XY occupancy bitmap. Ordinary blocks
/// have no mask; sculpted blocks need only 64 bytes before string encoding.
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

    pub(crate) fn is_transient_parent(cell: VoxelCell) -> bool {
        cell.secondary_property(CHISEL_MASK_PROPERTY)
            .is_some_and(|encoded| encoded.starts_with(TRANSIENT_PREFIX))
    }

    /// A new piece must restore a previously carved portion of this very
    /// macroblock. Legacy parents placed in air are not valid restore targets.
    pub(crate) fn can_restore(cell: VoxelCell) -> bool {
        Self::is_modified(cell)
            && !Self::is_transient_parent(cell)
            && Self::from_cell(cell) != Self::FULL
    }

    /// Reject corrupted disk masks before interning properties or exposing a
    /// partially loaded world. Existing saves without a mask remain valid.
    pub(crate) fn valid_saved(encoded: &str) -> bool {
        let encoded = encoded.strip_prefix(TRANSIENT_PREFIX).unwrap_or(encoded);
        encoded.len() == ENCODED_LENGTH
            && encoded.bytes().all(|byte| byte.is_ascii_hexdigit())
            && Self::decode(encoded).is_some()
    }

    pub(crate) fn from_cell(cell: VoxelCell) -> Self {
        let Some(encoded) = cell.secondary_property(CHISEL_MASK_PROPERTY) else {
            return Self::FULL;
        };
        let encoded = encoded.strip_prefix(TRANSIENT_PREFIX).unwrap_or(encoded);
        Self::decode(encoded).unwrap_or(Self::FULL)
    }

    pub(crate) fn contains(self, [x, y, z]: [usize; 3]) -> bool {
        if x >= LAYERS || y >= LAYERS || z >= LAYERS {
            return false;
        }
        self.layers[z] & (1_u64 << (x + y * LAYERS)) != 0
    }

    /// Approximate macro-cell light attenuation from its occupied microcells.
    /// Lighting is stored at macro resolution, so a carved cell cannot expose
    /// exact directional holes; occupancy-weighted attenuation preserves the
    /// useful distinction between full, partial, and empty geometry.
    pub(crate) fn light_dampening(self, full_dampening: u8) -> u8 {
        const MICROBLOCK_VOLUME: usize = LAYERS * LAYERS * LAYERS;
        let occupied = self
            .layers
            .iter()
            .map(|layer| layer.count_ones() as usize)
            .sum::<usize>();

        ((usize::from(full_dampening) * occupied)
            .saturating_add(MICROBLOCK_VOLUME - 1)
            / MICROBLOCK_VOLUME) as u8
    }

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

    pub(crate) fn apply_to_cell(self, cell: VoxelCell, transient: bool) -> VoxelCell {
        if self == Self::FULL && !transient {
            return cell.without_secondary_property(CHISEL_MASK_PROPERTY);
        }
        let mut encoded = String::with_capacity(ENCODED_LENGTH + usize::from(transient));
        if transient {
            encoded.push(TRANSIENT_PREFIX);
        }
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

/// Euclidean coordinates preserve adjacency at negative X/Z and chunk borders.
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
    let cell = world.cell_at(parent_voxel(fine))?;
    MicroblockMask::from_cell(cell)
        .contains(local_cell(fine))
        .then_some(cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_dampening_scales_with_occupied_microcells() {
        assert_eq!(MicroblockMask::EMPTY.light_dampening(15), 0);
        assert_eq!(MicroblockMask::FULL.light_dampening(15), 15);

        let mut half = MicroblockMask::EMPTY;
        for layer in 0..LAYERS / 2 {
            half.layers[layer] = u64::MAX;
        }
        assert_eq!(half.light_dampening(15), 8);
    }

    #[test]
    fn zero_base_dampening_stays_zero_for_sculpted_geometry() {
        assert_eq!(MicroblockMask::FULL.light_dampening(0), 0);
    }
}
