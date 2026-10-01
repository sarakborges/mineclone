use super::block::BlockDefinition;

pub(crate) const STACKABLE_LAYER_BLOCK_TAG: &str = "stackable_layer";
pub(crate) const STACKABLE_LAYER_HEIGHT: f32 = 1.0 / 8.0;

pub(crate) fn is_stackable_layer(block: &BlockDefinition) -> bool {
    block
        .tags
        .iter()
        .any(|tag| tag == STACKABLE_LAYER_BLOCK_TAG)
}

pub(crate) fn stackable_layer_full_block_id(block: &BlockDefinition) -> Option<&str> {
    if !is_stackable_layer(block) {
        return None;
    }
    block.id.strip_suffix("_layer")
}
