mod autocomplete;
mod commands;
mod locate;
mod placement;
mod visual;

use std::{collections::VecDeque, path::PathBuf};

use bevy::{
    ecs::system::SystemParam,
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    app::{
        game_state::GameState,
        keybinds::{KeybindAction, Keybinds},
        pause_state::PauseState,
        settings_state::SettingsState,
    },
    content::creature::CreatureRegistry,
    creatures::{
        CreatureAnimationState, CreatureDeathTimer, CreatureInstance, EntityMetaTags,
        spawn_creature_at_with_tags,
    },
    entity::EntityHealth,
    gameplay::modal::GameplayModalState,
    localization::ActiveLanguage,
    player::{
        PLAYER_DISPLAY_NAME,
        camera::look::MouseLookInputState,
        game_mode::{GameMode, not_spectator},
    },
    targeting::block::TargetedCreature,
    ui::text_input::editable_value,
    world::warp::{PendingWarp, WarpOutcome},
};

use autocomplete::{ChatAutocomplete, update_autocomplete};
use commands::{ModifyAction, ParsedLine, parse_line};
use locate::{ChatLocateContext, PendingLocate, poll_locate_task};
use placement::ChatPlacementContext;
use visual::{
    advance_chat_timeout, rebuild_chat_history, handle_chat_open_structure_file,
    handle_chat_warp_links, render_autocomplete, scroll_chat_history, scroll_chat_to_bottom,
    spawn_chat_ui, sync_chat_visibility,
};

const HISTORY_CAPACITY: usize = 64;
const CHAT_TIMEOUT_SECS: f32 = 10.0;
const MAX_INPUT_CHARS: usize = 256;

