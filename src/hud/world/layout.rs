use bevy::prelude::*;

use crate::{app::game_state::GameState, ui::typography};

use super::{
    banner::world_banner, biome::BiomeHudText, coordinates::CoordinatesHudText,
    dimension::DimensionHudText,
};

pub(super) fn spawn_world_hud(mut commands: Commands) {
    let shadow = TextShadow {
        offset: Vec2::new(1.5, 1.5),
        color: Color::srgba(0.0, 0.0, 0.0, 0.92),
    };

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
                    shadow,
                    DimensionHudText,
                ));
                banner.spawn((
                    typography::hud_subheading(""),
                    TextLayout::justify(Justify::Center),
                    shadow,
                    BiomeHudText,
                ));
                banner.spawn((
                    typography::hud(""),
                    TextLayout::justify(Justify::Center),
                    shadow,
                    CoordinatesHudText,
                ));
            });
        });
}
