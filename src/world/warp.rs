use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    time::{Duration, Instant},
};

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{
        crash_log::{log_gameplay_event, log_gameplay_warn},
        game_state::GameState,
    },
    content::{
        biome::{BiomeKind, BiomeRegistry},
        dimension::{DimensionDefinition, DimensionRegistry},
    },
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
        chunk::CHUNK_SIZE,
        collision::collides_aabb,
        coordinates::chunk_coord_from_world,
        world::VoxelWorld,
    },
};

use super::{
    InMemoryWorldSave, WorldLoadMode,
    dimension::{CurrentDimension, DimensionId},
    dimension_persistence::DimensionRuntimeContext,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WarpOutcome {
    Succeeded(IVec3),
    Failed,
}

#[derive(Resource, Default)]
pub(crate) struct PendingWarp {
    target: Option<IVec3>,
    dimension: Option<String>,
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
        self.target.map(|target| {
            let mut coord = chunk_coord_from_world(target);
            coord.y = coord.y.max(0);
            coord
        })
    }

    pub(super) fn streaming_radii(&self) -> Option<(i32, i32)> {
        if self.dimension.is_some() {
            return None;
        }
        self.target.map(|_| {
            let chunk_size = CHUNK_SIZE as i32;
            let search_radius_blocks = self.search.radius.max(0);
            let search_radius_chunks = ((search_radius_blocks + chunk_size - 1) / chunk_size)
                .max(WARP_STREAMING_MIN_RADIUS_CHUNKS);
            (search_radius_chunks, search_radius_chunks)
        })
    }

    fn fail(&mut self) {
        self.target = None;
        self.dimension = None;
        self.search.reset();
        self.outcome = Some(WarpOutcome::Failed);
    }
}

#[derive(Resource)]
pub(crate) struct PendingDimensionWarp {
    target: IVec3,
}

#[derive(SystemParam)]
pub(super) struct DimensionWarpContext<'w, 's> {
    commands: Commands<'w, 's>,
    dimensions: Res<'w, DimensionRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    current_dimension: ResMut<'w, CurrentDimension>,
    load_mode: ResMut<'w, WorldLoadMode>,
    save: ResMut<'w, InMemoryWorldSave>,
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
    mut slow_search_warned: Local<bool>,
    mut player: WarpPlayer,
) {
    let Some(target) = pending.target else {
        *slow_search_warned = false;
        return;
    };

    if let Some(requested_dimension) = pending.dimension.clone() {
        if requested_dimension == dimension.current_dimension.id.as_str() {
            pending.dimension = None;
        } else {
            let Some(definition) = dimension.dimensions.get(&requested_dimension) else {
                log_gameplay_warn(format!(
                    "warp.failed target={target:?} dimension={requested_dimension} reason=unknown_dimension"
                ));
                pending.fail();
                *slow_search_warned = false;
                return;
            };

            let spawn_biome = if let Some(saved) = dimension
                .runtime
                .inactive_spawn_biome(&requested_dimension)
                .map(|spawn_biome| spawn_biome.map(str::to_owned))
            {
                saved
            } else {
                match dimension_warp_spawn_biome(definition, &dimension.biomes, &dimension.save) {
                    Ok(spawn_biome) => spawn_biome,
                    Err(()) => {
                        log_gameplay_warn(format!(
                            "warp.failed target={target:?} dimension={requested_dimension} reason=no_single_biome_candidate"
                        ));
                        pending.fail();
                        *slow_search_warned = false;
                        return;
                    }
                }
            };

            let (_, _, flight, _, _, game_mode, health, camera) = &mut *player;
            let requested_eye = Vec3::new(
                target.x as f32 + 0.5,
                target.y as f32 + PLAYER_EYE_HEIGHT,
                target.z as f32 + 0.5,
            );

            let previous_dimension = dimension.current_dimension.id.clone();
            let previous_spawn_biome = dimension.save.spawn_biome().map(str::to_owned);
            let active_spawn_biome = match dimension.runtime.swap_to(
                &previous_dimension,
                previous_spawn_biome,
                &requested_dimension,
                spawn_biome,
            ) {
                Ok(spawn_biome) => spawn_biome,
                Err(error) => {
                    log_gameplay_warn(format!(
                        "warp.failed target={target:?} dimension={requested_dimension} reason=dimension_state error={error}"
                    ));
                    pending.fail();
                    *slow_search_warned = false;
                    return;
                }
            };

            dimension
                .save
                .prepare_dimension_warp(&requested_dimension, active_spawn_biome.as_deref());
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
            dimension
                .commands
                .insert_resource(PendingDimensionWarp { target });
            dimension.next_game_state.set(GameState::Loading);
            log_gameplay_event(format!(
                "warp.dimension_transition from={} to={} target={:?} next_state=Loading",
                previous_dimension_text, requested_dimension, target
            ));

            pending.target = None;
            pending.dimension = None;
            pending.search.reset();
            pending.outcome = None;
            *slow_search_warned = false;
            return;
        }
    }

    let search_started = Instant::now();
    let result = advance_safe_eye_position_search(dimension.runtime.world(), target, &mut pending.search);
    let search_elapsed = search_started.elapsed();
    if search_elapsed >= SLOW_WARP_SEARCH_WARNING && !*slow_search_warned {
        log_gameplay_warn(format!(
            "warp.search slow target={target:?} radius={} frontier={} visited={} elapsed_ms={:.2}",
            target,
            pending.search.radius,
            pending.search.frontier.len(),
            pending.search.visited.iter().filter(|visited| **visited).count(),
            search_elapsed.as_secs_f64() * 1_000.0
        ));
        *slow_search_warned = true;
    }

    match result {
        WarpSearchResult::Pending => {}
        WarpSearchResult::Found(destination) => {
            let (transform, walking, flight, gravity, swimming, _, _, _) = &mut *player;
            transform.translation = destination;
            walking.reset_motion();
            flight.reset_motion();
            gravity.reset_motion();
            swimming.reset_motion();
            let feet = (destination - Vec3::Y * PLAYER_EYE_HEIGHT).floor().as_ivec3();
            pending.target = None;
            pending.search.reset();
            log_gameplay_event(format!(
                "warp.success target={:?} destination={:?}",
                target, feet
            ));
            pending.outcome = Some(WarpOutcome::Succeeded(feet));
            *slow_search_warned = false;
        }
        WarpSearchResult::Exhausted => {
            warn!(
                "could not find a safe warp destination within {} blocks of {:?}",
                WARP_SEARCH_RADIUS_BLOCKS, target
            );
            pending.target = None;
            pending.search.reset();
            log_gameplay_warn(format!(
                "warp.failed target={:?} search_radius={} reason=no_safe_destination",
                target, WARP_SEARCH_RADIUS_BLOCKS
            ));
            pending.outcome = Some(WarpOutcome::Failed);
            *slow_search_warned = false;
        }
    }
}