#[derive(Clone, Debug)]
pub(super) enum ChatMessage {
    Text(String),
    Error(String),
    Located { prefix: String, target: IVec3 },
    StructureFile(PathBuf),
}

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

    pub(crate) fn append_text(&mut self, text: impl Into<String>) {
        self.append(ChatMessage::Text(text.into()));
    }

    pub(crate) fn append_error(&mut self, text: impl Into<String>) {
        self.append(ChatMessage::Error(text.into()));
    }

    pub(crate) fn append_structure_file(&mut self, path: PathBuf) {
        self.append(ChatMessage::StructureFile(path));
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
                    handle_chat_open_structure_file,
                    update_autocomplete.run_if(not_spectator),
                    interpret_chat_submissions,
                    poll_locate_task,
                    normalize_locate_failures,
                    poll_warp_outcome,
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
    keybinds: Res<'w, Keybinds>,
    pause: Res<'w, State<PauseState>>,
    settings: Res<'w, State<SettingsState>>,
    modal: Res<'w, State<GameplayModalState>>,
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
        && *input.modal.get() == GameplayModalState::Closed;
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
    if !can_open || !input.keys.just_pressed(input.keybinds.key_code(KeybindAction::Chat)) || has_command_modifier {
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

type CommandTargetQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Name,
        &'static Transform,
        &'static mut EntityHealth,
        &'static mut EntityMetaTags,
        Option<&'static mut CreatureAnimationState>,
    ),
    With<CreatureInstance>,
>;

#[derive(SystemParam)]
struct ChatCommandContent<'w, 's> {
    definitions: Res<'w, CreatureRegistry>,
    assets: Res<'w, AssetServer>,
    language: Res<'w, ActiveLanguage>,
    targeted: Res<'w, TargetedCreature>,
    targets: CommandTargetQuery<'w, 's>,
}

fn format_position(position: IVec3) -> String {
    format!("X: {} Z: {} Y: {}", position.x, position.z, position.y)
}

fn command_target_position(transform: &Transform) -> IVec3 {
    transform.translation.floor().as_ivec3()
}

fn interpret_chat_submissions(
    mut submissions: MessageReader<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    mut commands: Commands,
    game_mode: Single<&GameMode>,
    mut placement: ChatPlacementContext,
    mut locate: ChatLocateContext,
    mut warp: ResMut<PendingWarp>,
    mut content: ChatCommandContent,
) {
    let mut reserved = Vec::new();
    for submission in submissions.read() {
        let parsed = parse_line(&submission.0);
        if game_mode.is_spectator() && !matches!(parsed, ParsedLine::Say(_)) {
            chat.append_error("commands unavailable in spectator mode");
            continue;
        }

        match parsed {
            ParsedLine::Say(text) => chat.append_text(format!("<{PLAYER_DISPLAY_NAME}>: {text}")),
            ParsedLine::Usage(usage) => chat.append_error(format!("Usage: {usage}")),
            ParsedLine::Unknown(command) => chat.append_error(format!("Unknown command: {command}")),
            ParsedLine::Spawn(id, meta_tag) => {
                let Some(definition) = content.definitions.get(id) else {
                    chat.append_error("spawn failed");
                    continue;
                };
                let Some(position) = placement.player_block_position() else {
                    chat.append_error("spawn failed");
                    continue;
                };
                let name = definition.name.text(content.language.get()).to_owned();

                if let Some(meta_tag) = meta_tag {
                    let mut meta_tags = EntityMetaTags::default();
                    if meta_tags.add(meta_tag, None).is_err() {
                        chat.append_error("spawn failed");
                        continue;
                    }
                    let feet = Vec3::new(
                        position.x as f32 + 0.5,
                        position.y as f32,
                        position.z as f32 + 0.5,
                    );
                    if spawn_creature_at_with_tags(
                        &mut commands,
                        &content.definitions,
                        &content.assets,
                        content.language.get(),
                        id,
                        feet,
                        meta_tags,
                    )
                    .is_err()
                    {
                        chat.append_error("spawn failed");
                        continue;
                    }
                    chat.append_text(format!(
                        "spawned {name} at {}",
                        format_position(position)
                    ));
                    continue;
                }

                let response = placement.spawn(&mut commands, id, &mut reserved);
                if response.starts_with("Spawned ") {
                    chat.append_text(format!(
                        "spawned {name} at {}",
                        format_position(position)
                    ));
                } else {
                    chat.append_error("spawn failed");
                }
            }
            ParsedLine::Place(id, variation) => {
                let response = placement.place(id, variation, &reserved);
                if response.starts_with("Placed ") {
                    chat.append_text(response);
                } else {
                    chat.append_error("place failed");
                }
            }
            ParsedLine::Locate(kind, id, variation) => {
                let Some(player_block) = placement.player_block_position() else {
                    chat.append_error("locate failed");
                    continue;
                };
                let response = locate.start(kind, id, variation, player_block);
                if response.starts_with("Locating ") {
                    chat.append_text(response);
                } else {
                    chat.append_error("locate failed");
                }
            }
            ParsedLine::Warp(target) => {
                warp.request(target);
                chat.append_text(format!("warping to {}...", format_position(target)));
            }
            ParsedLine::Kill => {
                let Some(entity) = content.targeted.0 else {
                    chat.append_error("kill failed");
                    continue;
                };
                let Ok((name, transform, mut health, _, animation)) = content.targets.get_mut(entity) else {
                    chat.append_error("kill failed");
                    continue;
                };
                let position = command_target_position(transform);
                let current = health.current();
                health.damage(current);
                if let Some(mut animation) = animation {
                    animation.trigger("death");
                }
                commands.entity(entity).insert(CreatureDeathTimer(Timer::from_seconds(
                    0.75,
                    TimerMode::Once,
                )));
                chat.append_text(format!(
                    "killed {} at {}",
                    name.as_str(),
                    format_position(position)
                ));
            }
            ParsedLine::Modify(action, tag, value) => {
                let Some(entity) = content.targeted.0 else {
                    chat.append_error("modify failed");
                    continue;
                };
                let Ok((name, transform, _, mut meta_tags, _)) = content.targets.get_mut(entity) else {
                    chat.append_error("modify failed");
                    continue;
                };
                let result = match action {
                    ModifyAction::Add => meta_tags.add(tag, value.map(str::to_owned)),
                    ModifyAction::Remove => meta_tags.remove(tag),
                    ModifyAction::Edit => meta_tags.edit(tag, value.map(str::to_owned)),
                };
                if result.is_err() {
                    chat.append_error("modify failed");
                    continue;
                }
                let verb = match action {
                    ModifyAction::Add => "added",
                    ModifyAction::Remove => "removed",
                    ModifyAction::Edit => "edited",
                };
                chat.append_text(format!(
                    "{verb} {tag} on {} at {}",
                    name.as_str(),
                    format_position(command_target_position(transform))
                ));
            }
        }
    }
}

fn normalize_locate_failures(mut chat: ResMut<ChatState>) {
    let Some(ChatMessage::Text(text)) = chat.history.back() else {
        return;
    };
    if !text.contains("could not be found within") {
        return;
    }
    let Some(last) = chat.history.back_mut() else {
        return;
    };
    *last = ChatMessage::Error("locate failed".to_owned());
    chat.revision = chat.revision.wrapping_add(1);
}

fn poll_warp_outcome(mut warp: ResMut<PendingWarp>, mut chat: ResMut<ChatState>) {
    match warp.take_outcome() {
        Some(WarpOutcome::Succeeded(position)) => {
            chat.append_text(format!("warped to {}", format_position(position)));
        }
        Some(WarpOutcome::Failed) => chat.append_error("warp failed"),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_distinguished_from_plain_messages() {
        assert_eq!(parse_line("hello"), ParsedLine::Say("hello"));
        assert_eq!(
            parse_line(" /spawn asteria:meadow_slime "),
            ParsedLine::Spawn("asteria:meadow_slime", None)
        );
        assert_eq!(
            parse_line("/spawn"),
            ParsedLine::Usage("/spawn <id> [meta_tag]")
        );
        assert_eq!(parse_line("/kill"), ParsedLine::Kill);
        assert_eq!(
            parse_line("/modify add NO_AI"),
            ParsedLine::Modify(ModifyAction::Add, "NO_AI", None)
        );
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
