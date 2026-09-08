use bevy::prelude::*;

use crate::player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT};

use super::gravity::GravityState;

pub(super) fn enforce_world_floor(
    mut player: Single<(&mut Transform, &mut GravityState), With<GameplayCamera>>,
) {
    let (mut transform, mut gravity) = player.into_inner();

    if transform.translation.y >= PLAYER_EYE_HEIGHT {
        return;
    }

    transform.translation.y = PLAYER_EYE_HEIGHT;
    gravity.vertical_velocity = 0.0;
    gravity.grounded = true;
}
