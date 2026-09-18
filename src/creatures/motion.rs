use crate::entity::EntityHealth;
use bevy::prelude::*;

use crate::{
    content::creature::{CreatureCollider, CreatureRegistry},
    player::movement::config::{COLLISION_STEP, GRAVITY},
    voxel::{collision::collides_aabb, world::VoxelWorld},
};

use super::{CreatureInstance, visual::CreatureAnimationState};

const GROUND_PROBE: f32 = 0.06;
const DIRECTIONS: [(i32, i32); 8] = [
    (0, -1), (1, -1), (1, 0), (1, 1),
    (0, 1), (-1, 1), (-1, 0), (-1, -1),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum HopPhase {
    Idle,
    Anticipate,
    Airborne,
    Land,
}

#[derive(Component)]
pub(crate) struct CreatureMotion {
    phase: HopPhase,
    timer: f32,
    velocity_y: f32,
    direction: Vec2,
    facing_yaw: f32,
    random_state: u32,
    knockback: Vec3,
}

impl Default for CreatureMotion {
    fn default() -> Self {
        Self {
            phase: HopPhase::Idle,
            timer: 1.0,
            velocity_y: 0.0,
            direction: Vec2::ZERO,
            facing_yaw: 0.0,
            random_state: 0,
            knockback: Vec3::ZERO,
        }
    }
}

impl CreatureMotion {
    pub(super) fn facing_yaw(&self) -> f32 {
        self.facing_yaw
    }

    pub(crate) fn apply_knockback(&mut self, direction: Vec3, strength: f32) {
        let horizontal = Vec2::new(direction.x, direction.z);
        if horizontal.length_squared() > 0.0 && strength > 0.0 {
            let impulse = horizontal.normalize() * strength;
            self.knockback.x += impulse.x;
            self.knockback.z += impulse.y;
        }
    }

    /// Each creature has its own pseudorandom stream, seeded by its spawn slot.
    /// Only choose a new direction at the beginning of the next jump.
    fn choose_heading(&mut self, position: Vec3) {
        if self.random_state == 0 {
            self.random_state = position.x.to_bits()
                ^ position.y.to_bits().rotate_left(11)
                ^ position.z.to_bits().rotate_left(23)
                ^ 0xA341_316C;
            if self.random_state == 0 {
                self.random_state = 0x9E37_79B9;
            }
        }
        self.random_state ^= self.random_state << 13;
        self.random_state ^= self.random_state >> 17;
        self.random_state ^= self.random_state << 5;
        let (x, z) = DIRECTIONS[self.random_state as usize % DIRECTIONS.len()];
        self.direction = Vec2::new(x as f32, z as f32).normalize();
        // The shared slime model faces -Z. Rotate the visual wrapper only.
        self.facing_yaw = (-self.direction.x).atan2(-self.direction.y);
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
        &EntityHealth,
    )>,
) {
    let dt = time.delta_secs().min(0.05);
    for (instance, collider, mut transform, mut motion, mut animation, health) in &mut creatures {
        if health.is_dead() {
            set_animation(&mut animation, "death");
            continue;
        }
        let Some(definition) = definitions.get(&instance.definition_id) else {
            continue;
        };
        if !world.is_loaded_at(transform.translation.floor().as_ivec3()) {
            continue;
        }
        if motion.knockback.x != 0.0 || motion.knockback.z != 0.0 {
            let knockback = motion.knockback;
            if advance_horizontal(&world, *collider, &mut transform.translation, Vec2::new(knockback.x, knockback.z) * dt) {
                motion.knockback = Vec3::ZERO;
            } else {
                let damping = (1.0 - 8.0 * dt).max(0.0);
                motion.knockback.x *= damping;
                motion.knockback.z *= damping;
                if motion.knockback.x.abs() < 0.01 { motion.knockback.x = 0.0; }
                if motion.knockback.z.abs() < 0.01 { motion.knockback.z = 0.0; }
            }
        }
        if motion.phase != HopPhase::Airborne && !on_ground(&world, *collider, transform.translation) {
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
                    if definition.move_speed > 0.0 {
                        motion.choose_heading(transform.translation);
                    }
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
                let horizontal = motion.direction * definition.move_speed * dt;
                if horizontal != Vec2::ZERO
                    && advance_horizontal(&world, *collider, &mut transform.translation, horizontal)
                {
                    // A wall stops this hop, without ever moving the static collider through it.
                    motion.direction = Vec2::ZERO;
                }
                motion.velocity_y += GRAVITY * dt;
                let travel = motion.velocity_y * dt;
                let hit = advance_vertical(&world, *collider, &mut transform.translation, travel);
                if hit && motion.velocity_y <= 0.0 {
                    motion.phase = HopPhase::Land;
                    motion.timer = definition.landing_seconds;
                    motion.velocity_y = 0.0;
                    motion.direction = Vec2::ZERO;
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
    if state.name != next {
        state.name = next.to_owned();
    }
}

fn on_ground(world: &VoxelWorld, collider: CreatureCollider, feet: Vec3) -> bool {
    let (min, max) = collider.bounds(feet - Vec3::Y * GROUND_PROBE);
    collides_aabb(world, min, max)
}

/// Prevents crossing a solid or unloaded voxel while moving laterally in a jump.
fn advance_horizontal(
    world: &VoxelWorld,
    collider: CreatureCollider,
    feet: &mut Vec3,
    delta: Vec2,
) -> bool {
    let steps = (delta.length() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = delta / steps as f32;
    for _ in 0..steps {
        let next = *feet + Vec3::new(step.x, 0.0, step.y);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_heading_is_normalized_and_keeps_a_consistent_facing() {
        let mut motion = CreatureMotion::default();
        motion.choose_heading(Vec3::new(2.5, 12.0, -4.5));
        assert!((motion.direction.length() - 1.0).abs() < 0.0001);
        let direction = motion.direction;
        assert!((Vec2::new(-motion.facing_yaw.sin(), -motion.facing_yaw.cos()) - direction).length() < 0.0001);
    }
}