pub(super) fn resume_dimension_warp(
    mut commands: Commands,
    dimension_warp: Option<Res<PendingDimensionWarp>>,
    mut pending: ResMut<PendingWarp>,
) {
    let Some(dimension_warp) = dimension_warp else {
        return;
    };
    let target = dimension_warp.target;
    log_gameplay_event(format!(
        "warp.dimension_transition loaded=true target={target:?} resume_safe_search=true"
    ));
    pending.request(target, None);
    commands.remove_resource::<PendingDimensionWarp>();
}

fn dimension_warp_spawn_biome(
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    save: &InMemoryWorldSave,
) -> Result<Option<String>, ()> {
    if !save.world_generation().single_biome() {
        return Ok(None);
    }

    let is_valid = |id: &str| {
        dimension.biomes.iter().any(|entry| {
            entry.id == id
                && entry.weight > f32::EPSILON
                && biomes
                    .get(id)
                    .is_some_and(|biome| biome.kind == BiomeKind::Surface)
        })
    };

    if let Some(id) = save.spawn_biome().filter(|id| is_valid(id)) {
        return Ok(Some(id.to_owned()));
    }

    dimension
        .biomes
        .iter()
        .find(|entry| is_valid(&entry.id))
        .map(|entry| Some(entry.id.clone()))
        .ok_or(())
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
        pending.request(IVec3::new(15, 64, 15), None);

        assert_eq!(pending.streaming_radii(), Some((1, 1)));

        pending.search.radius = CHUNK_SIZE as i32;
        assert_eq!(pending.streaming_radii(), Some((1, 1)));

        pending.search.radius = CHUNK_SIZE as i32 + 1;
        assert_eq!(pending.streaming_radii(), Some((2, 2)));
    }

    #[test]
    fn dimension_warp_does_not_stream_the_old_dimension() {
        let mut pending = PendingWarp::default();
        pending.request(IVec3::new(15, 64, 15), Some("asteria:umbral"));

        assert_eq!(pending.streaming_center(), None);
        assert_eq!(pending.streaming_radii(), None);
    }

    #[test]
    fn requesting_a_new_warp_resets_previous_search_progress() {
        let mut pending = PendingWarp::default();
        pending.request(IVec3::new(10, 20, 30), Some("asteria:umbral"));
        pending.search.ensure_started();
        let origin = pending
            .search
            .pop_nearest()
            .expect("origin candidate must exist before reset");
        pending.search.expand_from(origin);
        pending.search.radius = 8;

        pending.request(IVec3::new(-4, 7, 9), None);

        assert_eq!(pending.target, Some(IVec3::new(-4, 7, 9)));
        assert!(pending.dimension.is_none());
        assert_eq!(pending.search.radius, 0);
        assert!(pending.search.frontier.is_empty());
        assert!(pending.search.visited.is_empty());
        assert!(pending.outcome.is_none());
    }
}
