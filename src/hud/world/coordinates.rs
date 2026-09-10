use bevy::prelude::*;

use crate::player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera};

#[derive(Component)]
pub(super) struct CoordinatesHudText;

pub(super) fn update_coordinates_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    mut coordinates_text: Single<&mut Text, With<CoordinatesHudText>>,
) {
    let position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let block_position = position.floor().as_ivec3();

    coordinates_text.0 = format!(
        "X: {} | Z: {} | Y: {}",
        block_position.x, block_position.z, block_position.y
    );
}
