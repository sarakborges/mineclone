use std::{
    cmp::Reverse,
    collections::BinaryHeap,
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
const WARP_SEARCH_DIAMETER: usize = (WARP_SEARCH_RADIUS_BLOCKS * 2 + 1) as usize;
const WARP_SEARCH_VOLUME: usize =
    WARP_SEARCH_DIAMETER * WARP_SEARCH_DIAMETER * WARP_SEARCH_DIAMETER;
const WARP_STREAMING_MIN_RADIUS_CHUNKS: i32 = 1;
const SUPPORT_PROBE: f32 = 0.08;
const BOUNDS_EPSILON: f32 = 0.0001;
const SLOW_WARP_SEARCH_WARNING: Duration = Duration::from_millis(4);
const WARP_SEARCH_FRAME_BUDGET: Duration = Duration::from_millis(1);
const WARP_SEARCH_BUDGET_CHECK_INTERVAL: usize = 64;

#[derive(Default)]
struct WarpSearchState {
    radius: i32,
    frontier: BinaryHeap<Reverse<(i32, i32, i32, i32)>>,
    visited: Vec<bool>,
}

impl WarpSearchState {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn ensure_started(&mut self) {
        if !self.visited.is_empty() {
            return;
        }
        self.visited = vec![false; WARP_SEARCH_VOLUME];
        self.enqueue(IVec3::ZERO);
    }

    fn enqueue(&mut self, offset: IVec3) {
        let Some(index) = warp_offset_index(offset) else {
            return;
        };
        if self.visited[index] {
            return;
        }
        self.visited[index] = true;
        self.frontier.push(warp_queue_entry(offset));
    }

    fn requeue(&mut self, offset: IVec3) {
        debug_assert!(warp_offset_index(offset).is_some());
        self.frontier.push(warp_queue_entry(offset));
    }

    fn pop_nearest(&mut self) -> Option<IVec3> {
        let Reverse((_distance_squared, x, y, z)) = self.frontier.pop()?;
        Some(IVec3::new(x, y, z))
    }

    fn expand_from(&mut self, offset: IVec3) {
        for direction in [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ] {
            self.enqueue(offset + direction);
        }
    }
}

fn warp_queue_entry(offset: IVec3) -> Reverse<(i32, i32, i32, i32)> {
    Reverse((
        offset.length_squared(),
        offset.x,
        offset.y,
        offset.z,
    ))
}

fn warp_offset_index(offset: IVec3) -> Option<usize> {
    if offset.abs().max_element() > WARP_SEARCH_RADIUS_BLOCKS {
        return None;
    }

    let shift = WARP_SEARCH_RADIUS_BLOCKS;
    let x = (offset.x + shift) as usize;
    let y = (offset.y + shift) as usize;
    let z = (offset.z + shift) as usize;
    Some(x + y * WARP_SEARCH_DIAMETER + z * WARP_SEARCH_DIAMETER * WARP_SEARCH_DIAMETER)
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
    search.ensure_started();

    loop {
        let Some(offset) = search.pop_nearest() else {
            return WarpSearchResult::Exhausted;
        };
        search.radius = search.radius.max(offset.abs().max_element());

        let Some(feet) = target
            .x
            .checked_add(offset.x)
            .zip(target.y.checked_add(offset.y))
            .zip(target.z.checked_add(offset.z))
            .map(|((x, y), z)| IVec3::new(x, y, z))
        else {
            search.expand_from(offset);
            continue;
        };

        match candidate_state(world, feet) {
            CandidateState::Unloaded => {
                search.requeue(offset);
                return WarpSearchResult::Pending;
            }
            CandidateState::Invalid => search.expand_from(offset),
            CandidateState::Valid(eye) => return WarpSearchResult::Found(eye),
        }

        candidates_since_budget_check += 1;
        if candidates_since_budget_check >= WARP_SEARCH_BUDGET_CHECK_INTERVAL {
            candidates_since_budget_check = 0;
            if frame_started.elapsed() >= WARP_SEARCH_FRAME_BUDGET {
                return WarpSearchResult::Pending;
            }
        }
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
    fn warp_priority_queue_visits_offsets_by_non_decreasing_distance() {
        let mut search = WarpSearchState::default();
        search.ensure_started();

        let mut previous_distance = 0;
        for index in 0..64 {
            let offset = search.pop_nearest().expect("search frontier must continue");
            let distance = offset.length_squared();
            if index == 0 {
                assert_eq!(offset, IVec3::ZERO);
            } else {
                assert!(distance >= previous_distance);
            }
            previous_distance = distance;
            search.expand_from(offset);
        }
    }

    #[test]
    fn unloaded_candidate_can_be_requeued_without_expanding_search() {
        let mut search = WarpSearchState::default();
        search.ensure_started();
        let candidate = search.pop_nearest().expect("origin candidate must exist");
        assert_eq!(candidate, IVec3::ZERO);

        search.requeue(candidate);

        assert_eq!(search.pop_nearest(), Some(IVec3::ZERO));
        assert_eq!(search.frontier.len(), 0);
    }

    #[test]
    fn warp_search_never_enqueues_offsets_outside_maximum_cube() {
        let mut search = WarpSearchState::default();
        search.ensure_started();
        search.enqueue(IVec3::new(WARP_SEARCH_RADIUS_BLOCKS + 1, 0, 0));
        assert_eq!(search.frontier.len(), 1);
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
        pending.search.ensure_started();
        let origin = pending
            .search
            .pop_nearest()
            .expect("origin candidate must exist before reset");
        pending.search.expand_from(origin);
        pending.search.radius = 8;

        pending.request(IVec3::new(-4, 7, 9));

        assert_eq!(pending.target, Some(IVec3::new(-4, 7, 9)));
        assert_eq!(pending.search.radius, 0);
        assert!(pending.search.frontier.is_empty());
        assert!(pending.search.visited.is_empty());
    }
}
