use bevy::prelude::*;

use crate::{app::game_state::GameState, ui::typography};

use super::{
    banner::world_banner,
    compass::{
        COMPASS_MARKER_COUNT, COMPASS_MARKER_STEP_DEGREES, COMPASS_MARKER_WIDTH, CompassMarker,
        direction_label,
    },
    coordinates::CoordinatesHudText,
    named::{BiomeHudText, DimensionHudText},
};

pub(super) fn spawn_world_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(14),
                left: px(0),
                width: percent(100),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn(world_banner()).with_children(|banner| {
                banner.spawn((
                    typography::hud_heading(""),
                    TextLayout::justify(Justify::Center),
                    typography::tooltip_shadow(),
                    DimensionHudText,
                ));
                banner.spawn((
                    typography::hud_subheading(""),
                    TextLayout::justify(Justify::Center),
                    typography::tooltip_shadow(),
                    BiomeHudText,
                ));
                banner.spawn((
                    typography::hud(""),
                    TextLayout::justify(Justify::Center),
                    typography::tooltip_shadow(),
                    CoordinatesHudText,
                ));
                banner
                    .spawn(Node {
                        width: px(360),
                        max_width: percent(100),
                        height: px(30),
                        margin: UiRect {
                            top: px(2),
                            ..default()
                        },
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip_x(),
                        ..default()
                    })
                    .with_children(|compass| {
                        compass.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(0),
                                bottom: px(0),
                                width: percent(100),
                                height: px(1),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                        ));

                        for step in 0..COMPASS_MARKER_COUNT {
                            let heading_degrees = step as f32 * COMPASS_MARKER_STEP_DEGREES;
                            let label = direction_label(step);
                            compass
                                .spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: percent(50),
                                        top: px(0),
                                        width: px(COMPASS_MARKER_WIDTH),
                                        height: percent(100),
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::FlexEnd,
                                        row_gap: px(1),
                                        ..default()
                                    },
                                    UiTransform::IDENTITY,
                                    CompassMarker::new(heading_degrees),
                                ))
                                .with_children(|marker| {
                                    if let Some(label) = label {
                                        marker.spawn((
                                            typography::hud(label),
                                            TextLayout::justify(Justify::Center),
                                            typography::tooltip_shadow(),
                                        ));
                                    }
                                    marker.spawn((
                                        Node {
                                            width: px(1),
                                            height: px(if label.is_some() { 7 } else { 4 }),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(
                                            1.0,
                                            1.0,
                                            1.0,
                                            if label.is_some() { 0.7 } else { 0.35 },
                                        )),
                                    ));
                                });
                        }

                        compass.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: percent(50),
                                top: px(0),
                                width: px(2),
                                height: px(6),
                                ..default()
                            },
                            UiTransform::from_xy(px(-1), px(0)),
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                        ));
                    });
            });
        });
}
