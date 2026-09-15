use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    content::{
        block::BlockRegistry,
        builtin_ids::{BRUSH_TOOL_ID, DYED_PROPERTY_ID},
        secondary_property::SecondaryPropertyRegistry,
    },
    gameplay::availability::world_interaction_available,
    localization::{ActiveLanguage, UiLocalization},
    player::{
        hotbar::{PlayerHotbar, PlayerHotbarSet},
        inventory::InventoryState,
    },
    targeting::block::TargetedBlock,
    tools::{BrushMode, BrushPaletteState},
    ui::{theme, typography, visibility::set_visibility},
};

use super::HudSettings;

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_crosshair)
            .add_systems(
                OnEnter(PauseState::Paused),
                set_visibility::<CrosshairRoot, false>,
            )
            .add_systems(
                OnEnter(SettingsState::Open),
                set_visibility::<CrosshairRoot, false>,
            )
            .add_systems(
                OnEnter(BrushPaletteState::Open),
                set_visibility::<CrosshairRoot, false>,
            )
            .add_systems(
                OnEnter(InventoryState::Open),
                set_visibility::<CrosshairRoot, false>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                update_action_hint
                    .after(PlayerHotbarSet::Selection)
                    .run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                set_visibility::<CrosshairRoot, true>.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(SettingsState::Closed),
                set_visibility::<CrosshairRoot, true>.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(InventoryState::Closed),
                set_visibility::<CrosshairRoot, true>.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Closed),
                set_visibility::<CrosshairRoot, true>.run_if(world_interaction_available),
            );
    }
}

#[derive(Component)]
struct CrosshairRoot;

#[derive(Component)]
struct ActionHint;

#[derive(SystemParam)]
struct ActionHintRuntime<'w> {
    hotbar: Res<'w, PlayerHotbar>,
    settings: Res<'w, HudSettings>,
    targeted: Res<'w, TargetedBlock>,
    brush_mode: Res<'w, BrushMode>,
}

#[derive(SystemParam)]
struct ActionHintContent<'w> {
    blocks: Res<'w, BlockRegistry>,
    secondary_properties: Res<'w, SecondaryPropertyRegistry>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
}

fn spawn_crosshair(mut commands: Commands) {
    commands
        .spawn((
            CrosshairRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Visible,
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(18),
                    height: px(18),
                    position_type: PositionType::Relative,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|crosshair| {
                crosshair.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(2),
                        top: px(8),
                        width: px(14),
                        height: px(2),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(theme::TEXT_PRIMARY.with_alpha(0.92)),
                ));
                crosshair.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(8),
                        top: px(2),
                        width: px(2),
                        height: px(14),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(theme::TEXT_PRIMARY.with_alpha(0.92)),
                ));
                crosshair.spawn((
                    typography::crosshair_hint(""),
                    TextLayout::justify(Justify::Center),
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(26),
                        left: px(-171),
                        width: px(360),
                        ..default()
                    },
                    ActionHint,
                    Visibility::Hidden,
                ));
            });
        });
}

fn update_action_hint(
    runtime: ActionHintRuntime,
    content: ActionHintContent,
    hint: Single<(&mut Text, &mut Visibility), With<ActionHint>>,
) {
    if !runtime.hotbar.is_changed()
        && !runtime.settings.is_changed()
        && !runtime.targeted.is_changed()
        && !runtime.brush_mode.is_changed()
        && !content.blocks.is_changed()
        && !content.secondary_properties.is_changed()
        && !content.localization.is_changed()
        && !content.language.is_changed()
    {
        return;
    }

    let language = content.language.get();
    let selected_item = runtime.hotbar.item_at(runtime.hotbar.selected_slot());
    let selected_block = selected_item.and_then(|id| content.blocks.get(id));

    let next_text = if !runtime.settings.display_tooltips() {
        None
    } else if let Some(hit) = runtime.targeted.0 {
        if selected_item == Some(BRUSH_TOOL_ID) {
            let can_dye = content.blocks.get(hit.block_id).is_some_and(|block| {
                block
                    .secondary_properties
                    .iter()
                    .any(|property| property == DYED_PROPERTY_ID)
            });
            if can_dye {
                Some(match runtime.brush_mode.dye_id() {
                    None => content
                        .localization
                        .text(language, "hud.brushClear")
                        .to_owned(),
                    Some(dye_id) => {
                        let color_name = content
                            .secondary_properties
                            .get(DYED_PROPERTY_ID, dye_id)
                            .map_or(dye_id, |definition| definition.name.text(language));
                        content
                            .localization
                            .text(language, "hud.brushPaint")
                            .replace("{color}", color_name)
                    }
                })
            } else {
                Some(
                    content
                        .localization
                        .text(language, "hud.breakBlock")
                        .to_owned(),
                )
            }
        } else if selected_block.is_some() {
            Some(
                content
                    .localization
                    .text(language, "hud.breakOrPlaceBlock")
                    .to_owned(),
            )
        } else {
            Some(
                content
                    .localization
                    .text(language, "hud.breakBlock")
                    .to_owned(),
            )
        }
    } else {
        selected_block
            .filter(|block| block.is_rotatable())
            .map(|_| {
                content
                    .localization
                    .text(language, "hud.rotateBlock")
                    .to_owned()
            })
    };

    let (mut text, mut visibility) = hint.into_inner();
    match next_text {
        Some(next_text) => {
            if text.0 != next_text {
                text.0 = next_text;
            }
            if *visibility != Visibility::Visible {
                *visibility = Visibility::Visible;
            }
        }
        None => {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}
