use std::collections::VecDeque;

use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    content::creature::CreatureRegistry,
    creatures::spawn_creature_at,
    localization::ActiveLanguage,
    player::{camera::{GameplayCamera, look::MouseLookInputState}, inventory::InventoryState},
    tools::BrushPaletteState,
    ui::{text_input::{TextInputState, select_all_pressed}, typography},
    voxel::world::VoxelWorld,
};

const PLAYER_DISPLAY_NAME: &str = "Yogg'Sara";
const HISTORY_CAPACITY: usize = 64;
const VISIBLE_MESSAGES: usize = 10;
const CHAT_TIMEOUT_SECS: f32 = 10.0;
const MAX_INPUT_CHARS: usize = 256;

/// Gameplay input is gated by this state; the history is independent of UI nodes.
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
        self.history.push_front(text);
        self.history.truncate(HISTORY_CAPACITY);
        self.since_last_message = 0.0;
        self.revision = self.revision.wrapping_add(1);
    }

    fn visible(&self) -> bool {
        self.open || (!self.history.is_empty() && self.since_last_message < CHAT_TIMEOUT_SECS)
    }
}

#[derive(Message)]
struct ChatSubmission(String);

#[derive(Component)]
struct ChatRoot;

#[derive(Component)]
struct ChatHistory;

#[derive(Component)]
struct ChatInputRoot;

#[derive(Component)]
struct ChatDraft;

pub(super) struct ChatHudPlugin;

impl Plugin for ChatHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatState>()
            .add_message::<ChatSubmission>()
            .add_systems(OnEnter(GameState::Gameplay), (reset_chat, spawn_chat_ui).chain())
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
                    sync_chat_visibility,
                    sync_chat_draft,
                    rebuild_chat_history,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn reset_chat(mut chat: ResMut<ChatState>) {
    *chat = ChatState::default();
}

fn close_chat_on_pause(mut chat: ResMut<ChatState>) {
    chat.close();
}

