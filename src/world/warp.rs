use std::{cmp::Reverse, collections::BinaryHeap};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{
        crash_log::{log_gameplay_event, log_gameplay_warn},
        game_state::GameState,
    },
    content::dimension::DimensionRegistry,
    entity::EntityHealth,
    player::{
        PLAYER_EYE_HEIGHT, PlayerEntity,
        camera::GameplayCamera,
        game_mode::GameMode,
        movement::{
            collision::player_bounds, flight::FlightState, gravity::GravityState,
            swimming::SwimmingState, walking::WalkingState,
        },
        player_id::LOCAL_PLAYER_ID,
    },
    voxel::{
        collision::collides_aabb, coordinates::chunk_coord_from_world, world::VoxelWorld,
    },
};

use super::{
    InMemoryWorldSave, WorldLoadMode,
    destination::find_generated_surface_destination,
    dimension::{CurrentDimension, DimensionId},
    dimension_persistence::DimensionRuntimeContext,
    generator::WorldGenerator,
};

const WARP_QUERY_RADIUS_BLOCKS: i32 = 32;
const WARP_RUNTIME_VALIDATION_RADIUS_BLOCKS: i32 = 2;
const WARP_RUNTIME_VALIDATION_DIAMETER: usize =
    (WARP_RUNTIME_VALIDATION_RADIUS_BLOCKS * 2 + 1) as usize;
const WARP_RUNTIME_VALIDATION_VOLUME: usize = WARP_RUNTIME_VALIDATION_DIAMETER
    * WARP_RUNTIME_VALIDATION_DIAMETER
    * WARP_RUNTIME_VALIDATION_DIAMETER;
const WARP_STREAMING_HORIZONTAL_RADIUS_CHUNKS: i32 = 1;
const SUPPORT_PROBE: f32 = 0.08;
const BOUNDS_EPSILON: f32 = 0.0001;

#[derive(Default)]
struct WarpSearchState {
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
        self.visited = vec![false; WARP_RUNTIME_VALIDATION_VOLUME];
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
    Reverse((offset.length_squared(), offset.x, offset.y, offset.z))
}

