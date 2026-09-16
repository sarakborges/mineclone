"""One-shot guarded repair for the chat feature branch. Removed after validated commit."""
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    found = text.count(old)
    if found != 1:
        raise RuntimeError(f"{label}: expected one match, got {found}")
    return text.replace(old, new, 1)


path = Path("src/creatures/visual.rs")
source = path.read_text(encoding="utf-8")
source = replace_once(source, "    prelude::*,", "    ecs::system::SystemParam,\n    prelude::*,", "creature import")
for marker in ("VisualAttached", "CreatureAnimationLink"):
    source = replace_once(source, f"struct {marker}", f"pub(super) struct {marker}", marker)
source = replace_once(source, "fn configure_loaded_scene(", """#[derive(SystemParam)]
struct CreatureTintAssets<'w> {
    materials: ResMut<'w, Assets<StandardMaterial>>,
    cache: ResMut<'w, TintedCreatureMaterials>,
}

fn configure_loaded_scene(""", "tint context")
source = replace_once(source, "    mut materials: ResMut<Assets<StandardMaterial>>,\n    mut tint_cache: ResMut<TintedCreatureMaterials>,", "    mut tint_assets: CreatureTintAssets,", "tint parameters")
prefix, rest = source.split("fn configure_loaded_scene(", 1)
body, tail = rest.split("pub(super) fn sync_creature_animations(", 1)
assert body.count("tint_cache.0") == 2
body = body.replace("tint_cache.0", "tint_assets.cache.0")
body = replace_once(body, "materials.get(original.id())", "tint_assets.materials.get(original.id())", "tint lookup")
body = replace_once(body, "materials.add(material)", "tint_assets.materials.add(material)", "tint insertion")
path.write_text(prefix + "fn configure_loaded_scene(" + body + "pub(super) fn sync_creature_animations(" + tail, encoding="utf-8")

path = Path("src/hud/chat.rs")
source = path.read_text(encoding="utf-8")
source = replace_once(source, "    input::{ButtonState, keyboard::KeyboardInput},", "    ecs::system::SystemParam,\n    input::{ButtonState, keyboard::KeyboardInput},", "chat import")
source = replace_once(source, "fn handle_chat_input(", """#[derive(SystemParam)]
struct ChatInputContext<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    pause: Res<'w, State<PauseState>>,
    settings: Res<'w, State<SettingsState>>,
    inventory: Res<'w, State<InventoryState>>,
    brush_palette: Res<'w, State<BrushPaletteState>>,
}

fn handle_chat_input(""", "chat input context")
old = """    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut submissions: MessageWriter<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    pause: Res<State<PauseState>>,
    settings: Res<State<SettingsState>>,
    inventory: Res<State<InventoryState>>,
    brush_palette: Res<State<BrushPaletteState>>,"""
new = """    input: ChatInputContext,
    mut keyboard: MessageReader<KeyboardInput>,
    mut submissions: MessageWriter<ChatSubmission>,
    mut chat: ResMut<ChatState>,"""
source = replace_once(source, old, new, "chat input signature")
prefix, rest = source.split("fn handle_chat_input(", 1)
body, tail = rest.split("fn restore_game_cursor(", 1)
body = body.replace("keys.just_pressed(", "input.keys.just_pressed(")
body = body.replace("select_all_pressed(&keys)", "select_all_pressed(&input.keys)")
for marker in ("pause", "settings", "inventory", "brush_palette"):
    body = body.replace(f"*{marker}.get()", f"*input.{marker}.get()")
source = prefix + "fn handle_chat_input(" + body + "fn restore_game_cursor(" + tail
source = replace_once(source, "fn interpret_chat_submissions(", """#[derive(SystemParam)]
struct ChatCreatureContext<'w> {
    definitions: Res<'w, CreatureRegistry>,
    assets: Res<'w, AssetServer>,
    world: Res<'w, VoxelWorld>,
    language: Res<'w, ActiveLanguage>,
}

fn interpret_chat_submissions(""", "command context")
old = """    definitions: Res<CreatureRegistry>,
    assets: Res<AssetServer>,
    world: Res<VoxelWorld>,
    language: Res<ActiveLanguage>,"""
source = replace_once(source, old, "    creature_context: ChatCreatureContext,", "command signature")
prefix, rest = source.split("fn interpret_chat_submissions(", 1)
body, tail = rest.split("#[cfg(test)]", 1)
for marker in ("definitions", "assets", "world"):
    body = body.replace(f"&{marker}", f"&creature_context.{marker}")
body = body.replace("language.get()", "creature_context.language.get()")
path.write_text(prefix + "fn interpret_chat_submissions(" + body + "#[cfg(test)]" + tail, encoding="utf-8")

path = Path("src/hud/chat/visual.rs")
source = path.read_text(encoding="utf-8")
for marker in ("ChatRoot", "ChatHistory", "ChatInputRoot", "ChatDraft"):
    source = replace_once(source, f"struct {marker};", f"pub(super) struct {marker};", marker)
path.write_text(source, encoding="utf-8")

version = Path("VERSION")
assert version.read_text(encoding="utf-8").strip() == "0.17.1"
version.write_text("0.17.2\n", encoding="utf-8")
with Path("HANDOFF.md").open("a", encoding="utf-8") as handoff:
    handoff.write("\n\n## CI fixes — feature/game-chat-creature-command 0.17.2\nRepair Rust marker visibility and group related resources into Bevy SystemParam in creature visual and chat. No lint suppressions. Settings/New World conditionally visible scrollbars remain from 0.17.1. Feature branch only; develop untouched. Test Clippy, check and gameplay before merge.\n")
print("Guarded source corrections applied; run formatter and canonical CI gates before committing.")
