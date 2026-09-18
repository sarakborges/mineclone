use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    localization::{ActiveLanguage, UiLocalization},
    player::inventory::InventoryState,
    ui::typography,
};

use super::{
    HudSettings,
    entity_card::{EntityCardSource, spawn_entity_card},
};

const PLAYER_HUD_MARGIN: f32 = 18.0;

pub struct PlayerHudPlugin;

impl Plugin for PlayerHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_player_hud)
            .add_systems(
                Update,
                (sync_player_hud_visibility, sync_inventory_hint)
                    .run_if(in_state(GameState::Gameplay)),
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
    pause_state: Res<State<PauseState>>,
    settings_state: Res<State<SettingsState>>,
) {
    let hint_visibility = if settings.display_tooltips() {
        Visibility::Inherited
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
            player_hud_visibility(*pause_state.get(), *settings_state.get()),
            GlobalZIndex(10),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            spawn_entity_card(root, EntityCardSource::LocalPlayer, None);
            root.spawn((
                InventoryHint,
                typography::crosshair_hint(localization.text(language.get(), hint_key).to_owned()),
                hint_visibility,
                Pickable::IGNORE,
            ));
        });
}

fn player_hud_visibility(pause: PauseState, settings: SettingsState) -> Visibility {
    if pause == PauseState::Paused || settings == SettingsState::Open {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

fn sync_player_hud_visibility(
    pause: Res<State<PauseState>>,
    settings: Res<State<SettingsState>>,
    mut root: Single<&mut Visibility, With<PlayerHudRoot>>,
) {
    if !pause.is_changed() && !settings.is_changed() {
        return;
    }
    let next = player_hud_visibility(*pause.get(), *settings.get());
    if **root != next {
        **root = next;
    }
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
        Visibility::Inherited
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
