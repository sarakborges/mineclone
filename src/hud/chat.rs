mod autocomplete;
mod locate;
mod placement;
mod visual;

use std::collections::VecDeque;

use bevy::{
    ecs::system::SystemParam,
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    player::{camera::look::MouseLookInputState, inventory::InventoryState},
    tools::BrushPaletteState,
    world::warp::PendingWarp,
    ui::text_input::editable_value,
};

use autocomplete::{ChatAutocomplete, ParsedLine, parse_line, update_autocomplete};
use locate::{ChatLocateContext, PendingLocate, poll_locate_task};
use placement::ChatPlacementContext;
use visual::{
    advance_chat_timeout, rebuild_chat_history, render_autocomplete, scroll_chat_history,
    handle_chat_warp_links, scroll_chat_to_bottom, spawn_chat_ui, sync_chat_visibility,
};

const PLAYER_DISPLAY_NAME: &str = "Yogg'Sara";
const HISTORY_CAPACITY: usize = 64;
const CHAT_TIMEOUT_SECS: f32 = 10.0;
const MAX_INPUT_CHARS: usize = 256;

#[derive(Clone, Debug)]
pub(super) enum ChatMessage {
    Text(String),
    Located { prefix: String, target: IVec3 },
}

/// Oldest entries are first; visual order is the same as chronological order.
#[derive(Resource, Default)]
pub(crate) struct ChatState {
    open: bool,
    escape_consumed: bool,
    history: VecDeque<ChatMessage>,
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
    }

    pub(super) fn append(&mut self, message: ChatMessage) {
        if self.history.len() == HISTORY_CAPACITY {
            self.history.pop_front();
        }
        self.history.push_back(message);
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
            .init_resource::<ChatAutocomplete>()
            .init_resource::<PendingLocate>()
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
                    handle_chat_warp_links,
                    update_autocomplete,
                    interpret_chat_submissions,
                    poll_locate_task,
                    advance_chat_timeout,
                    scroll_chat_history,
                    sync_chat_visibility,
                    rebuild_chat_history,
                    render_autocomplete,
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

fn reset_chat(
    mut chat: ResMut<ChatState>,
    mut autocomplete: ResMut<ChatAutocomplete>,
    mut pending_locate: ResMut<PendingLocate>,
) {
    *chat = ChatState::default();
    *autocomplete = ChatAutocomplete::default();
    *pending_locate = PendingLocate::default();
}

fn close_chat_on_pause(
    mut chat: ResMut<ChatState>,
    mut autocomplete: ResMut<ChatAutocomplete>,
    mut focus: ResMut<InputFocus>,
) {
    if chat.is_open() {
        focus.clear();
    }
    chat.close();
    *autocomplete = ChatAutocomplete::default();
}

#[derive(SystemParam)]
struct ChatInputContext<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    pause: Res<'w, State<PauseState>>,
    settings: Res<'w, State<SettingsState>>,
    inventory: Res<'w, State<InventoryState>>,
    brush_palette: Res<'w, State<BrushPaletteState>>,
    focus: ResMut<'w, InputFocus>,
    autocomplete: ResMut<'w, ChatAutocomplete>,
}

