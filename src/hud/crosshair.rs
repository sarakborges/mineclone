use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    ui::theme,
};

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_crosshair);
    }
}

fn spawn_crosshair(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
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
                    BoxShadow(vec![ShadowStyle {
                        color: theme::CYAN_GLOW,
                        x_offset: px(0),
                        y_offset: px(0),
                        spread_radius: px(0),
                        blur_radius: px(8),
                    }]),
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
                    BoxShadow(vec![ShadowStyle {
                        color: theme::CYAN_GLOW,
                        x_offset: px(0),
                        y_offset: px(0),
                        spread_radius: px(0),
                        blur_radius: px(8),
                    }]),
                ));
            });
        });
}
