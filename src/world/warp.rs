use bevy::prelude::*;

use crate::{
    player::{
        PLAYER_EYE_HEIGHT, PlayerEntity,
        movement::{
            collision::player_bounds, flight::FlightState, gravity::GravityState,
            swimming::SwimmingState, walking::WalkingState,
        },
    },
    voxel::{
        collision::collides_aabb,
        coordinates::chunk_coord_from_world,
        world::VoxelWorld,
    },
};

const WARP_SEARCH_RADIUS_BLOCKS: i32 = 32;
const SUPPORT_PROBE: f32 = 0.08;
const BOUNDS_EPSILON: f32 = 0.0001;

#[derive(Resource, Default)]
pub(crate) struct PendingWarp {
    target: Option<IVec3>,
}

impl PendingWarp {
    pub(crate) fn request(&mut self, target: IVec3) {
        self.target = Some(target);
    }

    pub(super) fn streaming_center(&self) -> Option<IVec3> {
        self.target.map(|target| {
            let mut coord = chunk_coord_from_world(target);
            coord.y = coord.y.max(0);
            coord
        })
    }
}

enum CandidateState {
    Unloaded,
    Invalid,
    Valid(Vec3),
}

enum WarpSearchResult {
    Pending,
    Found(Vec3),
    Exhausted,
}

pub(super) fn resolve_pending_warp(
    mut pending: ResMut<PendingWarp>,
    world: Res<VoxelWorld>,
    mut player: Single<
        (
            &mut Transform,
            &mut WalkingState,
            &mut FlightState,
            &mut GravityState,
            &mut SwimmingState,
        ),
        With<PlayerEntity>,
    >,
) {
    let Some(target) = pending.target else {
        return;
    };

    match find_nearest_safe_eye_position(&world, target) {
        WarpSearchResult::Pending => {}
        WarpSearchResult::Found(destination) => {
            let (transform, walking, flight, gravity, swimming) = &mut *player;
            transform.translation = destination;
            walking.reset_motion();
            flight.reset_motion();
            gravity.reset_motion();
            swimming.reset_motion();
            pending.target = None;
        }
        WarpSearchResult::Exhausted => {
            warn!(
                "could not find a safe warp destination within {} blocks of {:?}",
                WARP_SEARCH_RADIUS_BLOCKS,
                target
            );
            pending.target = None;
        }
    }
}

fn find_nearest_safe_eye_position(world: &VoxelWorld, target: IVec3) -> WarpSearchResult {
    let mut best: Option<(i32, Vec3)> = None;

    for radius in 0..=WARP_SEARCH_RADIUS_BLOCKS {
        let mut shell_unloaded = false;

        for y in -radius..=radius {
            for z in -radius..=radius {
                for x in -radius..=radius {
                    if radius > 0 && x.abs().max(y.abs()).max(z.abs()) != radius {
                        continue;
                    }

                    let offset = IVec3::new(x, y, z);
                    let Some(feet) = target
                        .x
                        .checked_add(offset.x)
                        .zip(target.y.checked_add(offset.y))
                        .zip(target.z.checked_add(offset.z))
                        .map(|((x, y), z)| IVec3::new(x, y, z))
                    else {
                        continue;
                    };
                    match candidate_state(world, feet) {
                        CandidateState::Unloaded => {
                            shell_unloaded = true;
                        }
                        CandidateState::Invalid => {}
                        CandidateState::Valid(eye) => {
                            let distance_squared = offset.length_squared();
                            if best
                                .as_ref()
                                .is_none_or(|(best_distance, _)| distance_squared < *best_distance)
                            {
                                best = Some((distance_squared, eye));
                            }
                        }
                    }
                }
            }
        }

        if shell_unloaded {
            return WarpSearchResult::Pending;
        }

        if let Some((distance_squared, eye)) = best
            && distance_squared <= (radius + 1).pow(2)
        {
            return WarpSearchResult::Found(eye);
        }
    }

    best.map_or(WarpSearchResult::Exhausted, |(_, eye)| {
        WarpSearchResult::Found(eye)
    })
}

fn candidate_state(world: &VoxelWorld, feet: IVec3) -> CandidateState {
    if feet.y < 0 {
        return CandidateState::Invalid;
    }

    let eye = Vec3::new(
        feet.x as f32 + 0.5,
        feet.y as f32 + PLAYER_EYE_HEIGHT,
        feet.z as f32 + 0.5,
    );
    let bounds = player_bounds(eye);
    let minimum = (bounds.0 + Vec3::splat(BOUNDS_EPSILON))
        .floor()
        .as_ivec3();
    let maximum = (bounds.1 - Vec3::splat(BOUNDS_EPSILON))
        .floor()
        .as_ivec3();

    for y in minimum.y..=maximum.y {
        for z in minimum.z..=maximum.z {
            for x in minimum.x..=maximum.x {
                let voxel = IVec3::new(x, y, z);
                if !world.is_loaded_at(voxel) {
                    return CandidateState::Unloaded;
                }
                if world.fluid_at(voxel).is_some() {
                    return CandidateState::Invalid;
                }
            }
        }
    }

    if collides_aabb(world, bounds.0, bounds.1) {
        return CandidateState::Invalid;
    }

    let support_minimum = bounds.0 - Vec3::Y * SUPPORT_PROBE;
    let support_maximum = bounds.1 - Vec3::Y * SUPPORT_PROBE;
    let support_voxel_min = (support_minimum + Vec3::splat(BOUNDS_EPSILON))
        .floor()
        .as_ivec3();
    let support_voxel_max = (support_maximum - Vec3::splat(BOUNDS_EPSILON))
        .floor()
        .as_ivec3();
    for y in support_voxel_min.y..=support_voxel_max.y {
        for z in support_voxel_min.z..=support_voxel_max.z {
            for x in support_voxel_min.x..=support_voxel_max.x {
                let voxel = IVec3::new(x, y, z);
                if voxel.y < 0 {
                    return CandidateState::Invalid;
                }
                if !world.is_loaded_at(voxel) {
                    return CandidateState::Unloaded;
                }
            }
        }
    }

    if !collides_aabb(world, support_minimum, support_maximum) {
        return CandidateState::Invalid;
    }

    CandidateState::Valid(eye)
}
