use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::player::PlayerDefinition,
    player::{
        PLAYER_EYE_HEIGHT, PlayerEntity, find_safe_spawn_position, spawn_player_entity,
        player_id::LOCAL_PLAYER_ID,
    },
    voxel::{coordinates::chunk_coord_from_position, world::VoxelWorld},
};

use super::{
    InMemoryWorldSave, NewWorldConfig,
    generator::WorldGenerator,
    streaming::ChunkStreamingState,
};

const INITIAL_LOADING_HORIZONTAL_RADIUS_CHUNKS: i32 = 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum WorldLoadingPhase {
    #[default]
    PreparingDestination,
    MaterializingInitialArea,
    Ready,
}

#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct WorldLoadingProgress {
    phase: WorldLoadingPhase,
    completed: usize,
    total: usize,
}

impl WorldLoadingProgress {
    pub(crate) const fn phase(&self) -> WorldLoadingPhase {
        self.phase
    }

    pub(crate) const fn completed(&self) -> usize {
        self.completed
    }

    pub(crate) const fn total(&self) -> usize {
        self.total
    }
}

#[derive(Resource, Clone, Debug)]
pub(super) struct WorldLoadingState {
    center: Option<IVec3>,
    target_eye: Option<Vec3>,
    horizontal_radius: i32,
}

impl Default for WorldLoadingState {
    fn default() -> Self {
        Self {
            center: None,
            target_eye: None,
            horizontal_radius: INITIAL_LOADING_HORIZONTAL_RADIUS_CHUNKS,
        }
    }
}

impl WorldLoadingState {
    pub(super) const fn streaming_center(&self) -> Option<IVec3> {
        self.center
    }

    pub(super) const fn horizontal_radius(&self) -> i32 {
        self.horizontal_radius
    }

    pub(crate) const fn target_eye(&self) -> Option<Vec3> {
        self.target_eye
    }
}

pub(super) fn prepare_loading_destination(
    mut commands: Commands,
    generator: Res<WorldGenerator>,
    player_definition: Res<PlayerDefinition>,
    config: Res<NewWorldConfig>,
    mut save: ResMut<InMemoryWorldSave>,
    players: Query<(), With<PlayerEntity>>,
    mut loading: ResMut<WorldLoadingState>,
    mut progress: ResMut<WorldLoadingProgress>,
) {
    if loading.center.is_some() || !players.is_empty() {
        return;
    }

    let saved = save.player(LOCAL_PLAYER_ID);
    let saved_position = saved.and_then(|player| player.position());
    let target_eye = saved_position.unwrap_or_else(|| {
        find_safe_spawn_position(&generator, IVec2::ZERO, |_| true)
            .expect("world generator must provide a safe initial spawn candidate")
    });
    let game_mode = saved.map_or_else(|| config.game_mode(), |player| player.game_mode());
    let saved_health = saved.and_then(|player| player.health());
    let saved_look = saved.and_then(|player| player.look());
    let saved_flying = saved.is_some_and(|player| player.flying());

    spawn_player_entity(
        &mut commands,
        target_eye,
        game_mode,
        &player_definition,
        saved_health,
        saved_look,
        saved_flying,
    );

    if saved_position.is_none() && save.has_world() {
        save.save_player_state_with_health(
            LOCAL_PLAYER_ID,
            target_eye,
            game_mode,
            None,
            None,
            false,
        );
    }

    let feet = target_eye - Vec3::Y * PLAYER_EYE_HEIGHT;
    let mut center = chunk_coord_from_position(feet);
    center.y = center.y.max(0);
    loading.center = Some(center);
    loading.target_eye = Some(target_eye);
    progress.phase = WorldLoadingPhase::MaterializingInitialArea;
    progress.completed = 0;
    progress.total = 0;
}

pub(super) fn finish_loading_when_ready(
    world: Res<VoxelWorld>,
    streaming: Res<ChunkStreamingState>,
    loading: Res<WorldLoadingState>,
    mut progress: ResMut<WorldLoadingProgress>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(center) = loading.streaming_center() else {
        return;
    };
    if streaming.center() != Some(center) {
        return;
    }

    let (completed, total) = streaming.desired_residency_counts(&world);
    progress.phase = WorldLoadingPhase::MaterializingInitialArea;
    progress.completed = completed;
    progress.total = total;

    if total == 0 || completed != total || !streaming.materialization_is_idle() {
        return;
    }

    progress.phase = WorldLoadingPhase::Ready;
    next_state.set(GameState::Gameplay);
}

pub(super) fn world_streaming_active(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::Loading | GameState::Gameplay)
}
