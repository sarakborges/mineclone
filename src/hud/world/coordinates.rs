use bevy::prelude::*;

use crate::{
    localization::{ActiveLanguage, Language, UiLocalization},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera},
};

#[derive(Component)]
pub(super) struct CoordinatesHudText;

pub(super) fn update_coordinates_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut coordinates_text: Single<&mut Text, With<CoordinatesHudText>>,
    mut cached: Local<Option<(IVec3, Language)>>,
) {
    let position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let block_position = position.floor().as_ivec3();
    let language = language.get();
    let cache_matches = cached
        .as_ref()
        .is_some_and(|(position, cached_language)| {
            *position == block_position && *cached_language == language
        });
    if cache_matches && !localization.is_changed() {
        return;
    }
    *cached = Some((block_position, language));

    let next_text = localization
        .text(language, "hud.coordinates")
        .replace("{x}", &block_position.x.to_string())
        .replace("{z}", &block_position.z.to_string())
        .replace("{y}", &block_position.y.to_string());

    if coordinates_text.0 != next_text {
        coordinates_text.0 = next_text;
    }
}
