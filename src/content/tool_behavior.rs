pub(crate) const NONE_TOOL_BEHAVIOR_ID: &str = "asteria:none";
pub(crate) const MINE_TOOL_BEHAVIOR_ID: &str = "asteria:mine";
pub(crate) const ARTISANS_KIT_REMOVE_BEHAVIOR_ID: &str = "asteria:artisans_kit/remove";
pub(crate) const ARTISANS_KIT_RESTORE_BEHAVIOR_ID: &str = "asteria:artisans_kit/restore";
pub(crate) const BRUSH_PAINT_BEHAVIOR_ID: &str = "asteria:brush/paint";
pub(crate) const BRUSH_PALETTE_BEHAVIOR_ID: &str = "asteria:brush/open_palette";
pub(crate) const LOG_HOLLOW_BEHAVIOR_ID: &str = "asteria:log/hollow";
pub(crate) const LOG_STRIP_BEHAVIOR_ID: &str = "asteria:log/strip";
pub(crate) const LAYER_REMOVE_BEHAVIOR_ID: &str = "asteria:layer/remove";
pub(crate) const STRUCTURE_SELECT_BEHAVIOR_ID: &str = "asteria:structure/select";

pub(crate) fn is_known_tool_behavior(id: &str) -> bool {
    matches!(
        id,
        NONE_TOOL_BEHAVIOR_ID
            | MINE_TOOL_BEHAVIOR_ID
            | ARTISANS_KIT_REMOVE_BEHAVIOR_ID
            | ARTISANS_KIT_RESTORE_BEHAVIOR_ID
            | BRUSH_PAINT_BEHAVIOR_ID
            | BRUSH_PALETTE_BEHAVIOR_ID
            | LOG_HOLLOW_BEHAVIOR_ID
            | LOG_STRIP_BEHAVIOR_ID
            | LAYER_REMOVE_BEHAVIOR_ID
            | STRUCTURE_SELECT_BEHAVIOR_ID
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_behavior_ids_are_registered() {
        for id in [
            NONE_TOOL_BEHAVIOR_ID,
            MINE_TOOL_BEHAVIOR_ID,
            ARTISANS_KIT_REMOVE_BEHAVIOR_ID,
            ARTISANS_KIT_RESTORE_BEHAVIOR_ID,
            BRUSH_PAINT_BEHAVIOR_ID,
            BRUSH_PALETTE_BEHAVIOR_ID,
            LOG_HOLLOW_BEHAVIOR_ID,
            LOG_STRIP_BEHAVIOR_ID,
            LAYER_REMOVE_BEHAVIOR_ID,
            STRUCTURE_SELECT_BEHAVIOR_ID,
        ] {
            assert!(is_known_tool_behavior(id));
        }
    }
}
