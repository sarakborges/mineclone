mod visual;

use std::collections::VecDeque;

use bevy::{
    ecs::system::SystemParam,
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    content::creature::CreatureRegistry,
    creatures::spawn_creature_at,
    localization::ActiveLanguage,
    player::{
        camera::{GameplayCamera, look::MouseLookInputState},
        inventory::InventoryState,
    },
    tools::BrushPaletteState,
    ui::text_input::{TextInputState, select_all_pressed},
    voxel::world::VoxelWorld,
};

use visual::{
    advance_chat_timeout, rebuild_chat_history, scroll_chat_history, scroll_chat_to_bottom,
    spawn_chat_ui, sync_chat_draft, sync_chat_visibility,
};

const PLAYER_DISPLAY_NAME: &str = "Yogg'Sara";
const HISTORY_CAPACITY: usize = 64;
const CHAT_TIMEOUT_SECS: f32 = 10.0;
const MAX_INPUT_CHARS: usize = 256;

/// Oldest entries are first; visual order is the same as chronological order.
#[derive(Resource, Default)]
pub(crate) struct ChatState {
    open: bool,
    escape_consumed: bool,
    draft: TextInputState,
    history: VecDeque<String>,
    since_last_message: f32,
    revision: u64,
}

impl ChatState {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn blocks_pause_escape(&self) -> bool {
        self.open || self.escape_consumed
    }

    fn close(&mut self) {
        self.open = false;
        self.draft.reset();
    }

    fn append(&mut self, text: String) {
        if self.history.len() == HISTORY_CAPACITY {
            self.history.pop_front();
        }
        self.history.push_back(text);
        self.since_last_message = 0.0;
        self.revision = self.revision.wrapping_add(1);
    }

    fn visible(&self) -> bool {
        self.open || (!self.history.is_empty() && self.since_last_message < CHAT_TIMEOUT_SECS)
    }
}

#[derive(Message)]
struct ChatSubmission(String);

pub(super) struct ChatHudPlugin;

