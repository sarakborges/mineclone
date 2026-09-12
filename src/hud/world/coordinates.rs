use bevy::prelude::*;

use crate::{
    localization::{ActiveLanguage, UiLocalization},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
};

#[derive(Component)]
pub(super) struct CoordinatesHudText;

pub(super) fn update_coordinates_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut coordinates_text: Single<&mut Text, With<CoordinatesHudText>>,
) {
    let position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let block_position = position.floor().as_ivec3();
    let next_text = localization
        .text(language.get(), "hud.coordinates")
        .replace("{x}", &block_position.x.to_string())
        .replace("{z}", &block_position.z.to_string())
        .replace("{y}", &block_position.y.to_string());

    if coordinates_text.0 != next_text {
        coordinates_text.0 = next_text;
    }
}