fn warp_offset_index(offset: IVec3) -> Option<usize> {
    if offset.abs().max_element() > WARP_RUNTIME_VALIDATION_RADIUS_BLOCKS {
        return None;
    }

    let shift = WARP_RUNTIME_VALIDATION_RADIUS_BLOCKS;
    let x = (offset.x + shift) as usize;
    let y = (offset.y + shift) as usize;
    let z = (offset.z + shift) as usize;
    Some(
        x + y * WARP_RUNTIME_VALIDATION_DIAMETER
            + z * WARP_RUNTIME_VALIDATION_DIAMETER * WARP_RUNTIME_VALIDATION_DIAMETER,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WarpOutcome {
    Succeeded(IVec3),
    Failed,
}

#[derive(Resource, Default)]
pub(crate) struct PendingWarp {
    target: Option<IVec3>,
    dimension: Option<String>,
    prepared_surface: Option<IVec3>,
    search: WarpSearchState,
    outcome: Option<WarpOutcome>,
}

impl PendingWarp {
    pub(crate) fn request(&mut self, target: IVec3, dimension: Option<&str>) {
        if let Some(previous) = self.target {
            log_gameplay_event(format!(
                "warp.superseded previous_target={:?} target={:?}",
                previous, target
            ));
        }
        log_gameplay_event(format!(
            "warp.request target={:?} dimension={:?}",
            target, dimension
        ));
        self.target = Some(target);
        self.dimension = dimension.map(str::to_owned);
        self.prepared_surface = None;
        self.search.reset();
        self.outcome = None;
    }

    pub(crate) fn take_outcome(&mut self) -> Option<WarpOutcome> {
        self.outcome.take()
    }

    pub(super) fn streaming_center(&self) -> Option<IVec3> {
        if self.dimension.is_some() {
            return None;
        }
        self.prepared_surface.or(self.target).map(|target| {
            let mut coord = chunk_coord_from_world(target);
            coord.y = coord.y.max(0);
            coord
        })
    }

    pub(super) fn streaming_horizontal_radius(&self) -> Option<i32> {
        (self.target.is_some() && self.dimension.is_none())
            .then_some(WARP_STREAMING_HORIZONTAL_RADIUS_CHUNKS)
    }

    fn prepare_surface(&mut self, target: IVec3) {
        self.prepared_surface = Some(target);
        self.search.reset();
    }

    fn clear_request(&mut self) {
        self.target = None;
        self.dimension = None;
        self.prepared_surface = None;
        self.search.reset();
    }

    fn fail(&mut self) {
        self.clear_request();
        self.outcome = Some(WarpOutcome::Failed);
    }
}

#[derive(SystemParam)]
pub(super) struct DimensionWarpContext<'w, 's> {
    dimensions: Res<'w, DimensionRegistry>,
    current_dimension: ResMut<'w, CurrentDimension>,
    load_mode: ResMut<'w, WorldLoadMode>,
    save: ResMut<'w, InMemoryWorldSave>,
    generator: Res<'w, WorldGenerator>,
    runtime: DimensionRuntimeContext<'w, 's>,
    next_game_state: ResMut<'w, NextState<GameState>>,
}

type WarpPlayer<'w, 's> = Single<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut WalkingState,
        &'static mut FlightState,
        &'static mut GravityState,
        &'static mut SwimmingState,
        &'static GameMode,
        &'static EntityHealth,
        &'static GameplayCamera,
    ),
    With<PlayerEntity>,
>;

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
    mut dimension: DimensionWarpContext,
    mut player: WarpPlayer,
) {
    let Some(target) = pending.target else {
        return;
    };

    if let Some(requested_dimension) = pending.dimension.clone() {
        if requested_dimension == dimension.current_dimension.id.as_str() {
            pending.dimension = None;
        } else {
            if dimension.dimensions.get(&requested_dimension).is_none() {
                log_gameplay_warn(format!(
                    "warp.failed target={target:?} dimension={requested_dimension} reason=unknown_dimension"
                ));
                pending.fail();
                return;
            }

            let (_, _, flight, _, _, game_mode, health, camera) = &mut *player;
            let requested_eye = Vec3::new(
                target.x as f32 + 0.5,
                target.y as f32 + PLAYER_EYE_HEIGHT,
                target.z as f32 + 0.5,
            );

            let previous_dimension = dimension.current_dimension.id.clone();
            if let Err(error) = dimension
                .runtime
                .swap_to(&previous_dimension, &requested_dimension)
            {
                log_gameplay_warn(format!(
                    "warp.failed target={target:?} dimension={requested_dimension} reason=dimension_state error={error}"
                ));
                pending.fail();
                return;
            }

            dimension.save.prepare_dimension_warp(&requested_dimension);
            dimension.save.save_player_state_with_health(
                LOCAL_PLAYER_ID,
                requested_eye,
                **game_mode,
                Some(health.current()),
                Some((camera.yaw, camera.pitch)),
                flight.is_active(),
            );

            let previous_dimension_text = previous_dimension.to_string();
            dimension.current_dimension.id = DimensionId::from(requested_dimension.clone());
            *dimension.load_mode = WorldLoadMode::Load;
            dimension.next_game_state.set(GameState::Loading);
            log_gameplay_event(format!(
                "warp.dimension_transition from={} to={} target={:?} next_state=Loading",
                previous_dimension_text, requested_dimension, target
            ));

            pending.clear_request();
            pending.outcome = None;
            return;
        }
    }

    let result = if let Some(prepared_surface) = pending.prepared_surface {
        advance_local_runtime_validation(
            dimension.runtime.world(),
            prepared_surface,
            &mut pending.search,
        )
    } else {
        match candidate_state(dimension.runtime.world(), target) {
            CandidateState::Unloaded => WarpSearchResult::Pending,
            CandidateState::Valid(eye) => WarpSearchResult::Found(eye),
            CandidateState::Invalid => {
                let generated = find_generated_surface_destination(
                    &dimension.generator,
                    target.xz(),
                    WARP_QUERY_RADIUS_BLOCKS,
                    |_| true,
                );
                let Some(generated) = generated else {
                    log_gameplay_warn(format!(
                        "warp.failed target={target:?} query_radius={} reason=no_generated_destination",
                        WARP_QUERY_RADIUS_BLOCKS
                    ));
                    pending.fail();
                    return;
                };
                log_gameplay_event(format!(
                    "warp.destination_prepared target={target:?} generated={generated:?} query_radius={}",
                    WARP_QUERY_RADIUS_BLOCKS
                ));
                pending.prepare_surface(generated);
                return;
            }
        }
    };

    match result {
        WarpSearchResult::Pending => {}
        WarpSearchResult::Found(destination) => {
            let (transform, walking, flight, gravity, swimming, _, _, _) = &mut *player;
            transform.translation = destination;
            walking.reset_motion();
            flight.reset_motion();
            gravity.reset_motion();
            swimming.reset_motion();
            let feet = (destination - Vec3::Y * PLAYER_EYE_HEIGHT)
                .floor()
                .as_ivec3();
            pending.clear_request();
            log_gameplay_event(format!(
                "warp.success target={:?} destination={:?}",
                target, feet
            ));
            pending.outcome = Some(WarpOutcome::Succeeded(feet));
        }
        WarpSearchResult::Exhausted => {
            log_gameplay_warn(format!(
                "warp.failed target={:?} query_radius={} runtime_validation_radius={} reason=no_safe_destination",
                target, WARP_QUERY_RADIUS_BLOCKS, WARP_RUNTIME_VALIDATION_RADIUS_BLOCKS
            ));
            pending.fail();
        }
    }
}

