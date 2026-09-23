pub(super) mod portrait;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{
        game_state::GameState,
        keybinds::{KeybindAction, Keybinds},
        pause_state::PauseState,
        settings_state::SettingsState,
    },
    gameplay::modal::GameplayModalState,
    localization::{ActiveLanguage, UiLocalization},
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
        app.add_systems(
                OnEnter(GameState::Gameplay),
                (portrait::spawn_player_preview_cameras, spawn_player_hud).chain(),
            )
            .add_systems(
                Update,
                portrait::sync_player_preview_cameras.run_if(in_state(GameState::Gameplay)),
            )
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

#[derive(SystemParam)]
struct InventoryHintContent<'w> {
    settings: Res<'w, HudSettings>,
    keybinds: Res<'w, Keybinds>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
    modal: Res<'w, State<GameplayModalState>>,
}

impl InventoryHintContent<'_> {
    fn kind(&self) -> HintKind {
        inventory_hint_kind(*self.modal.get())
    }

    fn text(&self) -> String {
        self.localization
            .text(self.language.get(), inventory_hint_key(*self.modal.get()))
            .replace(
                "{inventory}",
                self.keybinds.label(KeybindAction::Inventory),
            )
    }

    fn visibility(&self) -> Visibility {
        if self.settings.hint_enabled(self.kind()) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    }

    fn is_changed(&self) -> bool {
        self.settings.is_changed()
            || self.keybinds.is_changed()
            || self.localization.is_changed()
            || self.language.is_changed()
            || self.modal.is_changed()
    }
}

fn spawn_player_hud(
    mut commands: Commands,
    hint: InventoryHintContent,
    pause_state: Res<State<PauseState>>,
    settings_state: Res<State<SettingsState>>,
) {
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
                typography::crosshair_hint(hint.text()),
                hint.visibility(),
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
    content: InventoryHintContent,
    hint: Single<(&mut Text, &mut Visibility), With<InventoryHint>>,
) {
    if !content.is_changed() {
        return;
    }

    let (mut text, mut visibility) = hint.into_inner();
    let next_text = content.text();
    if text.0 != next_text {
        text.0 = next_text;
    }

    let next_visibility = content.visibility();
    if *visibility != next_visibility {
        *visibility = next_visibility;
    }
}

const fn inventory_hint_kind(state: GameplayModalState) -> HintKind {
    if matches!(state, GameplayModalState::CharacterInfo) {
        HintKind::CloseInventory
    } else {
        HintKind::OpenInventory
    }
}

const fn inventory_hint_key(state: GameplayModalState) -> &'static str {
    if matches!(state, GameplayModalState::CharacterInfo) {
        "hud.closeInventory"
    } else {
        "hud.openInventory"
    }
}
