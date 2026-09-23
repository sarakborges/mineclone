use crate::content::block::BlockDefinition;

use super::{block_face::BlockFace, cell::VoxelCell};

pub(crate) const HOLLOW_LOG_PROPERTY: &str = "asteria:hollow";
pub(crate) const STRIPPED_LOG_PROPERTY: &str = "asteria:stripped";
pub(crate) const LOG_STATE_ENABLED: &str = "true";

pub(crate) fn is_hollow(cell: VoxelCell) -> bool {
    cell.secondary_property(HOLLOW_LOG_PROPERTY) == Some(LOG_STATE_ENABLED)
}

pub(crate) fn is_stripped(cell: VoxelCell) -> bool {
    cell.secondary_property(STRIPPED_LOG_PROPERTY) == Some(LOG_STATE_ENABLED)
}

pub(crate) fn texture_face(cell: VoxelCell, source_face: BlockFace) -> BlockFace {
    if is_stripped(cell) && !matches!(source_face, BlockFace::Top | BlockFace::Bottom) {
        BlockFace::Top
    } else {
        source_face
    }
}

pub(crate) fn hollow_surface_texture_face(
    cell: VoxelCell,
    source_face: BlockFace,
    interior_surface: bool,
) -> BlockFace {
    if interior_surface && is_hollow(cell) {
        BlockFace::Top
    } else {
        texture_face(cell, source_face)
    }
}

pub(crate) fn has_valid_log_state(cell: VoxelCell, block: &BlockDefinition) -> bool {
    let hollow = cell.secondary_property(HOLLOW_LOG_PROPERTY);
    let stripped = cell.secondary_property(STRIPPED_LOG_PROPERTY);
    let values_valid = hollow.is_none_or(|value| value == LOG_STATE_ENABLED)
        && stripped.is_none_or(|value| value == LOG_STATE_ENABLED);
    values_valid && ((hollow.is_none() && stripped.is_none()) || block.is_log())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::texture_rotation::TextureRotation;

    #[test]
    fn stripped_and_hollow_states_can_coexist() {
        let cell = VoxelCell::new("asteria:log", TextureRotation::default())
            .with_secondary_property(HOLLOW_LOG_PROPERTY, LOG_STATE_ENABLED)
            .with_secondary_property(STRIPPED_LOG_PROPERTY, LOG_STATE_ENABLED);

        assert!(is_hollow(cell));
        assert!(is_stripped(cell));
    }

    #[test]
    fn stripped_logs_reuse_exposed_wood_texture_on_bark_faces() {
        let cell = VoxelCell::new("asteria:log", TextureRotation::default())
            .with_secondary_property(STRIPPED_LOG_PROPERTY, LOG_STATE_ENABLED);

        assert_eq!(texture_face(cell, BlockFace::Front), BlockFace::Top);
        assert_eq!(texture_face(cell, BlockFace::Top), BlockFace::Top);
    }

    #[test]
    fn hollow_interior_uses_exposed_wood_texture_without_stripping_outer_bark() {
        let cell = VoxelCell::new("asteria:log", TextureRotation::default())
            .with_secondary_property(HOLLOW_LOG_PROPERTY, LOG_STATE_ENABLED);

        assert_eq!(texture_face(cell, BlockFace::Front), BlockFace::Front);
        assert_eq!(
            hollow_surface_texture_face(cell, BlockFace::Front, true),
            BlockFace::Top
        );
    }
}
