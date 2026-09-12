use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    content::{
        block::BlockRegistry, builtin_ids::BRUSH_TOOL_ID,
        secondary_property::SecondaryPropertyRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::{
        hotbar::{PlayerHotbar, PlayerHotbarSet},
        inventory::InventoryState,
    },
    targeting::block::TargetedBlock,
    tools::{BrushMode, BrushPaletteState, DYED_PROPERTY_ID},
    ui::{theme, typography},
};

use super::HudSettings;

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_crosshair)
            .add_systems(OnEnter(PauseState::Paused), hide_crosshair)
            .add_systems(OnEnter(SettingsState::Open), hide_crosshair)
            .add_systems(OnEnter(BrushPaletteState::Open), hide_crosshair)
            .add_systems(
                OnEnter(InventoryState::Open),
                hide_crosshair.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                update_action_hint
                    .after(PlayerHotbarSet::Selection)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(InventoryState::Closed))
                    .run_if(in_state(BrushPaletteState::Closed)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                show_crosshair
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(SettingsState::Closed))
                    .run_if(in_state(InventoryState::Closed))
                    .run_if(in_state(BrushPaletteState::Closed)),
            )
            .add_systems(
                OnEnter(SettingsState::Closed),
                show_crosshair
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(InventoryState::Closed))
                    .run_if(in_state(BrushPaletteState::Closed)),
            )
            .add_systems(
                OnEnter(InventoryState::Closed),
                show_crosshair
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(SettingsState::Closed))
                    .run_if(in_state(BrushPaletteState::Closed)),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Closed),
                show_crosshair
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(SettingsState::Closed))
                    .run_if(in_state(InventoryState::Closed)),
            );
    }
}

#[derive(Component)]
struct CrosshairRoot;

#[derive(Component)]
struct ActionHint;

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
                    typography::opaque_caption(""),
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

#[allow(clippy::too_many_arguments)]
fn update_action_hint(
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    settings: Res<HudSettings>,
    targeted: Res<TargetedBlock>,
    brush_mode: Res<BrushMode>,
    secondary_properties: Res<SecondaryPropertyRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    hint: Single<(&mut Text, &mut Visibility), With<ActionHint>>,
) {
    let language = language.get();
    let selected_item = hotbar.item_at(hotbar.selected_slot());

    let next_text = if !settings.display_tooltips() {
        None
    } else {
        selected_item
            .and_then(|id| blocks.get(id))
            .filter(|block| block.is_rotatable())
            .map(|_| localization.text(language, "hud.rotateBlock").to_owned())
            .or_else(|| {
                if selected_item != Some(BRUSH_TOOL_ID) {
                    return None;
                }

                let hit = targeted.0?;
                let block = blocks.get(hit.block_id)?;
                if !block
                    .secondary_properties
                    .iter()
                    .any(|property| property == DYED_PROPERTY_ID)
                {
                    return None;
                }

                Some(match brush_mode.dye_id() {
                    None => localization.text(language, "hud.brushClear").to_owned(),
                    Some(dye_id) => {
                        let color_name = secondary_properties
                            .get(DYED_PROPERTY_ID, dye_id)
                            .map_or(dye_id, |definition| definition.name.text(language));
                        localization
                            .text(language, "hud.brushPaint")
                            .replace("{color}", color_name)
                    }
                })
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

fn hide_crosshair(mut roots: Query<&mut Visibility, With<CrosshairRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Hidden;
    }
}

fn show_crosshair(mut roots: Query<&mut Visibility, With<CrosshairRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Visible;
    }
}
