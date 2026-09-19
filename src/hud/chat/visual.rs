use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    text::{EditableText, FontWeight},
};

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    ui::{scrollbar, text_input, typography},
};

use super::{CHAT_TIMEOUT_SECS, ChatState, MAX_INPUT_CHARS, autocomplete::ChatAutocomplete};

// The chat grows naturally until fifteen lines of 17px HUD text at 22px
// line spacing, including wrapped visual lines. Beyond this, it scrolls.
const MAX_VISIBLE_LINES: f32 = 15.0;
const CHAT_LINE_HEIGHT: f32 = 22.0;
const MAX_HISTORY_HEIGHT: f32 = MAX_VISIBLE_LINES * CHAT_LINE_HEIGHT;
const MAX_SUGGESTIONS: usize = 7;

#[derive(Component)]
pub(super) struct ChatRoot;

#[derive(Component)]
pub(super) struct ChatHistory;

#[derive(Component)]
pub(super) struct ChatInputRoot;

#[derive(Component)]
pub(super) struct ChatDraft;

#[derive(Component)]
pub(super) struct ChatSuggestions;

pub(super) fn advance_chat_timeout(
    time: Res<Time>,
    pause: Res<State<PauseState>>,
    mut chat: ResMut<ChatState>,
) {
    if *pause.get() == PauseState::Running
        && !chat.history.is_empty()
        && chat.since_last_message < CHAT_TIMEOUT_SECS
    {
        chat.since_last_message += time.delta_secs();
    }
}

