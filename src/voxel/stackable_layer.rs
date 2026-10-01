use super::{
    cell::VoxelCell,
    microblock::{ArtisansKitResolution, MicroblockMask, MICROBLOCK_EDGE},
};

pub(crate) fn stackable_layer_mask(layer_count: usize) -> MicroblockMask {
    let edge = MICROBLOCK_EDGE as usize;
    let mut mask = MicroblockMask::EMPTY;
    for y in 0..layer_count.min(edge) {
        for z in 0..edge {
            for x in 0..edge {
                mask.edit(
                    [x, y, z],
                    ArtisansKitResolution::ExtraThin,
                    true,
                );
            }
        }
    }
    mask
}

pub(crate) fn stackable_layer_count(cell: VoxelCell) -> Option<usize> {
    if !MicroblockMask::is_modified(cell) {
        return None;
    }

    let mask = MicroblockMask::from_cell(cell);
    let edge = MICROBLOCK_EDGE as usize;
    let plane_area = edge * edge;
    let occupied = mask.occupied_count();
    if occupied == 0 || !occupied.is_multiple_of(plane_area) {
        return None;
    }

    let layer_count = occupied / plane_area;
    if layer_count >= edge {
        return None;
    }

    for y in 0..edge {
        let expected = y < layer_count;
        for z in 0..edge {
            for x in 0..edge {
                if mask.contains([x, y, z]) != expected {
                    return None;
                }
            }
        }
    }
    Some(layer_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        content::block_orientation::BlockOrientation,
        voxel::texture_rotation::TextureRotation,
    };

    #[test]
    fn horizontal_layers_fill_one_eighth_at_a_time() {
        let cell = VoxelCell::oriented(
            "asteria:test_layer",
            TextureRotation::default(),
            BlockOrientation::Y,
        );
        for count in 1..MICROBLOCK_EDGE as usize {
            let layered = stackable_layer_mask(count).apply_to_cell(cell, false);
            assert_eq!(stackable_layer_count(layered), Some(count));
        }
    }

    #[test]
    fn eighth_layer_normalizes_to_full_block_geometry() {
        let cell = VoxelCell::oriented(
            "asteria:test_layer",
            TextureRotation::default(),
            BlockOrientation::Y,
        );
        let full = stackable_layer_mask(MICROBLOCK_EDGE as usize).apply_to_cell(cell, false);
        assert!(!MicroblockMask::is_modified(full));
    }
}
