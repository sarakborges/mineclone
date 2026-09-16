from pathlib import Path

path = Path('tools/one_shot_brush_patch.py')
text = path.read_text(encoding='utf-8')
old_anchor = '    "    tools: &\'a ToolRegistry,\\n    language: Language,",'
new_anchor = '    "struct HotbarItemView<\'a> {\\n    asset_server: &\'a AssetServer,\\n    blocks: &\'a BlockRegistry,\\n    tools: &\'a ToolRegistry,\\n    language: Language,",'
old_result = '    "    tools: &\'a ToolRegistry,\\n    dyes: &\'a SecondaryPropertyRegistry,\\n    brush_mode: &\'a BrushMode,\\n    language: Language,")'
new_result = '    "struct HotbarItemView<\'a> {\\n    asset_server: &\'a AssetServer,\\n    blocks: &\'a BlockRegistry,\\n    tools: &\'a ToolRegistry,\\n    dyes: &\'a SecondaryPropertyRegistry,\\n    brush_mode: &\'a BrushMode,\\n    language: Language,")'
if text.count(old_anchor) != 1 or text.count(old_result) != 1:
    raise SystemExit('Unexpected brush patch source; refusing non-deterministic edit')
path.write_text(text.replace(old_anchor, new_anchor).replace(old_result, new_result), encoding='utf-8')
