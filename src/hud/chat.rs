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
            let target = chat.command_target;
            chat.close();
            editor.clear();
            input.focus.clear();
            restore_game_cursor(window.focused, &mut cursor, &mut mouse_look);
            if !line.is_empty() {
                submissions.write(ChatSubmission { line, target });
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
    if !can_open
        || !input
            .keys
            .just_pressed(input.keybinds.key_code(KeybindAction::Chat))
        || has_command_modifier
    {
        return;
    }

    editor.clear();
    *input.autocomplete = ChatAutocomplete::default();
    chat.command_target = input.targeted.0;
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
    localization: Res<'w, UiLocalization>,
    targets: CommandTargetQuery<'w, 's>,
}

fn format_position(position: IVec3) -> String {
    format!("X: {} Z: {} Y: {}", position.x, position.z, position.y)
}

fn command_target_position(transform: &Transform) -> IVec3 {
    transform.translation.floor().as_ivec3()
}

fn feedback(
    localization: &UiLocalization,
    language: Language,
    key: &str,
    replacements: &[(&str, &str)],
) -> String {
    localization.format(language, key, replacements)
}

fn parse_unknown_variation(message: &str) -> Option<(&str, &str, &str)> {
    let rest = message.strip_prefix("Unknown variation ")?;
    let (variation, rest) = rest.split_once(" for ")?;
    let (id, count) = rest.split_once("; expected 1..=")?;
    Some((variation, id, count.trim_end_matches('.')))
}