impl Plugin for ChatHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatState>()
            .add_message::<ChatSubmission>()
            .add_systems(
                OnEnter(GameState::Gameplay),
                (reset_chat, spawn_chat_ui).chain(),
            )
            .add_systems(
                OnEnter(PauseState::Paused),
                close_chat_on_pause.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                (
                    handle_chat_input,
                    interpret_chat_submissions,
                    advance_chat_timeout,
                    scroll_chat_history,
                    sync_chat_visibility,
                    sync_chat_draft,
                    rebuild_chat_history,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Last,
                scroll_chat_to_bottom.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn reset_chat(mut chat: ResMut<ChatState>) {
    *chat = ChatState::default();
}

fn close_chat_on_pause(mut chat: ResMut<ChatState>) {
    chat.close();
}

#[derive(SystemParam)]
struct ChatInputContext<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    pause: Res<'w, State<PauseState>>,
    settings: Res<'w, State<SettingsState>>,
    inventory: Res<'w, State<InventoryState>>,
    brush_palette: Res<'w, State<BrushPaletteState>>,
}

fn handle_chat_input(
    input: ChatInputContext,
    mut keyboard: MessageReader<KeyboardInput>,
    mut submissions: MessageWriter<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    window: Single<&Window>,
    mut cursor: Single<&mut CursorOptions>,
    mut mouse_look: ResMut<MouseLookInputState>,
) {
    if !input.keys.just_pressed(KeyCode::Escape) {
        chat.escape_consumed = false;
    }

    if chat.open {
        if *input.pause.get() != PauseState::Running {
            chat.close();
            keyboard.clear();
            return;
        }
        if input.keys.just_pressed(KeyCode::Escape) {
            chat.close();
            chat.escape_consumed = true;
            restore_game_cursor(window.focused, &mut cursor, &mut mouse_look);
            keyboard.clear();
            return;
        }
        if input.keys.just_pressed(KeyCode::Enter) || input.keys.just_pressed(KeyCode::NumpadEnter)
        {
            let line = chat.draft.text().trim().to_owned();
            chat.close();
            restore_game_cursor(window.focused, &mut cursor, &mut mouse_look);
            if !line.is_empty() {
                submissions.write(ChatSubmission(line));
            }
            keyboard.clear();
            return;
        }
        if select_all_pressed(&input.keys) {
            chat.draft.select_all();
        }
        for event in keyboard.read() {
            if event.state != ButtonState::Pressed {
                continue;
            }
            if event.key_code == KeyCode::Backspace {
                chat.draft.backspace();
            } else if let Some(text) = &event.text {
                let remaining = MAX_INPUT_CHARS.saturating_sub(chat.draft.text().chars().count());
                if remaining > 0 {
                    let limited: String = text
                        .chars()
                        .filter(|character| !character.is_control())
                        .take(remaining)
                        .collect();
                    chat.draft.push_text(&limited);
                }
            }
        }
        return;
    }

    keyboard.clear();
    let can_open = *input.pause.get() == PauseState::Running
        && *input.settings.get() == SettingsState::Closed
        && *input.inventory.get() == InventoryState::Closed
        && *input.brush_palette.get() == BrushPaletteState::Closed;
    if !can_open || !input.keys.just_pressed(KeyCode::KeyT) {
        return;
    }

    chat.draft.reset();
    chat.draft.focus();
    chat.open = true;
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
    mouse_look.ignore_next_delta = true;
}

fn restore_game_cursor(
    focused: bool,
    cursor: &mut CursorOptions,
    mouse_look: &mut MouseLookInputState,
) {
    cursor.grab_mode = if focused {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !focused;
    mouse_look.ignore_next_delta = true;
}

#[derive(Debug, PartialEq, Eq)]
enum ParsedLine<'a> {
    Say(&'a str),
    SpawnCreature(&'a str),
    Usage,
    Unknown(&'a str),
}

fn parse_line(input: &str) -> ParsedLine<'_> {
    let line = input.trim();
    if !line.starts_with('/') {
        return ParsedLine::Say(line);
    }
    let mut words = line.split_whitespace();
    match words.next() {
        Some("/spawn_creature") => match (words.next(), words.next()) {
            (Some(id), None) => ParsedLine::SpawnCreature(id),
            _ => ParsedLine::Usage,
        },
        Some(command) => ParsedLine::Unknown(command),
        None => ParsedLine::Unknown("/"),
    }
}

#[derive(SystemParam)]
struct ChatCreatureContext<'w> {
    definitions: Res<'w, CreatureRegistry>,
    assets: Res<'w, AssetServer>,
    world: Res<'w, VoxelWorld>,
    language: Res<'w, ActiveLanguage>,
}

fn interpret_chat_submissions(
    mut submissions: MessageReader<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    mut commands: Commands,
    creature_context: ChatCreatureContext,
    players: Query<&Transform, With<GameplayCamera>>,
) {
    for submission in submissions.read() {
        let response = match parse_line(&submission.0) {
            ParsedLine::Say(text) => format!("<{PLAYER_DISPLAY_NAME}>: {text}"),
            ParsedLine::Usage => "Usage: /spawn_creature <id>".to_owned(),
            ParsedLine::Unknown(command) => format!("Unknown command: {command}"),
            ParsedLine::SpawnCreature(id) => {
                if let Some(eye) = players.iter().next().map(|player| player.translation) {
                    match spawn_creature_at(
                        &mut commands,
                        &creature_context.definitions,
                        &creature_context.assets,
                        &creature_context.world,
                        creature_context.language.get(),
                        id,
                        eye,
                    ) {
                        Ok(name) => format!("Spawned {name} ({id})."),
                        Err(error) => error,
                    }
                } else {
                    "Cannot spawn creature: player is unavailable.".to_owned()
                }
            }
        };
        chat.append(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_distinguished_from_plain_messages() {
        assert_eq!(parse_line("hello"), ParsedLine::Say("hello"));
        assert_eq!(
            parse_line(" /spawn_creature asteria:meadow_slime "),
            ParsedLine::SpawnCreature("asteria:meadow_slime")
        );
        assert_eq!(parse_line("/spawn_creature"), ParsedLine::Usage);
        assert_eq!(parse_line("/spawn_creature slime extra"), ParsedLine::Usage);
        assert_eq!(parse_line("/unknown"), ParsedLine::Unknown("/unknown"));
    }

    #[test]
    fn new_messages_append_below_old_messages_and_expire_when_closed() {
        let mut chat = ChatState::default();
        chat.append("old".to_owned());
        chat.append("new".to_owned());
        assert_eq!(chat.history.front().map(String::as_str), Some("old"));
        assert_eq!(chat.history.back().map(String::as_str), Some("new"));
        chat.since_last_message = CHAT_TIMEOUT_SECS;
        assert!(!chat.visible());
        chat.open = true;
        assert!(chat.visible());
    }

    #[test]
    fn history_retains_last_entries_in_chronological_order() {
        let mut chat = ChatState::default();
        for index in 0..=HISTORY_CAPACITY {
            chat.append(index.to_string());
        }
        assert_eq!(chat.history.len(), HISTORY_CAPACITY);
        assert_eq!(chat.history.front().map(String::as_str), Some("1"));
        assert_eq!(chat.history.back().map(String::as_str), Some("64"));
    }
}
