use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::creature::CreatureCollider,
    creatures::CreatureInstance,
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PlayerEntity},
    voxel::{collision::aabb_is_clear, world::VoxelWorld},
};

use super::config::COLLISION_STEP;

// This is a positional contact response, not a second movement controller.
// Both gameplay roots move; neither the camera's view orientation nor the
// creature's animated visual wrapper participates in collision.
const CONTACT_EPSILON: f32 = 0.0001;
const PLAYER_PUSH_SHARE: f32 = 0.35;
const CREATURE_PUSH_SHARE: f32 = 0.5;
const MAX_CONTACT_PASSES: usize = 4;

type Bounds = (Vec3, Vec3);
type CreatureContacts<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static CreatureCollider),
    (With<CreatureInstance>, Without<PlayerEntity>),
>;

fn player_bounds(eye: Vec3) -> Bounds {
    let feet = eye.y - PLAYER_EYE_HEIGHT;
    (
        Vec3::new(
            eye.x - PLAYER_HALF_WIDTH,
            feet,
            eye.z - PLAYER_HALF_WIDTH,
        ),
        Vec3::new(
            eye.x + PLAYER_HALF_WIDTH,
            feet + PLAYER_HEIGHT,
            eye.z + PLAYER_HALF_WIDTH,
        ),
    )
}

#[derive(Clone, Copy)]
enum HorizontalAxis {
    X,
    Z,
}

#[derive(Clone, Copy)]
struct HorizontalContact {
    axis: HorizontalAxis,
    penetration: f32,
    second_direction: f32,
}

fn contact(first: Bounds, second: Bounds) -> Option<HorizontalContact> {
    let overlap_y = first.1.y.min(second.1.y) - first.0.y.max(second.0.y);
    if overlap_y <= 0.0 {
        return None;
    }

    let overlap_x = first.1.x.min(second.1.x) - first.0.x.max(second.0.x);
    let overlap_z = first.1.z.min(second.1.z) - first.0.z.max(second.0.z);
    if overlap_x <= 0.0 || overlap_z <= 0.0 {
        return None;
    }

    let (axis, penetration, first_center, second_center) = if overlap_x <= overlap_z {
        (
            HorizontalAxis::X,
            overlap_x,
            first.0.x + first.1.x,
            second.0.x + second.1.x,
        )
    } else {
        (
            HorizontalAxis::Z,
            overlap_z,
            first.0.z + first.1.z,
            second.0.z + second.1.z,
        )
    };
    // When centers coincide, pick a stable side rather than producing NaN.
    let second_direction = if second_center >= first_center {
        1.0
    } else {
        -1.0
    };

    Some(HorizontalContact {
        axis,
        penetration: penetration + CONTACT_EPSILON,
        second_direction,
    })
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
        if !aabb_is_clear(world, bounds_at(next)) {
            break;
        }
        *position = next;
        moved += step.abs();
    }
    moved
}

fn resolve_contact_pair(
    world: &VoxelWorld,
    first_position: &mut Vec3,
    second_position: &mut Vec3,
    contact: HorizontalContact,
    first_share: f32,
    first_bounds_at: impl Fn(Vec3) -> Bounds,
    second_bounds_at: impl Fn(Vec3) -> Bounds,
) {
    let first_target = contact.penetration * first_share;
    let second_target = contact.penetration - first_target;
    let first_moved = push(
        first_position,
        world,
        contact.axis,
        -contact.second_direction * first_target,
        &first_bounds_at,
    );
    let second_moved = push(
        second_position,
        world,
        contact.axis,
        contact.second_direction * second_target,
        &second_bounds_at,
    );
    let remainder = (contact.penetration - first_moved - second_moved).max(0.0);
    if remainder <= 0.0 {
        return;
    }

    let extra_second = push(
        second_position,
        world,
        contact.axis,
        contact.second_direction * remainder,
        &second_bounds_at,
    );
    if extra_second < remainder {
        push(
            first_position,
            world,
            contact.axis,
            -contact.second_direction * (remainder - extra_second),
            &first_bounds_at,
        );
    }
}

/// After the movement systems, share horizontal penetration between the
/// player and each creature. If one hits terrain, transfer the remainder to
/// the other. No displacement may pass through a solid or unloaded voxel.
pub(super) fn resolve_player_creature_contacts(
    world: Res<VoxelWorld>,
    mut player: Single<&mut Transform, With<PlayerEntity>>,
    mut creatures: CreatureContacts<'_, '_>,
) {
    for _ in 0..MAX_CONTACT_PASSES {
        let mut had_contact = false;
        for (mut creature, collider) in &mut creatures {
            let Some(contact) = contact(
                player_bounds(player.translation),
                collider.bounds(creature.translation),
            ) else {
                continue;
            };
            had_contact = true;

            resolve_contact_pair(
                &world,
                &mut player.translation,
                &mut creature.translation,
                contact,
                PLAYER_PUSH_SHARE,
                player_bounds,
                |feet| collider.bounds(feet),
            );
        }
        if !had_contact {
            break;
        }
    }
}

/// Share horizontal penetration between creature roots after their movement.
/// Creature animation wrappers never participate; only gameplay colliders move.
/// Terrain remains authoritative, and any displacement one creature cannot take
/// is transferred to the other instead of pushing either through a wall.
pub(super) fn resolve_creature_creature_contacts(
    world: Res<VoxelWorld>,
    mut creatures: CreatureContacts<'_, '_>,
) {
    for _ in 0..MAX_CONTACT_PASSES {
        let mut had_contact = false;
        let mut pairs = creatures.iter_combinations_mut::<2>();
        while let Some([(mut first, first_collider), (mut second, second_collider)]) =
            pairs.fetch_next()
        {
            let Some(contact) = contact(
                first_collider.bounds(first.translation),
                second_collider.bounds(second.translation),
            ) else {
                continue;
            };
            had_contact = true;

            resolve_contact_pair(
                &world,
                &mut first.translation,
                &mut second.translation,
                contact,
                CREATURE_PUSH_SHARE,
                |feet| first_collider.bounds(feet),
                |feet| second_collider.bounds(feet),
            );
        }
        if !had_contact {
            break;
        }
    }
}

pub(super) fn contacts_enabled() -> impl Fn(Res<State<GameState>>, Res<State<PauseState>>) -> bool {
    |game, pause| *game.get() == GameState::Gameplay && *pause.get() == PauseState::Running
}
