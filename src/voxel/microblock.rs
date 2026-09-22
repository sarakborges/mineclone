//! Persisted Chisel geometry. The original macro cell supplies the material,
//! texture rotation, orientation and visual properties; its occupancy mask
//! preserves the carved shape across saves and archived chunks.
//!
//! A leading `t` is retained for compatibility with legacy session-created
//! parent blocks. New Chisel placement never creates parents in empty space.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use bevy::prelude::*;

use super::{
    cell::VoxelCell,
    read::VoxelRead,
};

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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct MicroblockMask {
    layers: [u64; LAYERS],
}

impl MicroblockMask {
    pub(crate) const EMPTY: Self = Self { layers: [0; LAYERS] };
    pub(crate) const FULL: Self = Self {
        layers: [u64::MAX; LAYERS],
    };

    pub(crate) fn is_modified(cell: VoxelCell) -> bool {
        cell.microblock_layers().is_some()
    }

    pub(crate) fn is_transient_parent(cell: VoxelCell) -> bool {
        cell.microblock_layers().is_some() && cell.microblock_transient()
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
        Self::decode_saved(encoded).is_some()
    }

    pub(crate) fn from_cell(cell: VoxelCell) -> Self {
        cell.microblock_layers()
            .map(|layers| Self { layers: *layers })
            .unwrap_or(Self::FULL)
    }

    pub(crate) fn encoded_for_save(cell: VoxelCell) -> Option<String> {
        let mask = Self::from_cell(cell);
        Self::is_modified(cell).then(|| mask.encode(cell.microblock_transient()))
    }

    pub(crate) fn apply_saved(cell: VoxelCell, encoded: &str) -> Option<VoxelCell> {
        let (mask, transient) = Self::decode_saved(encoded)?;
        Some(cell.with_microblock_mask(Some(intern_mask(mask)), transient))
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
    pub(crate) fn occupied_count(self) -> usize {
        self.layers
            .iter()
            .map(|layer| layer.count_ones() as usize)
            .sum()
    }

    #[cfg(test)]
    pub(crate) fn occupied_fraction(self) -> f32 {
        self.occupied_count() as f32 / (LAYERS * LAYERS * LAYERS) as f32
    }

    pub(crate) fn light_dampening(self, full_dampening: u8) -> u8 {
        const MICROBLOCK_VOLUME: usize = LAYERS * LAYERS * LAYERS;
        ((usize::from(full_dampening) * self.occupied_count())
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
            return cell.with_microblock_mask(None, false);
        }
        cell.with_microblock_mask(Some(intern_mask(self)), transient)
    }

    pub(crate) fn has_room(cell: VoxelCell) -> bool {
        Self::is_modified(cell) || cell.secondary_properties().len() < 8
    }

    fn encode(self, transient: bool) -> String {
        let mut encoded = String::with_capacity(ENCODED_LENGTH + usize::from(transient));
        if transient {
            encoded.push(TRANSIENT_PREFIX);
        }
        use std::fmt::Write as _;
        for layer in self.layers {
            write!(&mut encoded, "{layer:016x}").expect("writing to a String cannot fail");
        }
        encoded
    }

    fn decode_saved(encoded: &str) -> Option<(Self, bool)> {
        let transient = encoded.starts_with(TRANSIENT_PREFIX);
        let encoded = encoded.strip_prefix(TRANSIENT_PREFIX).unwrap_or(encoded);
        if encoded.len() != ENCODED_LENGTH || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut layers = [0; LAYERS];
        for (index, layer) in layers.iter_mut().enumerate() {
            *layer = u64::from_str_radix(encoded.get(index * 16..(index + 1) * 16)?, 16).ok()?;
        }
        Some((Self { layers }, transient))
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

#[derive(Default)]
struct MicroblockMaskInterner {
    masks: HashMap<MicroblockMask, &'static [u64; LAYERS]>,
}

fn intern_mask(mask: MicroblockMask) -> &'static [u64; LAYERS] {
    static INTERNER: OnceLock<Mutex<MicroblockMaskInterner>> = OnceLock::new();
    let mut interner = INTERNER
        .get_or_init(|| Mutex::new(MicroblockMaskInterner::default()))
        .lock()
        .expect("microblock mask interner lock was poisoned");
    if let Some(&layers) = interner.masks.get(&mask) {
        return layers;
    }

    let layers: &'static [u64; LAYERS] = Box::leak(Box::new(mask.layers));
    interner.masks.insert(mask, layers);
    layers
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
