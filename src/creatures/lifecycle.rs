use bevy::prelude::*;

use crate::app::crash_log::log_gameplay_event;

#[derive(Component)]
pub(crate) struct CreatureDeathTimer(pub(crate) Timer);

pub(super) fn despawn_dead_creatures(
    time: Res<Time>,
    mut commands: Commands,
    mut dead: Query<(Entity, &mut CreatureDeathTimer)>,
) {
    for (entity, mut timer) in &mut dead {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            log_gameplay_event(format!(
                "entity.despawn entity={:?} reason=death_timer",
                entity
            ));
            commands.entity(entity).despawn();
        }
    }
}
