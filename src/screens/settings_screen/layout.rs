use bevy::prelude::*;

use crate::{
    app::settings_state::SettingsState,
    ui::{
        button::menu_button,
        cosmic_background::{self, STAR_FIELD},
        theme, typography,
    },
    world::render_distance::RenderDistanceSettings,
};

use super::{navigation::SettingsBackButton, render_distance_section::render_distance_section};

const CONTENT_WIDTH: f32 = 760.0;
const HEADER_HEIGHT: f32 = 116.0;
const FOOTER_HEIGHT: f32 = 104.0;

pub fn spawn_settings_screen(mut commands: Commands, render_distance: Res<RenderDistanceSettings>) {
    commands
        .spawn((
            DespawnOnExit(SettingsState::Open),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
            GlobalZIndex(500),
        ))
        .with_children(|root| {
            for &(left, top, size, phase, speed, red, green, blue, base_alpha) in STAR_FIELD {
                root.spawn(cosmic_background::star(
                    left, top, size, phase, speed, red, green, blue, base_alpha,
                ));
            }

            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(HEADER_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn(typography::title("SETTINGS"));
            });

            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: px(HEADER_HEIGHT),
                bottom: px(FOOTER_HEIGHT),
                left: px(0),
                right: px(0),
                padding: UiRect::axes(px(32), px(24)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    width: px(CONTENT_WIDTH),
                    max_width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(24),
                    ..default()
                })
                .with_children(|sections| {
                    sections.spawn(render_distance_section(render_distance.chunks()));
                });
            });

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                bottom: px(0),
                height: px(FOOTER_HEIGHT),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|footer| {
                footer.spawn(menu_button("Back", SettingsBackButton));
            });
        });
}