fn handle_chat_input(
    mut input: ChatInputContext,
    mut submissions: MessageWriter<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    window: Single<&Window>,
    mut cursor: Single<&mut CursorOptions>,
    mut mouse_look: ResMut<MouseLookInputState>,
    mut draft: Single<(Entity, &mut EditableText), With<visual::ChatDraft>>,
) {
    if !input.keys.just_pressed(KeyCode::Escape) {
        chat.escape_consumed = false;
    }
    let (entity, editor) = &mut *draft;
    let entity = *entity;

    if chat.open {
        if *input.pause.get() != PauseState::Running {
            chat.close();
            input.focus.clear();
            return;
        }
        if input.keys.just_pressed(KeyCode::Escape) && !editor.is_composing() {
            if input.autocomplete.visible() {
                // First Esc only dismisses suggestions; the editor and draft stay intact.
                input.autocomplete.dismiss(editor);
                chat.escape_consumed = true;
                return;
            }
            chat.close();
            chat.escape_consumed = true;
            editor.clear();
            input.focus.clear();
            restore_game_cursor(window.focused, &mut cursor, &mut mouse_look);
            return;
        }
        if (input.keys.just_pressed(KeyCode::Enter)
            || input.keys.just_pressed(KeyCode::NumpadEnter))
            && !editor.is_composing()
        {
            let line = editable_value(editor).trim().to_owned();
            chat.close();
            editor.clear();
            input.focus.clear();
            restore_game_cursor(window.focused, &mut cursor, &mut mouse_look);
            if !line.is_empty() {
                submissions.write(ChatSubmission(line));
            }
            return;
        }
        if input.focus.get() != Some(entity) {
            input.focus.set(entity, FocusCause::Navigated);
        }
        return;
    }

    let can_open = *input.pause.get() == PauseState::Running
        && *input.settings.get() == SettingsState::Closed
        && *input.inventory.get() == InventoryState::Closed
        && *input.brush_palette.get() == BrushPaletteState::Closed;
    let has_command_modifier = [
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
    ]
    .iter()
    .any(|key| input.keys.pressed(*key));
    if !can_open || !input.keys.just_pressed(KeyCode::KeyT) || has_command_modifier {
        return;
    }

    editor.clear();
    *input.autocomplete = ChatAutocomplete::default();
    chat.open = true;
    input.focus.set(entity, FocusCause::Navigated);
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

fn interpret_chat_submissions(
    mut submissions: MessageReader<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    mut commands: Commands,
    mut placement: ChatPlacementContext,
    mut locate: ChatLocateContext,
    mut warp: ResMut<PendingWarp>,
) {
    // Commands::spawn is deferred, so preflight must also account for creatures
    // already requested by earlier submissions during this same frame.
    let mut reserved = Vec::new();
    for submission in submissions.read() {
        let response = match parse_line(&submission.0) {
            ParsedLine::Say(text) => format!("<{PLAYER_DISPLAY_NAME}>: {text}"),
            ParsedLine::Usage(usage) => format!("Usage: {usage}"),
            ParsedLine::Unknown(command) => format!("Unknown command: {command}"),
            ParsedLine::Spawn(id) => placement.spawn(&mut commands, id, &mut reserved),
            ParsedLine::Place(id, variation) => placement.place(id, variation, &reserved),
            ParsedLine::Locate(kind, id) => {
                let Some(player_block) = placement.player_block_position() else {
                    chat.append(ChatMessage::Text(
                        "Cannot locate: player is unavailable.".to_owned(),
                    ));
                    continue;
                };
                locate.start(kind, id, player_block)
            },
            ParsedLine::Warp(target) => {
                warp.request(target);
                format!(
                    "Warping to X: {} Z: {} Y: {}...",
                    target.x, target.z, target.y
                )
            }
        };
        chat.append(ChatMessage::Text(response));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_distinguished_from_plain_messages() {
        assert_eq!(parse_line("hello"), ParsedLine::Say("hello"));
        assert_eq!(parse_line(" /spawn asteria:meadow_slime "), ParsedLine::Spawn("asteria:meadow_slime"));
        assert_eq!(parse_line("/spawn"), ParsedLine::Usage("/spawn <id>"));
        assert_eq!(
            parse_line("/place"),
            ParsedLine::Usage("/place <id> [variation]")
        );
        assert_eq!(
            parse_line("/place foo extra extra"),
            ParsedLine::Usage("/place <id> [variation]")
        );
        assert_eq!(parse_line("/spawn_creature foo"), ParsedLine::Unknown("/spawn_creature"));
        assert_eq!(parse_line("/unknown"), ParsedLine::Unknown("/unknown"));
    }

    #[test]
    fn new_messages_append_below_old_messages_and_expire_when_closed() {
        let mut chat = ChatState::default();
        chat.append(ChatMessage::Text("old".to_owned()));
        chat.append(ChatMessage::Text("new".to_owned()));
        assert!(matches!(chat.history.front(), Some(ChatMessage::Text(text)) if text == "old"));
        assert!(matches!(chat.history.back(), Some(ChatMessage::Text(text)) if text == "new"));
        chat.since_last_message = CHAT_TIMEOUT_SECS;
        assert!(!chat.visible());
        chat.open = true;
        assert!(chat.visible());
    }

    #[test]
    fn history_retains_last_entries_in_chronological_order() {
        let mut chat = ChatState::default();
        for index in 0..=HISTORY_CAPACITY {
            chat.append(ChatMessage::Text(index.to_string()));
        }
        assert_eq!(chat.history.len(), HISTORY_CAPACITY);
        assert!(matches!(chat.history.front(), Some(ChatMessage::Text(text)) if text == "1"));
        assert!(matches!(chat.history.back(), Some(ChatMessage::Text(text)) if text == "64"));
    }
}
