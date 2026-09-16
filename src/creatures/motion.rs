use bevy::prelude::*;

use crate::{
    content::creature::{CreatureCollider, CreatureRegistry},
    player::movement::config::{COLLISION_STEP, GRAVITY},
    voxel::{collision::collides_aabb, world::VoxelWorld},
};

use super::{CreatureInstance, visual::CreatureAnimationState};

const GROUND_PROBE: f32 = 0.06;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HopPhase {
    Idle,
    Anticipate,
    Airborne,
    Land,
}

#[derive(Component)]
pub(super) struct CreatureMotion {
    phase: HopPhase,
    timer: f32,
    velocity_y: f32,
}

impl Default for CreatureMotion {
    fn default() -> Self {
        Self {
            phase: HopPhase::Idle,
            timer: 1.0,
            velocity_y: 0.0,
        }
    }
}

pub(super) fn move_creatures(
    time: Res<Time>,
    world: Res<VoxelWorld>,
    definitions: Res<CreatureRegistry>,
    mut creatures: Query<(
        &CreatureInstance,
        &CreatureCollider,
        &mut Transform,
        &mut CreatureMotion,
        &mut CreatureAnimationState,
    )>,
) {
    let dt = time.delta_secs().min(0.05);
    for (instance, collider, mut transform, mut motion, mut animation) in &mut creatures {
        let Some(definition) = definitions.get(&instance.definition_id) else {
            continue;
        };
        if !world.is_loaded_at(transform.translation.floor().as_ivec3()) {
            continue;
        }
        if motion.phase != HopPhase::Airborne
            && !on_ground(&world, *collider, transform.translation)
        {
            motion.phase = HopPhase::Airborne;
            motion.velocity_y = 0.0;
            set_animation(&mut animation, "airborne");
        }
        match motion.phase {
            HopPhase::Idle => {
                if definition.jump_speed <= 0.0 {
                    continue;
                }
                motion.timer -= dt;
                if motion.timer <= 0.0 {
                    motion.phase = HopPhase::Anticipate;
                    motion.timer = definition.anticipation_seconds;
                    set_animation(&mut animation, "anticipate");
                }
            }
            HopPhase::Anticipate => {
                motion.timer -= dt;
                if motion.timer <= 0.0 {
                    motion.phase = HopPhase::Airborne;
                    motion.velocity_y = definition.jump_speed;
                    set_animation(&mut animation, "airborne");
                }
            }
            HopPhase::Airborne => {
                motion.velocity_y += GRAVITY * dt;
                let travel = motion.velocity_y * dt;
                let hit = advance_vertical(&world, *collider, &mut transform.translation, travel);
                if hit && motion.velocity_y <= 0.0 {
                    motion.phase = HopPhase::Land;
                    motion.timer = definition.landing_seconds;
                    motion.velocity_y = 0.0;
                    set_animation(&mut animation, "land");
                } else if hit {
                    motion.velocity_y = 0.0;
                }
            }
            HopPhase::Land => {
                motion.timer -= dt;
                if motion.timer <= 0.0 {
                    motion.phase = HopPhase::Idle;
                    motion.timer = definition.jump_interval;
                    set_animation(&mut animation, "idle");
                }
            }
        }
    }
}

fn set_animation(state: &mut CreatureAnimationState, next: &str) {
    if state.0 != next {
        state.0 = next.to_owned();
    }
}

fn on_ground(world: &VoxelWorld, collider: CreatureCollider, feet: Vec3) -> bool {
    let (min, max) = collider.bounds(feet - Vec3::Y * GROUND_PROBE);
    collides_aabb(world, min, max)
}

/// Substeps stop thin blocks from being skipped at high vertical speed.
/// The collider remains the same size throughout every squash/stretch clip.
fn advance_vertical(
    world: &VoxelWorld,
    collider: CreatureCollider,
    feet: &mut Vec3,
    delta: f32,
) -> bool {
    let steps = (delta.abs() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = delta / steps as f32;
    for _ in 0..steps {
        let next = *feet + Vec3::Y * step;
        let (min, max) = collider.bounds(next);
        if !world.is_loaded_at(min.floor().as_ivec3())
            || !world.is_loaded_at(max.floor().as_ivec3())
            || collides_aabb(world, min, max)
        {
            return true;
        }
        *feet = next;
    }
    false
}
