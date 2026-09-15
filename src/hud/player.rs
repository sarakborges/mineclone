use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    localization::{ActiveLanguage, UiLocalization},
    player::inventory::InventoryState,
    ui::{surface, theme, typography, visibility::set_visibility},
};

use super::HudSettings;

const PLAYER_HUD_MARGIN: f32 = 18.0;
const AVATAR_SIZE: f32 = 64.0;
const PLAYER_INFO_WIDTH: f32 = 180.0;
const HEALTH_BAR_HEIGHT: f32 = 22.0;
const PLACEHOLDER_HEALTH_PERCENT: f32 = 50.0;
const HEALTH_FILL_COLOR: Color = Color::srgba(0.78, 0.16, 0.25, 0.94);

pub struct PlayerHudPlugin;

impl Plugin for PlayerHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_player_hud)
            .add_systems(
                OnEnter(PauseState::Paused),
                set_visibility::<PlayerHudRoot, false>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                set_visibility::<PlayerHudRoot, true>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                sync_inventory_hint.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct PlayerHudRoot;

#[derive(Component)]
struct InventoryHint;

fn spawn_player_hud(
    mut commands: Commands,
    settings: Res<HudSettings>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    inventory_state: Res<State<InventoryState>>,
) {
    let (avatar_background, avatar_border) = surface::hud_control_static(false);
    let hint_visibility = if settings.display_tooltips() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let hint_key = inventory_hint_key(*inventory_state.get());

    commands
        .spawn((
            PlayerHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(PLAYER_HUD_MARGIN),
                bottom: px(PLAYER_HUD_MARGIN),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: px(8),
                ..default()
            },
            GlobalZIndex(10),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(12),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    Node {
                        width: px(AVATAR_SIZE),
                        height: px(AVATAR_SIZE),
                        min_width: px(AVATAR_SIZE),
                        min_height: px(AVATAR_SIZE),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(4)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(avatar_background),
                    BorderColor::all(avatar_border),
                    Pickable::IGNORE,
                ))
                .with_children(|avatar| {
                    avatar.spawn((
                        typography::hud_subheading("?"),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                });

                row.spawn((
                    Node {
                        width: px(PLAYER_INFO_WIDTH),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        row_gap: px(8),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|info| {
                    info.spawn((typography::hud("Yogg'Sara"), Pickable::IGNORE));

                    info.spawn((
                        Node {
                            position_type: PositionType::Relative,
                            width: percent(100),
                            height: px(HEALTH_BAR_HEIGHT),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(theme::SLIDER_TRACK),
                        BorderColor::all(surface::HUD_BORDER_COLOR),
                        Pickable::IGNORE,
                    ))
                    .with_children(|health| {
                        health.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(0),
                                top: px(0),
                                width: percent(PLACEHOLDER_HEALTH_PERCENT),
                                height: percent(100),
                                border_radius: BorderRadius::all(px(3)),
                                ..default()
                            },
                            BackgroundColor(HEALTH_FILL_COLOR),
                            Pickable::IGNORE,
                        ));

                        health
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(0),
                                    top: px(0),
                                    width: percent(100),
                                    height: percent(100),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|label| {
                                label.spawn((
                                    typography::inventory_category("50 / 100"),
                                    typography::tooltip_shadow(),
                                    TextLayout::justify(Justify::Center),
                                    Pickable::IGNORE,
                                ));
                            });
                    });
                });
            });

            root.spawn((
                InventoryHint,
                typography::crosshair_hint(localization.text(language.get(), hint_key).to_owned()),
                hint_visibility,
                Pickable::IGNORE,
            ));
        });
}

fn sync_inventory_hint(
    settings: Res<HudSettings>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    inventory_state: Res<State<InventoryState>>,
    hint: Single<(&mut Text, &mut Visibility), With<InventoryHint>>,
) {
    if !settings.is_changed()
        && !localization.is_changed()
        && !language.is_changed()
        && !inventory_state.is_changed()
    {
        return;
    }

    let (mut text, mut visibility) = hint.into_inner();
    let next_text = localization
        .text(language.get(), inventory_hint_key(*inventory_state.get()))
        .to_owned();
    if text.0 != next_text {
        text.0 = next_text;
    }

    let next_visibility = if settings.display_tooltips() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *visibility != next_visibility {
        *visibility = next_visibility;
    }
}

const fn inventory_hint_key(state: InventoryState) -> &'static str {
    match state {
        InventoryState::Closed => "hud.openInventory",
        InventoryState::Open => "hud.closeInventory",
    }
}
