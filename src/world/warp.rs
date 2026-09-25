use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

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
        chunk::CHUNK_SIZE,
        collision::collides_aabb,
        coordinates::chunk_coord_from_world,
        world::VoxelWorld,
    },
};

const WARP_SEARCH_RADIUS_BLOCKS: i32 = 32;
const WARP_STREAMING_MIN_RADIUS_CHUNKS: i32 = 1;
const SUPPORT_PROBE: f32 = 0.08;
const BOUNDS_EPSILON: f32 = 0.0001;
const SLOW_WARP_SEARCH_WARNING: Duration = Duration::from_millis(4);
const WARP_SEARCH_FRAME_BUDGET: Duration = Duration::from_millis(1);
const WARP_SEARCH_BUDGET_CHECK_INTERVAL: usize = 64;

#[derive(Default)]
struct WarpSearchState {
    radius: i32,
    remaining: VecDeque<IVec3>,
    unloaded: Vec<IVec3>,
    best: Option<(i32, Vec3)>,
}

impl WarpSearchState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn prepare_current_shell(&mut self) {
        debug_assert!(self.remaining.is_empty());
        debug_assert!(self.unloaded.is_empty());
        let radius = self.radius;
        for y in -radius..=radius {
            for z in -radius..=radius {
                for x in -radius..=radius {
                    if radius > 0 && x.abs().max(y.abs()).max(z.abs()) != radius {
                        continue;
                    }
                    self.remaining.push_back(IVec3::new(x, y, z));
                }
            }
        }
    }

    fn prepare_unloaded_retry(&mut self) {
        debug_assert!(self.remaining.is_empty());
        self.remaining.extend(self.unloaded.drain(..));
    }
}

#[derive(Resource, Default)]
pub(crate) struct PendingWarp {
    target: Option<IVec3>,
    search: WarpSearchState,
}

impl PendingWarp {
    pub(crate) fn request(&mut self, target: IVec3) {
        self.target = Some(target);
        self.search.reset();
    }

    pub(super) fn streaming_center(&self) -> Option<IVec3> {
        self.target.map(|target| {
            let mut coord = chunk_coord_from_world(target);
            coord.y = coord.y.max(0);
            coord
        })
    }

