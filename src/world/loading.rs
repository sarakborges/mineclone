use bevy::{ecs::system::SystemParam, prelude::*};

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
    horizontal_radius: i32,
}

impl Default for WorldLoadingState {
    fn default() -> Self {
        Self {
            center: None,
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
}

#[derive(SystemParam)]
pub(super) struct LoadingDestinationContext<'w, 's> {
    generator: Res<'w, WorldGenerator>,
    player_definition: Res<'w, PlayerDefinition>,
    config: Res<'w, NewWorldConfig>,
    save: ResMut<'w, InMemoryWorldSave>,
    players: Query<'w, 's, (), With<PlayerEntity>>,
    loading: ResMut<'w, WorldLoadingState>,
    progress: ResMut<'w, WorldLoadingProgress>,
}

pub(super) fn prepare_loading_destination(
    mut commands: Commands,
    mut context: LoadingDestinationContext,
) {
    if context.loading.center.is_some() || !context.players.is_empty() {
        return;
    }

    let saved = context.save.player(LOCAL_PLAYER_ID);
    let saved_position = saved.and_then(|player| player.position());
    let target_eye = saved_position.unwrap_or_else(|| {
        find_safe_spawn_position(&context.generator, IVec2::ZERO, |_| true)
            .expect("world generator must provide a safe initial spawn candidate")
    });
    let game_mode = saved.map_or_else(|| context.config.game_mode(), |player| player.game_mode());
    let saved_health = saved.and_then(|player| player.health());
    let saved_look = saved.and_then(|player| player.look());
    let saved_flying = saved.is_some_and(|player| player.flying());

    spawn_player_entity(
        &mut commands,
        target_eye,
        game_mode,
        &context.player_definition,
        saved_health,
        saved_look,
        saved_flying,
    );

    if saved_position.is_none() && context.save.has_world() {
        context.save.save_player_state_with_health(
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
    context.loading.center = Some(center);
    context.progress.phase = WorldLoadingPhase::MaterializingInitialArea;
    context.progress.completed = 0;
    context.progress.total = 0;
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
