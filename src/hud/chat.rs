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
        crash_log::log_gameplay_event,
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
    localization::{ActiveLanguage, Language, UiLocalization},
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
    advance_chat_timeout, handle_chat_open_structure_file, handle_chat_warp_links,
    rebuild_chat_history, render_autocomplete, scroll_chat_history, scroll_chat_to_bottom,
    spawn_chat_ui, sync_chat_visibility,
};

const HISTORY_CAPACITY: usize = 64;
const CHAT_TIMEOUT_SECS: f32 = 10.0;
const MAX_INPUT_CHARS: usize = 256;

#[derive(Clone, Debug)]
pub(super) enum ChatMessage {
    Text(String),
    Error(String),
    #[allow(dead_code)]
    Located { prefix: String, target: IVec3 },
    StructureFile(PathBuf),
}

#[derive(Resource, Default)]
pub(crate) struct ChatState {
    open: bool,
    escape_consumed: bool,
    command_target: Option<Entity>,
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
        self.command_target = None;
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
        let text = text.into();
        log_gameplay_event(format!("chat.feedback kind=text text={text:?}"));
        self.append(ChatMessage::Text(text));
    }

    pub(crate) fn append_error(&mut self, text: impl Into<String>) {
        let text = text.into();
        log_gameplay_event(format!("chat.feedback kind=error text={text:?}"));
        self.append(ChatMessage::Error(text));
    }

    pub(crate) fn append_structure_file(&mut self, path: PathBuf) {
        self.append(ChatMessage::StructureFile(path));
    }

    fn visible(&self) -> bool {
        self.open || (!self.history.is_empty() && self.since_last_message < CHAT_TIMEOUT_SECS)
    }
}

