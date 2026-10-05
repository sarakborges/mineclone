// Player restore/spawn entry points are temporarily dormant while Phase 1 has
// no world activation path. The gameplay implementations remain intact for the
// query-first spawn/warp integration instead of being replaced with legacy shims.
#[allow(dead_code)]
pub(crate) mod camera;
pub(crate) mod character_info;
pub(crate) mod game_mode;
mod held_sprite;
pub(crate) mod hotbar;
pub(crate) mod inventory;
pub(crate) mod item_stack;
mod material;
pub(crate) mod model;
#[allow(dead_code)]
pub(crate) mod movement;
pub(crate) mod player_id;
#[allow(dead_code)]
pub(crate) mod save;
pub(crate) mod skin_uv;
pub(crate) mod viewmodel;

use bevy::{
    camera::{CameraOutputMode, Hdr},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
};

use crate::{
    app::{crash_log::log_gameplay_event, game_state::GameState},
    content::player::PlayerDefinition,
    entity::EntityHealth,
    rendering::camera_stack::WORLD_CAMERA_ORDER,
    voxel::world::VoxelWorld,
    world::{destination::find_generated_surface_destination, generator::WorldGenerator},
};
use camera::{GameplayCamera, GameplayWorldCamera};
use game_mode::GameMode;
use movement::{
    flight::FlightState, gravity::GravityState, swimming::SwimmingState, walking::WalkingState,
};
use player_id::LOCAL_PLAYER_ID;

pub(crate) use material::apply_player_skin_material;

pub(crate) const PLAYER_HEIGHT: f32 = 1.8;
pub(crate) const PLAYER_EYE_HEIGHT: f32 = 1.62;
pub(crate) const PLAYER_HALF_WIDTH: f32 = 0.3;
pub(crate) const PLAYER_DISPLAY_NAME: &str = "Yogg'Sara";

#[derive(Component, Default)]
pub(crate) struct PlayerEntity;

#[allow(dead_code)]
const SPAWN_SEARCH_RADIUS_BLOCKS: i32 = 64;

#[allow(dead_code)]
pub(crate) fn spawn_player_entity(
    commands: &mut Commands,
    translation: Vec3,
    game_mode: GameMode,
    definition: &PlayerDefinition,
    saved_health: Option<f32>,
    saved_look: Option<(f32, f32)>,
    saved_flying: bool,
) {
    let gameplay_camera = saved_look.map_or_else(GameplayCamera::default, |(yaw, pitch)| {
        GameplayCamera::restored(yaw, pitch)
    });
    let transform =
        Transform::from_translation(translation).with_rotation(gameplay_camera.rotation());
    log_gameplay_event(format!(
        "entity.spawn type=player id={:?} mode={:?} position={:?} health={:.3}",
        LOCAL_PLAYER_ID,
        game_mode,
        translation,
        saved_health.unwrap_or(definition.health)
    ));
    commands
        .spawn((
            PlayerEntity,
            transform,
            Visibility::Inherited,
            gameplay_camera,
            LOCAL_PLAYER_ID,
            game_mode,
            WalkingState::default(),
            FlightState::restored(saved_flying && game_mode.allows_flight()),
            GravityState::default(),
            SwimmingState::default(),
            DespawnOnExit(GameState::Gameplay),
            Name::new("Player"),
            saved_health.map_or_else(
                || EntityHealth::new(definition.health),
                |health| EntityHealth::restored(definition.health, health),
            ),
        ))
        .with_children(|player| {
            player.spawn((
                GameplayWorldCamera,
                Camera3d::default(),
                Camera {
                    // The player is spawned while Loading is still covered by
                    // the transition UI. Keep the world camera active so Bevy
                    // can prepare the 3D view before the first Gameplay frame.
                    is_active: true,
                    order: WORLD_CAMERA_ORDER,
                    output_mode: CameraOutputMode::Skip,
                    ..default()
                },
                Hdr,
                Tonemapping::None,
                Msaa::Off,
                Transform::default(),
            ));
        });
}

#[allow(dead_code)]
pub(crate) fn player_position_is_clear(world: &VoxelWorld, translation: Vec3) -> bool {
    let feet = translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let feet_voxel = feet.floor().as_ivec3();
    let head_voxel = feet_voxel + IVec3::Y;

    world.is_loaded_at(feet_voxel)
        && world.is_loaded_at(head_voxel)
        && !world.is_solid(feet_voxel)
        && !world.is_solid(head_voxel)
        && world.fluid_at(feet_voxel).is_none()
        && world.fluid_at(head_voxel).is_none()
}

#[allow(dead_code)]
pub(crate) fn find_safe_spawn_position(
    generator: &WorldGenerator,
    preferred_column: IVec2,
    accepts_column: impl FnMut(IVec2) -> bool,
) -> Option<Vec3> {
    let feet = find_generated_surface_destination(
        generator,
        preferred_column,
        SPAWN_SEARCH_RADIUS_BLOCKS,
        accepts_column,
    )?;
    Some(Vec3::new(
        feet.x as f32 + 0.5,
        feet.y as f32 + PLAYER_EYE_HEIGHT,
        feet.z as f32 + 0.5,
    ))
}

#[allow(dead_code)]
pub(crate) fn safe_spawn_position(generator: &WorldGenerator, preferred_column: IVec2) -> Vec3 {
    find_safe_spawn_position(generator, preferred_column, |_| true).unwrap_or_else(|| {
        panic!(
            "could not find a safe generated player spawn within {} blocks of {:?}",
            SPAWN_SEARCH_RADIUS_BLOCKS, preferred_column
        )
    })
}
