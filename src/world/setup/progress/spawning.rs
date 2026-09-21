use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{
        find_safe_spawn_position, player_id::LOCAL_PLAYER_ID, player_position_is_clear,
        safe_spawn_position, spawn_player_entity,
    },
    ui::transition::{ScreenTransition, ScreenTransitionTarget},
    world::{
        WorldLoadMode,
        chunk_system_params::{ChunkContent, ChunkRenderer},
    },
};

use super::super::system_params::{WorldSetupPersistence, WorldSetupProgress};

pub(super) fn spawn_loaded_world(
    content: &ChunkContent<'_>,
    renderer: &mut ChunkRenderer<'_, '_>,
    progress: &mut WorldSetupProgress<'_>,
    transition: &mut ScreenTransition,
    persistence: &WorldSetupPersistence<'_>,
    player_definition: &crate::content::player::PlayerDefinition,
) {
    if progress.loading_state.transition_requested {
        return;
    }

    let saved_position = (*persistence.load_mode == WorldLoadMode::Load)
        .then(|| persistence.save.player_position(LOCAL_PLAYER_ID))
        .flatten();
    let translation = saved_position
        .filter(|position| player_position_is_clear(&progress.world, *position))
        .unwrap_or_else(|| spawn_position(content, progress, persistence));
    let game_mode = if *persistence.load_mode == WorldLoadMode::Load {
        persistence.save.player_game_mode(LOCAL_PLAYER_ID)
    } else {
        persistence.new_world_config.game_mode()
    };

    let saved_health = (*persistence.load_mode == WorldLoadMode::Load)
        .then(|| persistence.save.player_health(LOCAL_PLAYER_ID))
        .flatten();
    let saved_look = (*persistence.load_mode == WorldLoadMode::Load)
        .then(|| persistence.save.player_look(LOCAL_PLAYER_ID))
        .flatten();
    let saved_flying = *persistence.load_mode == WorldLoadMode::Load
        && persistence.save.player_flying(LOCAL_PLAYER_ID);
    spawn_player_entity(
        &mut renderer.commands,
        translation,
        game_mode,
        player_definition,
        saved_health,
        saved_look,
        saved_flying,
    );
    progress.loading_state.transition_requested = true;
    transition.request(ScreenTransitionTarget::game(GameState::Gameplay));
}

fn spawn_position(
    content: &ChunkContent<'_>,
    progress: &WorldSetupProgress<'_>,
    persistence: &WorldSetupPersistence<'_>,
) -> Vec3 {
    let preferred_column = progress.loading_state.spawn_column;
    let Some(forced_biome) = (*persistence.load_mode == WorldLoadMode::New)
        .then(|| persistence.new_world_config.spawn_biome())
        .flatten()
    else {
        return safe_spawn_position(&progress.world, preferred_column);
    };

    find_safe_spawn_position(&progress.world, preferred_column, |column| {
        content
            .biome_field
            .sample_surface(column.as_vec2() + Vec2::splat(0.5))
            .primary_id
            == forced_biome
    })
    .unwrap_or_else(|| {
        panic!(
            "could not find a safe generated player spawn in biome {forced_biome} near {preferred_column:?}"
        )
    })
}