#[derive(Message)]
struct ChatSubmission {
    line: String,
    target: Option<Entity>,
}

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
                    localize_locate_feedback,
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
    *pending_locate = PendingLocate;
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
    targeted: Res<'w, TargetedCreature>,
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

        if input.keys.just_pressed(KeyCode::Escape) {
            chat.close();
            input.focus.clear();
            chat.escape_consumed = true;
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            mouse_look.skip_next_motion();
            return;
        }

        if input.keys.just_pressed(KeyCode::Enter) {
            let line = editable_value(editor);
            if !line.is_empty() {
                submissions.write(ChatSubmission {
                    line,
                    target: chat.command_target,
                });
            }
            **editor = String::new();
            chat.close();
            input.focus.clear();
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            mouse_look.skip_next_motion();
            return;
        }

        return;
    }

    if *input.pause.get() != PauseState::Running
        || *input.settings.get() != SettingsState::Closed
        || *input.modal.get() != GameplayModalState::Closed
    {
        return;
    }

    if input
        .keys
        .just_pressed(input.keybinds.key_code(KeybindAction::OpenChat))
    {
        **editor = String::new();
        chat.open = true;
        chat.command_target = None;
        input.focus.set(entity, FocusCause::Programmatic);
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        return;
    }

    if input.keys.just_pressed(KeyCode::Slash) {
        **editor = "/".to_owned();
        chat.open = true;
        chat.command_target = None;
        input.focus.set(entity, FocusCause::Programmatic);
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        return;
    }

    if input
        .keys
        .just_pressed(input.keybinds.key_code(KeybindAction::OpenCommand))
        && let Some(entity) = input.targeted.entity
    {
        **editor = "/".to_owned();
        chat.open = true;
        chat.command_target = Some(entity);
        input.focus.set(entity, FocusCause::Programmatic);
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn interpret_chat_submissions(
    mut messages: MessageReader<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    mut locate: ChatLocateContext,
    mut placement: ChatPlacementContext,
    creatures: Res<CreatureRegistry>,
    language: Res<ActiveLanguage>,
    mut warps: ResMut<PendingWarp>,
    mut commands: Commands,
    mut targets: Query<(
        Entity,
        &mut Transform,
        &mut EntityHealth,
        &mut EntityMetaTags,
        &mut CreatureAnimationState,
        Option<&CreatureDeathTimer>,
    ), With<CreatureInstance>>,
    game_modes: Query<&mut GameMode>,
) {
    for message in messages.read() {
        let parsed = parse_line(&message.line);
        let response = match parsed {
            ParsedLine::Chat => message.line.clone(),
            ParsedLine::Command(command) => match command {
                commands::ChatCommand::Locate {
                    target_kind,
                    id,
                    variation,
                } => {
                    let player_block = targets
                        .get(message.target.unwrap_or(Entity::PLACEHOLDER))
                        .ok()
                        .map(|(_, transform, ..)| transform.translation.floor().as_ivec3())
                        .unwrap_or_default();
                    locate.start(&target_kind, &id, variation, player_block)
                }
                commands::ChatCommand::PlaceStructure { id, variation } => {
                    placement.place(&id, variation)
                }
                commands::ChatCommand::Warp { x, y, z } => {
                    warps.request(IVec3::new(x, y, z));
                    format!("Warping to X: {x} Z: {z} Y: {y}.")
                }
                commands::ChatCommand::Spawn {
                    id,
                    position,
                    tags,
                } => {
                    let Some(definition) = creatures.get(&id) else {
                        chat.append_error(format!("Unknown creature id: {id}"));
                        continue;
                    };
                    spawn_creature_at_with_tags(
                        &mut commands,
                        definition,
                        position.as_vec3() + Vec3::splat(0.5),
                        &tags,
                    );
                    format!("Spawned {id}.")
                }
                commands::ChatCommand::Kill => {
                    let Some(target) = message.target else {
                        chat.append_error("No creature targeted.");
                        continue;
                    };
                    let Ok((entity, _, mut health, _, _, death_timer)) = targets.get_mut(target)
                    else {
                        chat.append_error("Target is no longer available.");
                        continue;
                    };
                    if death_timer.is_some() || health.current() <= 0.0 {
                        chat.append_error("Target is already dead.");
                        continue;
                    }
                    health.set_current(0.0);
                    commands.entity(entity).insert(CreatureDeathTimer::default());
                    "Killed target.".to_owned()
                }
                commands::ChatCommand::Modify { action, value } => {
                    let Some(target) = message.target else {
                        chat.append_error("No creature targeted.");
                        continue;
                    };
                    let Ok((_, mut transform, mut health, mut tags, mut animation, _)) =
                        targets.get_mut(target)
                    else {
                        chat.append_error("Target is no longer available.");
                        continue;
                    };
                    match action {
                        ModifyAction::Scale => {
                            let scale = value.parse::<f32>().ok();
                            let Some(scale) = scale.filter(|scale| *scale > 0.0) else {
                                chat.append_error("Scale must be a positive number.");
                                continue;
                            };
                            transform.scale = Vec3::splat(scale);
                            format!("Set scale to {scale}.")
                        }
                        ModifyAction::Health => {
                            let value = value.parse::<f32>().ok();
                            let Some(value) = value.filter(|value| *value >= 0.0) else {
                                chat.append_error("Health must be zero or greater.");
                                continue;
                            };
                            health.set_current(value);
                            format!("Set health to {value}.")
                        }
                        ModifyAction::AddTag => {
                            tags.insert(value.clone());
                            format!("Added tag {value}.")
                        }
                        ModifyAction::RemoveTag => {
                            tags.remove(&value);
                            format!("Removed tag {value}.")
                        }
                        ModifyAction::Animation => {
                            animation.set_override(value.clone());
                            format!("Set animation override to {value}.")
                        }
                    }
                }
                commands::ChatCommand::GameMode { mode } => {
                    let Some(target) = message.target else {
                        chat.append_error("No player targeted.");
                        continue;
                    };
                    let Ok(mut game_mode) = game_modes.get_mut(target) else {
                        chat.append_error("Target player is no longer available.");
                        continue;
                    };
                    *game_mode = mode;
                    format!("Set game mode to {:?}.", mode)
                }
            },
        };
        chat.append_text(response);
    }
}

fn localize_locate_feedback(mut chat: ResMut<ChatState>, localization: Res<UiLocalization>) {
    if !chat.is_changed() {
        return;
    }
    for message in chat.history.iter_mut() {
        if let ChatMessage::Located { prefix, .. } = message
            && prefix.starts_with("__locate__")
        {
            *prefix = localization.text("chat.locate.found").to_owned();
        }
    }
}

fn poll_warp_outcome(
    mut outcomes: MessageReader<WarpOutcome>,
    mut chat: ResMut<ChatState>,
    language: Res<ActiveLanguage>,
) {
    for outcome in outcomes.read() {
        match outcome {
            WarpOutcome::Arrived(position) => chat.append_text(format!(
                "Warped to X: {} Z: {} Y: {}.",
                position.x, position.z, position.y
            )),
            WarpOutcome::Failed(error) => chat.append_error(error.clone()),
        }
    }
}