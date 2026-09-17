use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::creature::CreatureCollider,
    creatures::CreatureInstance,
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, camera::GameplayCamera},
    voxel::{collision::collides_aabb, world::VoxelWorld},
};

use super::config::COLLISION_STEP;

// This is a positional contact response, not a second movement controller.
// Both gameplay roots move; neither the camera's view orientation nor the
// creature's animated visual wrapper participates in collision.
const CONTACT_EPSILON: f32 = 0.0001;
const PLAYER_PUSH_SHARE: f32 = 0.35;
const MAX_CONTACT_PASSES: usize = 4;

type Bounds = (Vec3, Vec3);

fn player_bounds(eye: Vec3) -> Bounds {
    let feet = eye.y - PLAYER_EYE_HEIGHT;
    (
        Vec3::new(eye.x - PLAYER_HALF_WIDTH, feet, eye.z - PLAYER_HALF_WIDTH),
        Vec3::new(eye.x + PLAYER_HALF_WIDTH, feet + PLAYER_HEIGHT, eye.z + PLAYER_HALF_WIDTH),
    )
}

#[derive(Clone, Copy)]
enum HorizontalAxis {
    X,
    Z,
}

fn contact(player: Bounds, creature: Bounds) -> Option<(HorizontalAxis, f32, f32)> {
    let overlap_y = player.1.y.min(creature.1.y) - player.0.y.max(creature.0.y);
    if overlap_y <= 0.0 {
        return None;
    }
    let overlap_x = player.1.x.min(creature.1.x) - player.0.x.max(creature.0.x);
    let overlap_z = player.1.z.min(creature.1.z) - player.0.z.max(creature.0.z);
    if overlap_x <= 0.0 || overlap_z <= 0.0 {
        return None;
    }
    let (axis, penetration, player_center, creature_center) = if overlap_x <= overlap_z {
        (HorizontalAxis::X, overlap_x, player.0.x + player.1.x, creature.0.x + creature.1.x)
    } else {
        (HorizontalAxis::Z, overlap_z, player.0.z + player.1.z, creature.0.z + creature.1.z)
    };
    // When centers coincide, pick a stable side rather than producing NaN.
    let creature_direction = if creature_center >= player_center { 1.0 } else { -1.0 };
    Some((axis, penetration + CONTACT_EPSILON, creature_direction))
}

fn clear_volume(world: &VoxelWorld, bounds: Bounds) -> bool {
    let min = (bounds.0 + Vec3::splat(CONTACT_EPSILON)).floor().as_ivec3();
    let max = (bounds.1 - Vec3::splat(CONTACT_EPSILON)).floor().as_ivec3();
    for y in min.y..=max.y {
        for z in min.z..=max.z {
            for x in min.x..=max.x {
                if !world.is_loaded_at(IVec3::new(x, y, z)) {
                    return false;
                }
            }
        }
    }
    !collides_aabb(world, bounds.0, bounds.1)
}

/// Sweep small steps so the push cannot tunnel through a wall or unloaded chunk.
/// Return the actual distance traveled, allowing the other participant to
/// absorb any displacement that this one could not take.
fn push(
    position: &mut Vec3,
    world: &VoxelWorld,
    axis: HorizontalAxis,
    distance: f32,
    bounds_at: impl Fn(Vec3) -> Bounds,
) -> f32 {
    if distance == 0.0 {
        return 0.0;
    }
    let steps = (distance.abs() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = distance / steps as f32;
    let mut moved = 0.0;
    for _ in 0..steps {
        let mut next = *position;
        match axis {
            HorizontalAxis::X => next.x += step,
            HorizontalAxis::Z => next.z += step,
        }
        if !clear_volume(world, bounds_at(next)) {
            break;
        }
        *position = next;
        moved += step.abs();
    }
    moved
}

/// After the movement systems, share horizontal penetration between the
/// player and each creature. If one hits terrain, transfer the remainder to
/// the other. No displacement may pass through a solid or unloaded voxel.
pub(super) fn resolve_player_creature_contacts(
    world: Res<VoxelWorld>,
    mut player: Single<&mut Transform, (With<GameplayCamera>, Without<CreatureInstance>)>,
    mut creatures: Query<(&mut Transform, &CreatureCollider), (With<CreatureInstance>, Without<GameplayCamera>)>,
) {
    for _ in 0..MAX_CONTACT_PASSES {
        let mut changed = false;
        for (mut creature, collider) in &mut creatures {
            let Some((axis, penetration, direction)) =
                contact(player_bounds(player.translation), collider.bounds(creature.translation))
            else {
                continue;
            };
            let player_target = penetration * PLAYER_PUSH_SHARE;
            let creature_target = penetration - player_target;
            let player_moved = push(
                &mut player.translation,
                &world,
                axis,
                -direction * player_target,
                player_bounds,
            );
            let creature_moved = push(
                &mut creature.translation,
                &world,
                axis,
                direction * creature_target,
                |feet| collider.bounds(feet),
            );
            let remainder = (penetration - player_moved - creature_moved).max(0.0);
            if remainder > 0.0 {
                let extra_creature = push(
                    &mut creature.translation,
                    &world,
                    axis,
                    direction * remainder,
                    |feet| collider.bounds(feet),
                );
                if extra_creature < remainder {
                    push(
                        &mut player.translation,
                        &world,
                        axis,
                        -direction * (remainder - extra_creature),
                        player_bounds,
                    );
                }
            }
            changed |= contact(player_bounds(player.translation), collider.bounds(creature.translation)).is_none();
        }
        if !changed {
            break;
        }
    }
}

pub(super) fn contacts_enabled() -> impl Fn(Res<State<GameState>>, Res<State<PauseState>>) -> bool {
    |game, pause| *game.get() == GameState::Gameplay && *pause.get() == PauseState::Running
}
