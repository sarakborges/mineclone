use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    ui::{scrollbar, typography},
};

use super::{CHAT_TIMEOUT_SECS, ChatState};

// The chat grows naturally until fifteen lines of 17px HUD text at 22px
// line spacing, including wrapped visual lines. Beyond this, it scrolls.
const MAX_VISIBLE_LINES: f32 = 15.0;
const CHAT_LINE_HEIGHT: f32 = 22.0;
const MAX_HISTORY_HEIGHT: f32 = MAX_VISIBLE_LINES * CHAT_LINE_HEIGHT;

#[derive(Component)]
pub(super) struct ChatRoot;

#[derive(Component)]
pub(super) struct ChatHistory;

#[derive(Component)]
pub(super) struct ChatInputRoot;

#[derive(Component)]
pub(super) struct ChatDraft;

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
                ChatInputRoot,
                Node {
                    width: percent(100),
                    min_height: px(34),
                    padding: UiRect {
                        left: px(10),
                        right: px(10),
                        top: px(6),
                        bottom: px(6),
                    },
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
                    Node {
                        width: percent(100),
                        ..default()
                    },
                    Pickable::IGNORE,
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

pub(super) fn sync_chat_draft(chat: Res<ChatState>, mut text: Single<&mut Text, With<ChatDraft>>) {
    let next = if chat.open {
        format!("> {}▏", chat.draft.text())
    } else {
        String::new()
    };
    if text.0 != next {
        text.0 = next;
    }
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