fn advance_local_runtime_validation(
    world: &VoxelWorld,
    target: IVec3,
    search: &mut WarpSearchState,
) -> WarpSearchResult {
    search.ensure_started();

    loop {
        let Some(offset) = search.pop_nearest() else {
            return WarpSearchResult::Exhausted;
        };

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
    let minimum = (bounds.0 + Vec3::splat(BOUNDS_EPSILON)).floor().as_ivec3();
    let maximum = (bounds.1 - Vec3::splat(BOUNDS_EPSILON)).floor().as_ivec3();

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
    fn runtime_validation_visits_offsets_by_non_decreasing_distance() {
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
    fn unloaded_candidate_can_be_requeued_without_expanding_validation() {
        let mut search = WarpSearchState::default();
        search.ensure_started();
        let candidate = search.pop_nearest().expect("origin candidate must exist");
        assert_eq!(candidate, IVec3::ZERO);

        search.requeue(candidate);

        assert_eq!(search.pop_nearest(), Some(IVec3::ZERO));
        assert_eq!(search.frontier.len(), 0);
    }

    #[test]
    fn runtime_validation_never_enqueues_offsets_outside_small_cube() {
        let mut search = WarpSearchState::default();
        search.ensure_started();
        search.enqueue(IVec3::new(
            WARP_RUNTIME_VALIDATION_RADIUS_BLOCKS + 1,
            0,
            0,
        ));
        assert_eq!(search.frontier.len(), 1);
    }

    #[test]
    fn dimension_warp_does_not_stream_the_old_dimension() {
        let mut pending = PendingWarp::default();
        pending.request(IVec3::new(15, 64, 15), Some("asteria:umbral"));

        assert_eq!(pending.streaming_center(), None);
        assert_eq!(pending.streaming_horizontal_radius(), None);
    }

    #[test]
    fn prepared_surface_retargets_streaming_directly() {
        let mut pending = PendingWarp::default();
        let requested = IVec3::new(15, 10, 15);
        let generated = IVec3::new(80, 70, -40);
        pending.request(requested, None);
        let requested_center = pending.streaming_center();

        pending.prepare_surface(generated);

        assert_ne!(pending.streaming_center(), requested_center);
        assert_eq!(
            pending.streaming_center(),
            Some(chunk_coord_from_world(generated).with_y(
                chunk_coord_from_world(generated).y.max(0)
            ))
        );
        assert_eq!(
            pending.streaming_horizontal_radius(),
            Some(WARP_STREAMING_HORIZONTAL_RADIUS_CHUNKS)
        );
    }

    #[test]
    fn requesting_a_new_warp_resets_prepared_destination_and_validation() {
        let mut pending = PendingWarp::default();
        pending.request(IVec3::new(10, 20, 30), None);
        pending.prepare_surface(IVec3::new(12, 64, 34));
        pending.search.ensure_started();
        let origin = pending
            .search
            .pop_nearest()
            .expect("origin candidate must exist before reset");
        pending.search.expand_from(origin);

        pending.request(IVec3::new(-4, 7, 9), None);

        assert_eq!(pending.target, Some(IVec3::new(-4, 7, 9)));
        assert!(pending.dimension.is_none());
        assert!(pending.prepared_surface.is_none());
        assert!(pending.search.frontier.is_empty());
        assert!(pending.search.visited.is_empty());
        assert!(pending.outcome.is_none());
    }
}
