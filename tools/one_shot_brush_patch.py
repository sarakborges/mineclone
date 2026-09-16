#!/usr/bin/env python3
"""Apply only the brush HUD wiring, preserving all provided binary assets."""
from pathlib import Path
import subprocess

asset_paths = [
    'assets/textures/blocks/bassalt.png',
    'assets/textures/tools/brush.png',
    'assets/textures/tools/brush-tint.png',
]
original_assets = {path: Path(path).read_bytes() for path in asset_paths}


def replace(path, before, after, expected=1):
    file = Path(path)
    text = file.read_text(encoding='utf-8')
    actual = text.count(before)
    if actual != expected:
        raise SystemExit(f'{path}: expected {expected} replacements, found {actual}: {before!r}')
    file.write_text(text.replace(before, after), encoding='utf-8')


hotbar = 'src/hud/hotbar.rs'
replace(hotbar,
    'content::{block::BlockRegistry, block_orientation::BlockOrientation, tool::ToolRegistry},',
    'content::{\n        block::BlockRegistry, block_orientation::BlockOrientation,\n        secondary_property::SecondaryPropertyRegistry, tool::ToolRegistry,\n    },')
replace(hotbar,
    'hud::block_icon::BlockIconMaterial,',
    'hud::{block_icon::BlockIconMaterial, tool_icon::spawn_tool_icon},')
replace(hotbar,
    '    ui::{surface, typography, visibility::set_visibility},',
    '    tools::BrushMode,\n    ui::{surface, typography, visibility::set_visibility},')
replace(hotbar,
    "    tools: Res<'w, ToolRegistry>,\n    hotbar: Res<'w, PlayerHotbar>,",
    "    tools: Res<'w, ToolRegistry>,\n    dyes: Res<'w, SecondaryPropertyRegistry>,\n    brush_mode: Res<'w, BrushMode>,\n    hotbar: Res<'w, PlayerHotbar>,")
replace(hotbar,
    "    tools: &'a ToolRegistry,\n    language: Language,",
    "    tools: &'a ToolRegistry,\n    dyes: &'a SecondaryPropertyRegistry,\n    brush_mode: &'a BrushMode,\n    language: Language,")
replace(hotbar,
    '        tools: &content.tools,\n        language,',
    '        tools: &content.tools,\n        dyes: &content.dyes,\n        brush_mode: &content.brush_mode,\n        language,', 2)
replace(hotbar,
    '''    if let Some(tool) = items.tools.get(item_id) {
        slot.spawn((
            typography::caption(tool.name.text(items.language)),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ));
        return;
    }''',
    '''    if let Some(tool) = items.tools.get(item_id) {
        spawn_tool_icon(
            slot,
            tool,
            items.asset_server,
            items.brush_mode,
            items.dyes,
            items.language,
            ITEM_ICON_SIZE,
        );
        return;
    }''')

layout = 'src/hud/inventory/layout.rs'
replace(layout,
    'use crate::hud::block_icon::BlockIconMaterial;',
    'use crate::hud::{block_icon::BlockIconMaterial, tool_icon::spawn_tool_icon};')
replace(layout,
    "    pub(super) tools: &'a ToolRegistry,\n    pub(super) biomes: &'a BiomeRegistry,",
    "    pub(super) tools: &'a ToolRegistry,\n    pub(super) dyes: &'a crate::content::secondary_property::SecondaryPropertyRegistry,\n    pub(super) brush_mode: &'a crate::tools::BrushMode,\n    pub(super) biomes: &'a BiomeRegistry,")
replace(layout,
    '''    if let Some(tool) = items.tools.get(item_id) {
        root.spawn((
            InventoryCursorIcon,
            typography::caption(tool.name.text(items.language)),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x - 36.0),
                top: px(position.y - ITEM_ICON_SIZE * 0.5),
                width: px(72),
                ..default()
            },
            Pickable::IGNORE,
        ));
        return;
    }''',
    '''    if let Some(tool) = items.tools.get(item_id) {
        if tool.icon.is_empty() {
            root.spawn((
                InventoryCursorIcon,
                typography::caption(tool.name.text(items.language)),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(position.x - 36.0),
                    top: px(position.y - ITEM_ICON_SIZE * 0.5),
                    width: px(72),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        } else {
            root.spawn((
                InventoryCursorIcon,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(position.x - ITEM_ICON_SIZE * 0.5),
                    top: px(position.y - ITEM_ICON_SIZE * 0.5),
                    width: px(ITEM_ICON_SIZE),
                    height: px(ITEM_ICON_SIZE),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|cursor| {
                spawn_tool_icon(
                    cursor,
                    tool,
                    items.asset_server,
                    items.brush_mode,
                    items.dyes,
                    items.language,
                    ITEM_ICON_SIZE,
                );
            });
        }
        return;
    }''')
replace(layout,
    '''                CreativeCatalogItem::Tool(tool) => {
                    slot.spawn((
                        typography::caption(tool.name.text(items.language)),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                }''',
    '''                CreativeCatalogItem::Tool(tool) => {
                    spawn_tool_icon(
                        slot,
                        tool,
                        items.asset_server,
                        items.brush_mode,
                        items.dyes,
                        items.language,
                        ITEM_ICON_SIZE,
                    );
                }''')
replace(layout,
    '''    if let Some(tool) = items.tools.get(item_id) {
        slot.spawn((
            typography::caption(tool.name.text(items.language)),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ));
        return;
    }''',
    '''    if let Some(tool) = items.tools.get(item_id) {
        spawn_tool_icon(
            slot,
            tool,
            items.asset_server,
            items.brush_mode,
            items.dyes,
            items.language,
            ITEM_ICON_SIZE,
        );
        return;
    }''')

sync = 'src/hud/inventory/sync.rs'
replace(sync,
    'content::{inventory_category::InventoryCategoryRegistry, tool::ToolRegistry},',
    'content::{\n        inventory_category::InventoryCategoryRegistry,\n        secondary_property::SecondaryPropertyRegistry, tool::ToolRegistry,\n    },')
replace(sync,
    '    ui::surface,',
    '    tools::BrushMode,\n    ui::surface,')
replace(sync,
    "    tools: Res<'w, ToolRegistry>,\n    language: Res<'w, ActiveLanguage>,",
    "    tools: Res<'w, ToolRegistry>,\n    dyes: Res<'w, SecondaryPropertyRegistry>,\n    brush_mode: Res<'w, BrushMode>,\n    language: Res<'w, ActiveLanguage>,")
replace(sync,
    '            tools: &self.tools,\n            biomes: &self.visual.biomes,',
    '            tools: &self.tools,\n            dyes: &self.dyes,\n            brush_mode: &self.brush_mode,\n            biomes: &self.visual.biomes,')

for path, original in original_assets.items():
    if Path(path).read_bytes() != original:
        raise SystemExit(f'Asset modified unexpectedly: {path}')
subprocess.run(['git', 'diff', '--check'], check=True)
print('Brush icon wiring complete; all three supplied PNG assets unchanged.')
