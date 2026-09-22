pub(super) mod portrait;

use bevy::prelude::*;

use crate::{
    app::{
        game_state::GameState,
        keybinds::{KeybindAction, Keybinds},
        pause_state::PauseState,
        settings_state::SettingsState,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::inventory::InventoryState,
    ui::typography,
};

use super::{
    HintKind, HudSettings,
    entity_card::{EntityCardSource, spawn_entity_card},
};

const PLAYER_HUD_MARGIN: f32 = 18.0;

pub struct PlayerHudPlugin;

impl Plugin for PlayerHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<portrait::PlayerPreviewImages>()
            .init_resource::<portrait::PlayerPreviewRenderState>()
            .add_systems(PostStartup, portrait::spawn_player_preview_renderer)
            .add_systems(
                Update,
                (
                    portrait::attach_player_preview_model,
                    portrait::render_player_preview,
                )
                    .chain(),
            )
            .add_systems(OnEnter(GameState::Gameplay), spawn_player_hud)
            .add_systems(
                Update,
                (sync_player_hud_visibility, sync_inventory_hint)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct PlayerHudRoot;


#[derive(Component)]
struct InventoryHint;

#[allow(clippy::too_many_arguments)]
fn spawn_player_hud(
    mut commands: Commands,
    settings: Res<HudSettings>,
    keybinds: Res<Keybinds>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    inventory_state: Res<State<InventoryState>>,
    pause_state: Res<State<PauseState>>,
    settings_state: Res<State<SettingsState>>,
    preview_images: Res<portrait::PlayerPreviewImages>,
    mut preview_render: ResMut<portrait::PlayerPreviewRenderState>,
) {
    let portrait_image = preview_images.portrait();
    preview_render.request_portrait();
    let hint_kind = inventory_hint_kind(*inventory_state.get());
    let hint_visibility = if settings.hint_enabled(hint_kind) {
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
            spawn_entity_card(
                root,
                EntityCardSource::LocalPlayer,
                portrait_image.clone(),
            );
            root.spawn((
                InventoryHint,
                typography::crosshair_hint(
                    localization
                        .text(language.get(), hint_key)
                        .replace("{inventory}", keybinds.label(KeybindAction::Inventory)),
                ),
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
    mut roots: Query<&mut Visibility, With<PlayerHudRoot>>,
) {
    let next = player_hud_visibility(*pause.get(), *settings.get());
    for mut visibility in &mut roots {
        if *visibility != next {
            *visibility = next;
        }
    }

}

fn sync_inventory_hint(
    settings: Res<HudSettings>,
    keybinds: Res<Keybinds>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    inventory_state: Res<State<InventoryState>>,
    hint: Single<(&mut Text, &mut Visibility), With<InventoryHint>>,
) {
    if !settings.is_changed()
        && !keybinds.is_changed()
        && !localization.is_changed()
        && !language.is_changed()
        && !inventory_state.is_changed()
    {
        return;
    }

    let (mut text, mut visibility) = hint.into_inner();
    let next_text = localization
        .text(language.get(), inventory_hint_key(*inventory_state.get()))
        .replace("{inventory}", keybinds.label(KeybindAction::Inventory));
    if text.0 != next_text {
        text.0 = next_text;
    }

    let next_visibility = if settings.hint_enabled(inventory_hint_kind(*inventory_state.get())) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != next_visibility {
        *visibility = next_visibility;
    }
}

const fn inventory_hint_kind(state: InventoryState) -> HintKind {
    match state {
        InventoryState::Closed => HintKind::OpenInventory,
        InventoryState::Open => HintKind::CloseInventory,
    }
}

const fn inventory_hint_key(state: InventoryState) -> &'static str {
    match state {
        InventoryState::Closed => "hud.openInventory",
        InventoryState::Open => "hud.closeInventory",
    }
}
