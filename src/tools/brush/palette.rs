use std::cmp::Ordering;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        builtin_ids::DYED_PROPERTY_ID,
        secondary_property::{SecondaryPropertyDefinition, SecondaryPropertyRegistry},
    },
    localization::{ActiveLanguage, UiLocalization},
    ui::{theme, typography},
};

use super::{BrushMode, BrushPaletteState, BrushSelection};

const PALETTE_COLUMNS: usize = 9;
const SWATCH_SIZE: f32 = 30.0;
const SWATCH_GAP: f32 = 4.0;
const PALETTE_WIDTH: f32 =
    PALETTE_COLUMNS as f32 * SWATCH_SIZE + (PALETTE_COLUMNS - 1) as f32 * SWATCH_GAP;
const ACHROMATIC_SATURATION_EPSILON: f32 = 0.001;

#[derive(Component)]
struct BrushPaletteRoot;

#[derive(Component, Clone)]
pub(super) struct BrushPaletteChoice {
    selection: BrushSelection,
}

pub(super) fn spawn_brush_palette(
    mut commands: Commands,
    properties: Res<SecondaryPropertyRegistry>,
    mode: Res<BrushMode>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    let language = language.get();
    let mut colors = properties.iter(DYED_PROPERTY_ID).collect::<Vec<_>>();
    colors.sort_by(compare_palette_colors);

    commands
        .spawn((
            BrushPaletteRoot,
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
            GlobalZIndex(120),
            Pickable::IGNORE,
            DespawnOnExit(BrushPaletteState::Open),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(8),
                    padding: UiRect::all(px(10)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(9)),
                    ..default()
                },
                BackgroundColor(theme::FROSTED_SURFACE),
                theme::frosted_surface_gradient(),
                BorderColor::all(Color::srgba(0.70, 0.72, 0.92, 0.20)),
                Pickable::IGNORE,
            ))
            .with_children(|panel| {
                panel.spawn((
                    typography::caption(localization.text(language, "brush.palette.title")),
                    Pickable::IGNORE,
                ));

                let clear_selected = mode.dye_id().is_none();
                panel
                    .spawn((
                        Button,
                        BrushPaletteChoice {
                            selection: BrushSelection::Clear,
                        },
                        Node {
                            width: px(PALETTE_WIDTH),
                            height: px(30),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(theme::HUD_SURFACE),
                        BorderColor::all(if clear_selected {
                            theme::TEXT_PRIMARY
                        } else {
                            Color::srgba(0.70, 0.72, 0.82, 0.28)
                        }),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            typography::caption(localization.text(language, "brush.palette.clear")),
                            Pickable::IGNORE,
                        ));
                    });

                panel
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(SWATCH_GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|grid| {
                        for row in colors.chunks(PALETTE_COLUMNS) {
                            grid.spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: px(SWATCH_GAP),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|row_node| {
                                for color in row {
                                    let selected = mode.dye_id() == Some(color.id.as_str());
                                    row_node.spawn((
                                        Button,
                                        BrushPaletteChoice {
                                            selection: BrushSelection::Dye(color.id.clone()),
                                        },
                                        Node {
                                            width: px(SWATCH_SIZE),
                                            height: px(SWATCH_SIZE),
                                            border: UiRect::all(px(2)),
                                            border_radius: BorderRadius::all(px(5)),
                                            ..default()
                                        },
                                        BackgroundColor(color.color.to_color()),
                                        BorderColor::all(if selected {
                                            theme::TEXT_PRIMARY
                                        } else {
                                            Color::srgba(0.70, 0.72, 0.82, 0.32)
                                        }),
                                    ));
                                }
                            });
                        }
                    });
            });
        });
}

fn compare_palette_colors(
    left: &&SecondaryPropertyDefinition,
    right: &&SecondaryPropertyDefinition,
) -> Ordering {
    let left_achromatic = left.color.saturation <= ACHROMATIC_SATURATION_EPSILON;
    let right_achromatic = right.color.saturation <= ACHROMATIC_SATURATION_EPSILON;

    left_achromatic
        .cmp(&right_achromatic)
        .then_with(|| {
            if left_achromatic && right_achromatic {
                right.color.intensity.total_cmp(&left.color.intensity)
            } else {
                left.color
                    .hue
                    .rem_euclid(360.0)
                    .total_cmp(&right.color.hue.rem_euclid(360.0))
            }
        })
        .then_with(|| right.color.saturation.total_cmp(&left.color.saturation))
        .then_with(|| right.color.intensity.total_cmp(&left.color.intensity))
        .then_with(|| left.id.cmp(&right.id))
}

pub(super) fn handle_palette_selection(
    choices: Query<(&Interaction, &BrushPaletteChoice), Changed<Interaction>>,
    mut mode: ResMut<BrushMode>,
    mut next_palette: ResMut<NextState<BrushPaletteState>>,
) {
    for (interaction, choice) in &choices {
        if *interaction != Interaction::Pressed {
            continue;
        }

        mode.selection = choice.selection.clone();
        next_palette.set(BrushPaletteState::Closed);
        break;
    }
}
