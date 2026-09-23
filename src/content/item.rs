use crate::localization::Language;

use super::{block::BlockRegistry, layer::LayerRegistry, tool::ToolRegistry};

pub(crate) fn display_name<'a>(
    item_id: &'a str,
    blocks: &'a BlockRegistry,
    layers: &'a LayerRegistry,
    tools: &'a ToolRegistry,
    language: Language,
) -> &'a str {
    if let Some(block) = blocks.get(item_id) {
        return block.name.text(language);
    }
    if let Some(layer) = layers.get(item_id) {
        return layer.name.text(language);
    }
    if let Some(tool) = tools.get(item_id) {
        return tool.name.text(language);
    }
    item_id
}
