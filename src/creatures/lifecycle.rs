use bevy::prelude::*;

use crate::{
    app::crash_log::log_gameplay_event,
    player::camera::GameplayCamera,
};

use super::{CreatureInstance, EntityMetaTags};

const CREATURE_DESPAWN_RADIUS: f32 = 128.0;
const CREATURE_DESPAWN_RADIUS_SQUARED: f32 = CREATURE_DESPAWN_RADIUS * CREATURE_DESPAWN_RADIUS;
const CREATURE_DESPAWN_CHECK_INTERVAL: f32 = 1.0;
const CREATURE_DESPAWN_GRACE_SECONDS: f32 = 5.0;

#[derive(Component)]
pub(crate) struct CreatureDeathTimer(pub(crate) Timer);

#[derive(Component)]
pub(crate) struct CreatureDespawnGrace {
    seconds_remaining: f32,
}

impl Default for CreatureDespawnGrace {
    fn default() -> Self {
        Self {
            seconds_remaining: CREATURE_DESPAWN_GRACE_SECONDS,
        }
    }
}

impl CreatureDespawnGrace {
    fn tick(&mut self, delta_seconds: f32) {
        self.seconds_remaining = (self.seconds_remaining - delta_seconds).max(0.0);
    }

    fn is_active(&self) -> bool {
        self.seconds_remaining > 0.0
    }
}

#[derive(Default)]
pub(super) struct CreatureDespawnState {
    seconds_until_check: f32,
}

pub(super) fn despawn_distant_creatures(
    time: Res<Time>,
    mut commands: Commands,
    player: Single<&Transform, With<GameplayCamera>>,
    mut creatures: Query<
        (Entity, &Transform, &EntityMetaTags, &mut CreatureDespawnGrace),
        (With<CreatureInstance>, Without<CreatureDeathTimer>),
    >,
    mut state: Local<CreatureDespawnState>,
) {
    let delta_seconds = time.delta_secs();
    state.seconds_until_check -= delta_seconds;
    let should_check_distance = state.seconds_until_check <= 0.0;
    if should_check_distance {
        state.seconds_until_check = CREATURE_DESPAWN_CHECK_INTERVAL;
    }

    for (entity, transform, meta_tags, mut grace) in &mut creatures {
        grace.tick(delta_seconds);
        if !should_check_distance || grace.is_active() || meta_tags.is_persistent() {
            continue;
        }

        let distance_squared = transform.translation.distance_squared(player.translation);
        if distance_squared <= CREATURE_DESPAWN_RADIUS_SQUARED {
            continue;
        }

        log_gameplay_event(format!(
            "entity.despawn entity={:?} type=creature reason=distance distance={:.1} radius={:.1}",
            entity,
            distance_squared.sqrt(),
            CREATURE_DESPAWN_RADIUS
        ));
        commands.entity(entity).despawn();
    }
}

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