fn handle_chat_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut submissions: MessageWriter<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    pause: Res<State<PauseState>>,
    settings: Res<State<SettingsState>>,
    inventory: Res<State<InventoryState>>,
    brush_palette: Res<State<BrushPaletteState>>,
    window: Single<&Window>,
    mut cursor: Single<&mut CursorOptions>,
    mut mouse_look: ResMut<MouseLookInputState>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        chat.escape_consumed = false;
    }

    if chat.open {
        if *pause.get() != PauseState::Running {
            chat.close();
            keyboard.clear();
            return;
        }
        if keys.just_pressed(KeyCode::Escape) {
            chat.close();
            chat.escape_consumed = true;
            restore_game_cursor(window.focused, &mut cursor, &mut mouse_look);
            keyboard.clear();
            return;
        }
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
            let line = chat.draft.text().trim().to_owned();
            chat.close();
            restore_game_cursor(window.focused, &mut cursor, &mut mouse_look);
            if !line.is_empty() {
                submissions.write(ChatSubmission(line));
            }
            keyboard.clear();
            return;
        }
        if select_all_pressed(&keys) {
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
                    let limited: String = text.chars().filter(|ch| !ch.is_control()).take(remaining).collect();
                    chat.draft.push_text(&limited);
                }
            }
        }
        return;
    }

    keyboard.clear();
    let can_open = *pause.get() == PauseState::Running
        && *settings.get() == SettingsState::Closed
        && *inventory.get() == InventoryState::Closed
        && *brush_palette.get() == BrushPaletteState::Closed;
    if !can_open || !keys.just_pressed(KeyCode::KeyT) {
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
    cursor.grab_mode = if focused { CursorGrabMode::Locked } else { CursorGrabMode::None };
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

fn interpret_chat_submissions(
    mut submissions: MessageReader<ChatSubmission>,
    mut chat: ResMut<ChatState>,
    mut commands: Commands,
    definitions: Res<CreatureRegistry>,
    assets: Res<AssetServer>,
    world: Res<VoxelWorld>,
    language: Res<ActiveLanguage>,
    players: Query<&Transform, With<GameplayCamera>>,
) {
    for submission in submissions.read() {
        let result = match parse_line(&submission.0) {
            ParsedLine::Say(text) => format!("<{PLAYER_DISPLAY_NAME}>: {text}"),
            ParsedLine::Usage => "Usage: /spawn_creature <id>".to_owned(),
            ParsedLine::Unknown(command) => format!("Unknown command: {command}"),
            ParsedLine::SpawnCreature(id) => {
                if let Some(eye) = players.iter().next().map(|player| player.translation) {
                    match spawn_creature_at(
                        &mut commands,
                        &definitions,
                        &assets,
                        &world,
                        language.get(),
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
        chat.append(result);
    }
}

fn advance_chat_timeout(time: Res<Time>, mut chat: ResMut<ChatState>) {
    if !chat.history.is_empty() && chat.since_last_message < CHAT_TIMEOUT_SECS {
        chat.since_last_message += time.delta_secs();
    }
}

fn spawn_chat_ui(mut commands: Commands) {
    commands
        .spawn((
            ChatRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(18),
                bottom: px(138),
                width: px(500),
                height: px(246),
                max_width: percent(92),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.03, 0.075, 0.23)),
            BorderColor::all(Color::srgba(0.74, 0.70, 0.92, 0.20)),
            Visibility::Hidden,
            GlobalZIndex(20),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                ChatHistory,
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    row_gap: px(3),
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            root.spawn((
                ChatInputRoot,
                Node {
                    width: percent(100),
                    min_height: px(34),
                    padding: UiRect::axes(px(10), px(6)),
                    border_radius: BorderRadius::all(px(4)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.035, 0.09, 0.76)),
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .with_children(|field| {
                field.spawn((
                    ChatDraft,
                    typography::hud(""),
                    typography::tooltip_shadow(),
                    Node { width: percent(100), ..default() },
                    Pickable::IGNORE,
                ));
            });
        });
}

fn sync_chat_visibility(
    chat: Res<ChatState>,
    pause: Res<State<PauseState>>,
    mut root: Single<&mut Visibility, (With<ChatRoot>, Without<ChatInputRoot>)>,
    mut entry: Single<&mut Visibility, (With<ChatInputRoot>, Without<ChatRoot>)>,
) {
    let running = *pause.get() == PauseState::Running;
    let root_visibility = if running && chat.visible() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **root != root_visibility {
        **root = root_visibility;
    }
    let input_visibility = if running && chat.open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **entry != input_visibility {
        **entry = input_visibility;
    }
}

fn sync_chat_draft(chat: Res<ChatState>, mut text: Single<&mut Text, With<ChatDraft>>) {
    let next = if chat.open {
        format!("> {}▏", chat.draft.text())
    } else {
        String::new()
    };
    if text.0 != next {
        text.0 = next;
    }
}

fn rebuild_chat_history(
    mut commands: Commands,
    chat: Res<ChatState>,
    history: Single<(Entity, &Children), With<ChatHistory>>,
    mut rendered_revision: Local<u64>,
) {
    if chat.revision == *rendered_revision {
        return;
    }
    *rendered_revision = chat.revision;
    let (root, children) = history.into_inner();
    for child in children.iter() {
        commands.entity(child).despawn();
    }
    commands.entity(root).with_children(|list| {
        // The newest message is first in the visual column; the column itself
        // is bottom-anchored so its content grows upward.
        for message in chat.history.iter().take(VISIBLE_MESSAGES) {
            list.spawn((
                typography::hud(message.clone()),
                typography::tooltip_shadow(),
                Node { width: percent(100), ..default() },
                Pickable::IGNORE,
            ));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_distinguished_from_plain_messages() {
        assert_eq!(parse_line("hello"), ParsedLine::Say("hello"));
        assert_eq!(parse_line(" /spawn_creature asteria:meadow_slime "),
            ParsedLine::SpawnCreature("asteria:meadow_slime"));
        assert_eq!(parse_line("/spawn_creature"), ParsedLine::Usage);
        assert_eq!(parse_line("/spawn_creature slime extra"), ParsedLine::Usage);
        assert_eq!(parse_line("/unknown"), ParsedLine::Unknown("/unknown"));
    }

    #[test]
    fn chat_history_is_newest_first_and_expires_when_closed() {
        let mut state = ChatState::default();
        state.append("old".to_owned());
        state.append("new".to_owned());
        assert_eq!(state.history.front().map(String::as_str), Some("new"));
        state.since_last_message = CHAT_TIMEOUT_SECS;
        assert!(!state.visible());
        state.open = true;
        assert!(state.visible());
    }
}
