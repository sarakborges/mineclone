"""Guarded corrections after native text editor migration, on diagnostics only."""
from pathlib import Path


def change(path: str, old: str, new: str) -> None:
    file = Path(path)
    source = file.read_text()
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f'{path}: expected exactly one occurrence of {old!r}, got {count}')
    file.write_text(source.replace(old, new, 1))


biome = 'src/screens/settings_screen/spawn_biome_section/systems.rs'
change(biome, "struct SpawnBiomeSearchContext<'w, 's> {", "pub(in crate::screens::settings_screen) struct SpawnBiomeSearchContext<'w, 's> {")

chat = 'src/hud/chat.rs'
change(chat,
       "    brush_palette: Res<'w, State<BrushPaletteState>>,\n}",
       "    brush_palette: Res<'w, State<BrushPaletteState>>,\n    focus: ResMut<'w, InputFocus>,\n}")
change(chat, '    input: ChatInputContext,\n', '    mut input: ChatInputContext,\n')
change(chat, '    mut focus: ResMut<InputFocus>,\n', '')
p = Path(chat)
s = p.read_text()
start = s.index('fn handle_chat_input(')
stop = s.index('fn restore_game_cursor(', start)
body = s[start:stop]
if body.count('focus.') != 6:
    raise RuntimeError(f'chat focus references changed: expected 6, got {body.count("focus.")}')
p.write_text(s[:start] + body.replace('focus.', 'input.focus.') + s[stop:])

screen = 'src/screens/settings_screen.rs'
for name in (
    'handle_spawn_biome_dropdown_button',
    'handle_spawn_biome_search_focus',
    'handle_spawn_biome_option_buttons',
    'handle_spawn_biome_search_keyboard',
    'handle_seed_keyboard',
    'sync_spawn_biome_dropdown_state',
    'sync_spawn_biome_options',
):
    change(screen, f'                    {name},\n',
           f'                    {name}.run_if(in_state(GameState::NewWorld)),\n')
change(screen, '                    handle_ticks_keyboard,\n',
       '                    handle_ticks_keyboard.run_if(has_ticks_input),\n')
p = Path(screen)
s = p.read_text()
if 'fn has_ticks_input(' in s:
    raise RuntimeError('ticks presence run condition already exists')
p.write_text(s + '\nfn has_ticks_input(inputs: Query<(), With<game_rules_section::TicksPerSecondInput>>) -> bool {\n    !inputs.is_empty()\n}\n')
print('Corrected two Clippy errors and gated new-world-only and conditional Settings text fields.')