fn localize_internal_error(
    message: &str,
    localization: &UiLocalization,
    language: Language,
) -> String {
    let message = message.trim();

    if let Some(tag) = message.strip_prefix("unknown meta tag: ") {
        return feedback(
            localization,
            language,
            "chat.command.meta.unknown",
            &[("tag", tag)],
        );
    }
    if let Some(tag) = message.strip_prefix("meta tag already exists: ") {
        return feedback(
            localization,
            language,
            "chat.command.meta.exists",
            &[("tag", tag)],
        );
    }
    if let Some(tag) = message.strip_prefix("meta tag is not set: ") {
        return feedback(
            localization,
            language,
            "chat.command.meta.notSet",
            &[("tag", tag)],
        );
    }
    if let Some(id) = message.strip_prefix("Unknown creature id: ") {
        return feedback(
            localization,
            language,
            "chat.command.spawn.unknownCreature",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if message == "Cannot spawn creature: player is unavailable." {
        return feedback(
            localization,
            language,
            "chat.command.spawn.playerUnavailable",
            &[],
        );
    }
    if let Some(id) = message.strip_prefix("not enough space to spawn ") {
        return feedback(
            localization,
            language,
            "chat.command.spawn.noSpace",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message
        .strip_prefix("Structure set ")
        .and_then(|rest| rest.strip_suffix(" does not have variations."))
    {
        return feedback(
            localization,
            language,
            "chat.command.place.setNoVariations",
            &[("id", id)],
        );
    }
    if let Some(id) = message.strip_prefix("Unknown structure, structure group, or structure set: ") {
        return feedback(
            localization,
            language,
            "chat.command.place.unknownStructure",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some((variation, id, count)) = parse_unknown_variation(message) {
        return feedback(
            localization,
            language,
            "chat.command.place.unknownVariation",
            &[("variation", variation), ("id", id), ("count", count)],
        );
    }
    if message == "Cannot place structure: player is unavailable." {
        return feedback(
            localization,
            language,
            "chat.command.place.playerUnavailable",
            &[],
        );
    }
    if message == "Cannot place structure set: player is unavailable." {
        return feedback(
            localization,
            language,
            "chat.command.place.setPlayerUnavailable",
            &[],
        );
    }
    if let Some(id) = message.strip_prefix("no loaded ground available to place ") {
        return feedback(
            localization,
            language,
            "chat.command.place.noGround",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message.strip_prefix("not enough loaded space to place ") {
        return feedback(
            localization,
            language,
            "chat.command.place.noLoadedSpace",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message.strip_prefix("not enough safe space to place ") {
        return feedback(
            localization,
            language,
            "chat.command.place.noSafeSpace",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message
        .strip_prefix("could not resolve structure set ")
        .and_then(|rest| rest.strip_suffix(" in loaded terrain"))
    {
        return feedback(
            localization,
            language,
            "chat.command.place.setResolveFailed",
            &[("id", id)],
        );
    }
    if message == "Cannot locate: current dimension is unavailable." {
        return feedback(
            localization,
            language,
            "chat.command.locate.dimensionUnavailable",
            &[],
        );
    }
    if let Some(id) = message.strip_prefix("Unknown biome id: ") {
        return feedback(
            localization,
            language,
            "chat.command.locate.unknownBiome",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message.strip_prefix("Biome is not active in this dimension: ") {
        return feedback(
            localization,
            language,
            "chat.command.locate.biomeInactive",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message.strip_prefix("Structure set cannot be located by command: ") {
        return feedback(
            localization,
            language,
            "chat.command.locate.setNotLocatable",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message.strip_prefix("Structure set is not generated in this dimension: ") {
        return feedback(
            localization,
            language,
            "chat.command.locate.setNotGenerated",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message.strip_prefix("Structure cannot be located by command: ") {
        return feedback(
            localization,
            language,
            "chat.command.locate.structureNotLocatable",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(id) = message.strip_prefix("Structure is not generated in this dimension: ") {
        return feedback(
            localization,
            language,
            "chat.command.locate.structureNotGenerated",
            &[("id", id.trim_end_matches('.'))],
        );
    }
    if let Some(usage) = message.strip_prefix("Usage: ") {
        return feedback(
            localization,
            language,
            "chat.command.usage",
            &[("usage", usage)],
        );
    }

    feedback(localization, language, "chat.command.failed", &[])
}

fn localize_place_success(
    response: &str,
    localization: &UiLocalization,
    language: Language,
) -> Option<String> {
    let rest = response.strip_prefix("Placed ")?.strip_suffix('.')?;
    let (subject, suffix) = rest
        .split_once(" with ")
        .map_or((rest, None), |(subject, suffix)| (subject, Some(suffix)));
    let (name, id) = subject.rsplit_once(" (")?;
    let id = id.strip_suffix(')')?;

    match suffix {
        None => Some(feedback(
            localization,
            language,
            "chat.command.place.success",
            &[("name", name), ("id", id)],
        )),
        Some(suffix) => {
            let count = suffix.split_whitespace().next()?;
            let key = if suffix.ends_with(" connected structures") {
                "chat.command.place.successConnected"
            } else if suffix.ends_with(" structures") {
                "chat.command.place.setSuccess"
            } else {
                return None;
            };
            Some(feedback(
                localization,
                language,
                key,
                &[("name", name), ("id", id), ("count", count)],
            ))
        }
    }
}

fn localize_locate_start(
    response: &str,
    localization: &UiLocalization,
    language: Language,
) -> Option<String> {
    let name = response.strip_prefix("Locating ")?.strip_suffix("...")?;
    Some(feedback(
        localization,
        language,
        "chat.command.locate.searching",
        &[("name", name)],
    ))
}

// Bevy systems expose their independent ECS inputs as function parameters.
#[allow(clippy::too_many_arguments)]
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
        let language = content.language.get();
        let localization = content.localization.as_ref();
        let parsed = parse_line(&submission.line);
        log_gameplay_event(format!(
            "command.submit line={:?} target={:?} parsed={:?} spectator={}",
            submission.line,
            submission.target,
            parsed,
            game_mode.is_spectator()
        ));
        if game_mode.is_spectator() && !matches!(parsed, ParsedLine::Say(_)) {
            chat.append_error(feedback(
                localization,
                language,
                "chat.command.spectatorUnavailable",
                &[],
            ));
            continue;
        }

        match parsed {
            ParsedLine::Say(text) => chat.append_text(format!("<{PLAYER_DISPLAY_NAME}>: {text}")),
            ParsedLine::Usage(usage) => chat.append_error(feedback(
                localization,
                language,
                "chat.command.usage",
                &[("usage", usage)],
            )),
            ParsedLine::Unknown(command) => chat.append_error(feedback(
                localization,
                language,
                "chat.command.unknown",
                &[("command", command)],
            )),
            ParsedLine::Spawn(id, meta_tag) => {
                let Some(definition) = content.definitions.get(id) else {
                    chat.append_error(feedback(
                        localization,
                        language,
                        "chat.command.spawn.unknownCreature",
                        &[("id", id)],
                    ));
                    continue;
                };
                let Some(position) = placement.player_block_position() else {
                    chat.append_error(feedback(
                        localization,
                        language,
                        "chat.command.spawn.playerUnavailable",
                        &[],
                    ));
                    continue;
                };
                let name = definition.name.text(language).to_owned();
                let position_text = format_position(position);

                if let Some(meta_tag) = meta_tag {
                    let mut meta_tags = EntityMetaTags::default();
                    if let Err(error) = meta_tags.add(meta_tag, None) {
                        chat.append_error(localize_internal_error(
                            &error,
                            localization,
                            language,
                        ));
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
                        language,
                        id,
                        feet,
                        meta_tags,
                    )
                    .is_err()
                    {
                        chat.append_error(feedback(
                            localization,
                            language,
                            "chat.command.spawn.failed",
                            &[("id", id)],
                        ));
                        continue;
                    }
                    chat.append_text(feedback(
                        localization,
                        language,
                        "chat.command.spawn.success",
                        &[("name", &name), ("position", &position_text)],
                    ));
                    continue;
                }

                let response = placement.spawn(&mut commands, id, &mut reserved);
                if response.starts_with("Spawned ") {
                    chat.append_text(feedback(
                        localization,
                        language,
                        "chat.command.spawn.success",
                        &[("name", &name), ("position", &position_text)],
                    ));
                } else {
                    chat.append_error(localize_internal_error(
                        &response,
                        localization,
                        language,
                    ));
                }
            }
            ParsedLine::Place(id, variation) => {
                let response = placement.place(id, variation, &reserved);
                if response.starts_with("Placed ") {
                    chat.append_text(localize_place_success(&response, localization, language)
                        .unwrap_or_else(|| {
                            feedback(localization, language, "chat.command.failed", &[])
                        }));
                } else {
                    chat.append_error(localize_internal_error(
                        &response,
                        localization,
                        language,
                    ));
                }
            }
            ParsedLine::Locate(kind, id, variation) => {
                let Some(player_block) = placement.player_block_position() else {
                    chat.append_error(feedback(
                        localization,
                        language,
                        "chat.command.locate.playerUnavailable",
                        &[],
                    ));
                    continue;
                };
                let response = locate.start(kind, id, variation, player_block);
                if response.starts_with("Locating ") {
                    chat.append_text(localize_locate_start(&response, localization, language)
                        .unwrap_or_else(|| {
                            feedback(localization, language, "chat.command.failed", &[])
                        }));
                } else {
                    chat.append_error(localize_internal_error(
                        &response,
                        localization,
                        language,
                    ));
                }
            }
            ParsedLine::Warp(target) => {
                warp.request(target);
                let position = format_position(target);
                chat.append_text(feedback(
                    localization,
                    language,
                    "chat.command.warp.start",
                    &[("position", &position)],
                ));
            }
            ParsedLine::Kill => {
                let Some(entity) = submission.target else {
                    chat.append_error(feedback(
                        localization,
                        language,
                        "chat.command.target.none",
                        &[],
                    ));
                    continue;
                };
                let Ok((name, transform, mut health, _, animation)) = content.targets.get_mut(entity)
                else {
                    chat.append_error(feedback(
                        localization,
                        language,
                        "chat.command.target.unavailable",
                        &[],
                    ));
                    continue;
                };
                let position = format_position(command_target_position(transform));
                let name = name.as_str().to_owned();
                let current = health.current();
                health.damage(current);
                if let Some(mut animation) = animation {
                    animation.trigger("death");
                }
                commands.entity(entity).insert(CreatureDeathTimer(Timer::from_seconds(
                    0.75,
                    TimerMode::Once,
                )));
                chat.append_text(feedback(
                    localization,
                    language,
                    "chat.command.kill.success",
                    &[("name", &name), ("position", &position)],
                ));
            }
            ParsedLine::Modify(action, tag, value) => {
                let Some(entity) = submission.target else {
                    chat.append_error(feedback(
                        localization,
                        language,
                        "chat.command.target.none",
                        &[],
                    ));
                    continue;
                };
                let Ok((name, transform, _, mut meta_tags, _)) = content.targets.get_mut(entity)
                else {
                    chat.append_error(feedback(
                        localization,
                        language,
                        "chat.command.target.unavailable",
                        &[],
                    ));
                    continue;
                };
                let before_tags = meta_tags.clone();
                let result = match action {
                    ModifyAction::Add => meta_tags.add(tag, value.map(str::to_owned)),
                    ModifyAction::Remove => meta_tags.remove(tag),
                    ModifyAction::Edit => meta_tags.edit(tag, value.map(str::to_owned)),
                };
                if let Err(error) = result {
                    chat.append_error(localize_internal_error(
                        &error,
                        localization,
                        language,
                    ));
                    continue;
                }
                log_gameplay_event(format!(
                    "entity.modify entity={:?} name={} action={:?} tag={} value={:?} before={:?} after={:?}",
                    entity,
                    name,
                    action,
                    tag,
                    value,
                    before_tags,
                    meta_tags
                ));
                let key = match action {
                    ModifyAction::Add => "chat.command.modify.addSuccess",
                    ModifyAction::Remove => "chat.command.modify.removeSuccess",
                    ModifyAction::Edit => "chat.command.modify.editSuccess",
                };
                let name = name.as_str().to_owned();
                let position = format_position(command_target_position(transform));
                chat.append_text(feedback(
                    localization,
                    language,
                    key,
                    &[("tag", tag), ("name", &name), ("position", &position)],
                ));
            }
        }
    }
}

fn localize_locate_feedback(
    mut chat: ResMut<ChatState>,
    language: Res<ActiveLanguage>,
    localization: Res<UiLocalization>,
) {
    let Some(last) = chat.history.back() else {
        return;
    };

    let replacement = match last {
        ChatMessage::Text(text) => {
            let Some((name, rest)) = text.split_once(" could not be found within ") else {
                return;
            };
            let Some(radius) = rest.strip_suffix(" blocks.") else {
                return;
            };
            Some(ChatMessage::Error(feedback(
                &localization,
                language.get(),
                "chat.command.locate.notFound",
                &[("name", name), ("radius", radius)],
            )))
        }
        ChatMessage::Located { prefix, target } => {
            let Some((name, _)) = prefix.split_once(" found at ") else {
                return;
            };
            let position = format_position(*target);
            let localized_prefix = feedback(
                &localization,
                language.get(),
                "chat.command.locate.found",
                &[("name", name), ("position", &position)],
            );
            if localized_prefix == *prefix {
                return;
            }
            Some(ChatMessage::Located {
                prefix: localized_prefix,
                target: *target,
            })
        }
        _ => None,
    };

    let Some(replacement) = replacement else {
        return;
    };
    let Some(last) = chat.history.back_mut() else {
        return;
    };
    *last = replacement;
    chat.since_last_message = 0.0;
    chat.revision = chat.revision.wrapping_add(1);
}

fn poll_warp_outcome(
    mut warp: ResMut<PendingWarp>,
    mut chat: ResMut<ChatState>,
    language: Res<ActiveLanguage>,
    localization: Res<UiLocalization>,
) {
    match warp.take_outcome() {
        Some(WarpOutcome::Succeeded(position)) => {
            let position = format_position(position);
            chat.append_text(feedback(
                &localization,
                language.get(),
                "chat.command.warp.success",
                &[("position", &position)],
            ));
        }
        Some(WarpOutcome::Failed) => chat.append_error(feedback(
            &localization,
            language.get(),
            "chat.command.warp.failed",
            &[],
        )),
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
    fn unknown_variation_feedback_is_parsed_structurally() {
        assert_eq!(
            parse_unknown_variation("Unknown variation 3 for asteria:test; expected 1..=2."),
            Some(("3", "asteria:test", "2"))
        );
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