pub(super) fn spawn_chat_ui(mut commands: Commands) {
    commands
        .spawn((
            ChatRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(18),
                bottom: px(138),
                width: px(500),
                max_width: percent(92),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(1)),
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
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Stretch,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                let viewport = row
                    .spawn((
                        ChatHistory,
                        ScrollPosition::default(),
                        Node {
                            flex_grow: 1.0,
                            min_width: px(0),
                            max_height: px(MAX_HISTORY_HEIGHT),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                row.spawn(scrollbar::vertical_scrollbar(viewport));
            });

            root.spawn((
                ChatSuggestions,
                Node {
                    width: percent(100),
                    padding: UiRect::all(px(5)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.045, 0.04, 0.105, 0.96)),
                Visibility::Hidden,
                Pickable::IGNORE,
            ));

            root.spawn((
                ChatInputRoot,
                Node {
                    width: percent(100),
                    min_height: px(38),
                    min_width: px(0),
                    padding: UiRect::axes(px(text_input::INPUT_PADDING_X), px(7)),
                    border: UiRect::all(px(1)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                text_input::frame_surface(true),
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .with_children(|field| {
                field.spawn((typography::hud("> "), Pickable::IGNORE));
                field.spawn((
                    ChatDraft,
                    EditableText {
                        max_characters: Some(MAX_INPUT_CHARS),
                        ..default()
                    },
                    text_input::editor_style(17.0, FontWeight::NORMAL),
                    Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        min_height: px(21),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ));
            });
        });
}

pub(super) fn sync_chat_visibility(
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
    let entry_visibility = if running && chat.open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **entry != entry_visibility {
        **entry = entry_visibility;
    }
}

/// Updates only when the completion list or selection changes, not on every frame.
pub(super) fn render_autocomplete(
    mut commands: Commands,
    chat: Res<ChatState>,
    autocomplete: Res<ChatAutocomplete>,
    mut panel: Single<(Entity, &mut Visibility), With<ChatSuggestions>>,
    descendants: Query<&Children>,
    mut rendered_revision: Local<Option<u64>>,
) {
    let (panel_entity, visibility) = &mut *panel;
    let panel_entity = *panel_entity;
    let visible = chat.open && autocomplete.visible();
    let desired = if visible { Visibility::Inherited } else { Visibility::Hidden };
    if **visibility != desired {
        **visibility = desired;
    }
    if *rendered_revision == Some(autocomplete.revision) {
        return;
    }
    *rendered_revision = Some(autocomplete.revision);
    if let Ok(children) = descendants.get(panel_entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
    if !visible {
        return;
    }
    let count = autocomplete.suggestions.len();
    let first = autocomplete.selected.saturating_sub(MAX_SUGGESTIONS / 2)
        .min(count.saturating_sub(MAX_SUGGESTIONS));
    commands.entity(panel_entity).with_children(|list| {
        list.spawn((
            typography::caption("Use ↑ and ↓ to choose an option. Press Tab to complete it. Press Esc to close suggestions."),
            Node { width: percent(100), padding: UiRect::horizontal(px(5)), ..default() },
            Pickable::IGNORE,
        ));
        for (index, suggestion) in autocomplete.suggestions.iter().enumerate().skip(first).take(MAX_SUGGESTIONS) {
            let selected = index == autocomplete.selected;
            list.spawn((
                Node {
                    width: percent(100),
                    min_height: px(42),
                    padding: UiRect::axes(px(8), px(5)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    ..default()
                },
                BackgroundColor(if selected {
                    Color::srgba(0.31, 0.25, 0.53, 0.93)
                } else {
                    Color::NONE
                }),
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                // Selection is conveyed only by the highlighted background, never a chevron.
                row.spawn((typography::hud(suggestion.value.clone()), Pickable::IGNORE));
                row.spawn((
                    typography::caption(suggestion.description.clone()),
                    Node { width: percent(100), ..default() },
                    Pickable::IGNORE,
                ));
            });
        }
    });
}

pub(super) fn rebuild_chat_history(
    mut commands: Commands,
    chat: Res<ChatState>,
    history: Single<Entity, With<ChatHistory>>,
    descendants: Query<&Children>,
    mut rendered_revision: Local<u64>,
) {
    if chat.revision == *rendered_revision {
        return;
    }
    *rendered_revision = chat.revision;
    let root = *history;
    if let Ok(children) = descendants.get(root) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
    commands.entity(root).with_children(|list| {
        // Older messages above; new messages are appended at the bottom.
        // Wrapped lines contribute to the actual computed scroll height.
        for message in &chat.history {
            list.spawn((
                typography::hud(message.clone()),
                typography::tooltip_shadow(),
                Node {
                    width: percent(100),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
    });
}

pub(super) fn scroll_chat_history(
    mut wheel: MessageReader<MouseWheel>,
    chat: Res<ChatState>,
    mut history: Single<(&mut ScrollPosition, &ComputedNode), With<ChatHistory>>,
) {
    if !chat.open {
        wheel.clear();
        return;
    }
    let delta: f32 = wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => -event.y * CHAT_LINE_HEIGHT * 3.0,
            MouseScrollUnit::Pixel => -event.y,
        })
        .sum();
    if delta == 0.0 {
        return;
    }
    let (position, computed) = &mut *history;
    let maximum = ((computed.content_size().y - computed.size().y).max(0.0)
        * computed.inverse_scale_factor())
    .max(0.0);
    let target = (position.0.y + delta).clamp(0.0, maximum);
    if position.0.y != target {
        position.0.y = target;
    }
}

/// Last runs after UI layout: use measured, wrapped content height to follow
/// new messages rather than guessing the number of visual lines.
pub(super) fn scroll_chat_to_bottom(
    chat: Res<ChatState>,
    mut history: Single<(&mut ScrollPosition, &ComputedNode), With<ChatHistory>>,
    mut latest_revision: Local<u64>,
) {
    if *latest_revision == chat.revision {
        return;
    }
    *latest_revision = chat.revision;
    let (position, computed) = &mut *history;
    let bottom = ((computed.content_size().y - computed.size().y).max(0.0)
        * computed.inverse_scale_factor())
    .max(0.0);
    if position.0.y != bottom {
        position.0.y = bottom;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_history_height_is_fifteen_visual_lines() {
        assert_eq!(MAX_HISTORY_HEIGHT, 15.0 * CHAT_LINE_HEIGHT);
    }
}