    pub(super) fn streaming_radii(&self) -> Option<(i32, i32)> {
        self.target.map(|_| {
            let chunk_size = CHUNK_SIZE as i32;
            let search_radius_blocks = self.search.radius.max(0);
            let search_radius_chunks = ((search_radius_blocks + chunk_size - 1) / chunk_size)
                .max(WARP_STREAMING_MIN_RADIUS_CHUNKS);
            (search_radius_chunks, search_radius_chunks)
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
    mut slow_search_warned: Local<bool>,
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
        *slow_search_warned = false;
        return;
    };

    let search_started = Instant::now();
    let result = advance_safe_eye_position_search(&world, target, &mut pending.search);
    let search_elapsed = search_started.elapsed();
    if search_elapsed >= SLOW_WARP_SEARCH_WARNING && !*slow_search_warned {
        warn!(
            "slow warp safe-position search: target={target:?} elapsed_ms={:.2}",
            search_elapsed.as_secs_f64() * 1_000.0
        );
        *slow_search_warned = true;
    }

    match result {
        WarpSearchResult::Pending => {}
        WarpSearchResult::Found(destination) => {
            let (transform, walking, flight, gravity, swimming) = &mut *player;
            transform.translation = destination;
            walking.reset_motion();
            flight.reset_motion();
            gravity.reset_motion();
            swimming.reset_motion();
            pending.target = None;
            pending.search.reset();
            *slow_search_warned = false;
        }
        WarpSearchResult::Exhausted => {
            warn!(
                "could not find a safe warp destination within {} blocks of {:?}",
                WARP_SEARCH_RADIUS_BLOCKS,
                target
            );
            pending.target = None;
            pending.search.reset();
            *slow_search_warned = false;
        }
    }
}

fn advance_safe_eye_position_search(
    world: &VoxelWorld,
    target: IVec3,
    search: &mut WarpSearchState,
) -> WarpSearchResult {
    let frame_started = Instant::now();
    let mut candidates_since_budget_check = 0_usize;

    loop {
        if search.remaining.is_empty() {
            if !search.unloaded.is_empty() {
                search.prepare_unloaded_retry();
            } else if search.radius > WARP_SEARCH_RADIUS_BLOCKS {
                return search.best.map_or(WarpSearchResult::Exhausted, |(_, eye)| {
                    WarpSearchResult::Found(eye)
                });
            } else {
                search.prepare_current_shell();
            }
        }

        while let Some(offset) = search.remaining.pop_front() {
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
                CandidateState::Unloaded => search.unloaded.push(offset),
                CandidateState::Invalid => {}
                CandidateState::Valid(eye) => {
                    let distance_squared = offset.length_squared();
                    if search
                        .best
                        .as_ref()
                        .is_none_or(|(best_distance, _)| distance_squared < *best_distance)
                    {
                        search.best = Some((distance_squared, eye));
                    }
                }
            }

            candidates_since_budget_check += 1;
            if candidates_since_budget_check >= WARP_SEARCH_BUDGET_CHECK_INTERVAL {
                candidates_since_budget_check = 0;
                if frame_started.elapsed() >= WARP_SEARCH_FRAME_BUDGET {
                    return WarpSearchResult::Pending;
                }
            }
        }

        if let Some((distance_squared, eye)) = search.best
            && distance_squared <= (search.radius + 1).pow(2)
        {
            return WarpSearchResult::Found(eye);
        }

        if !search.unloaded.is_empty() {
            return WarpSearchResult::Pending;
        }

        search.radius += 1;
    }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_shell_contains_only_the_requested_chebyshev_radius() {
        let mut search = WarpSearchState {
            radius: 2,
            ..default()
        };
        search.prepare_current_shell();

        assert!(!search.remaining.is_empty());
        assert!(search.remaining.iter().all(|offset| {
            offset.x.abs().max(offset.y.abs()).max(offset.z.abs()) == 2
        }));
        assert_eq!(search.remaining.len(), 5_usize.pow(3) - 3_usize.pow(3));
    }

    #[test]
    fn warp_search_retries_only_offsets_that_were_unloaded() {
        let mut search = WarpSearchState {
            radius: 3,
            unloaded: vec![IVec3::new(1, 2, 3), IVec3::new(-2, 0, 3)],
            ..default()
        };
        search.prepare_unloaded_retry();

        assert!(search.unloaded.is_empty());
        assert_eq!(search.remaining.len(), 2);
        assert_eq!(search.remaining.pop_front(), Some(IVec3::new(1, 2, 3)));
        assert_eq!(search.remaining.pop_front(), Some(IVec3::new(-2, 0, 3)));
    }

    #[test]
    fn warp_streaming_radius_grows_only_when_search_crosses_a_chunk() {
        let mut pending = PendingWarp::default();
        pending.request(IVec3::new(15, 64, 15));

        assert_eq!(pending.streaming_radii(), Some((1, 1)));

        pending.search.radius = CHUNK_SIZE as i32;
        assert_eq!(pending.streaming_radii(), Some((1, 1)));

        pending.search.radius = CHUNK_SIZE as i32 + 1;
        assert_eq!(pending.streaming_radii(), Some((2, 2)));
    }

    #[test]
    fn requesting_a_new_warp_resets_previous_search_progress() {
        let mut pending = PendingWarp::default();
        pending.request(IVec3::new(10, 20, 30));
        pending.search.radius = 8;
        pending.search.unloaded.push(IVec3::ONE);

        pending.request(IVec3::new(-4, 7, 9));

        assert_eq!(pending.target, Some(IVec3::new(-4, 7, 9)));
        assert_eq!(pending.search.radius, 0);
        assert!(pending.search.remaining.is_empty());
        assert!(pending.search.unloaded.is_empty());
        assert!(pending.search.best.is_none());
    }
}
