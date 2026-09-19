pub(crate) mod camera;
pub(crate) mod game_mode;
pub(crate) mod hotbar;
pub(crate) mod inventory;
pub(crate) mod movement;
pub(crate) mod player_id;
pub(crate) mod save;
pub(crate) mod viewmodel;

use bevy::{
    camera::{CameraOutputMode, Hdr},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::player::PlayerDefinition,
    entity::EntityHealth,
    rendering::camera_stack::WORLD_CAMERA_ORDER,
    voxel::{spatial_search::find_map_square_rings, world::VoxelWorld},
};
use camera::GameplayCamera;
use game_mode::GameMode;
use movement::{
    flight::FlightState, gravity::GravityState, swimming::SwimmingState, walking::WalkingState,
};
use player_id::LOCAL_PLAYER_ID;

pub(crate) const PLAYER_HEIGHT: f32 = 1.8;
pub(crate) const PLAYER_EYE_HEIGHT: f32 = 1.62;
pub(crate) const PLAYER_HALF_WIDTH: f32 = 0.3;

#[derive(Component, Default)]
pub(crate) struct PlayerEntity;

const SPAWN_SEARCH_RADIUS_BLOCKS: i32 = 64;

pub(crate) fn spawn_player_entity(
    commands: &mut Commands,
    translation: Vec3,
    game_mode: GameMode,
    definition: &PlayerDefinition,
    saved_health: Option<f32>,
    saved_look: Option<(f32, f32)>,
) {
    let gameplay_camera = saved_look.map_or_else(GameplayCamera::default, |(yaw, pitch)| GameplayCamera::restored(yaw, pitch));
    let transform = Transform::from_translation(translation).with_rotation(gameplay_camera.rotation());
    commands
        .spawn((
            PlayerEntity,
            Camera3d::default(),
            Camera {
                order: WORLD_CAMERA_ORDER,
                output_mode: CameraOutputMode::Skip,
                ..default()
            },
            Hdr,
            Tonemapping::None,
            Msaa::Off,
            transform,
            gameplay_camera,
            LOCAL_PLAYER_ID,
            game_mode,
            WalkingState::default(),
            FlightState::default(),
            GravityState::default(),
            SwimmingState::default(),
            DespawnOnExit(GameState::Gameplay),
        ))
        .insert((
            Name::new("Player"),
            saved_health.map_or_else(
                || EntityHealth::new(definition.health),
                |health| EntityHealth::restored(definition.health, health),
            ),
        ));
}

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

pub(crate) fn find_safe_spawn_position(
    world: &VoxelWorld,
    preferred_column: IVec2,
    mut accepts_column: impl FnMut(IVec2) -> bool,
) -> Option<Vec3> {
    find_map_square_rings(
        preferred_column,
        SPAWN_SEARCH_RADIUS_BLOCKS,
        1,
        |column| {
            if !accepts_column(column) {
                return None;
            }

            let feet_y = safe_surface_feet_y(world, column)?;
            Some(Vec3::new(
                column.x as f32 + 0.5,
                feet_y as f32 + PLAYER_EYE_HEIGHT,
                column.y as f32 + 0.5,
            ))
        },
    )
}

pub(crate) fn safe_spawn_position(world: &VoxelWorld, preferred_column: IVec2) -> Vec3 {
    find_safe_spawn_position(world, preferred_column, |_| true).unwrap_or_else(|| {
        panic!(
            "could not find a safe generated player spawn within {} blocks of {:?}",
            SPAWN_SEARCH_RADIUS_BLOCKS, preferred_column
        )
    })
}

fn safe_surface_feet_y(world: &VoxelWorld, column: IVec2) -> Option<i32> {
    let highest_y = world.highest_loaded_world_y_in_column(column.x, column.y)?;

    for support_y in (0..=highest_y).rev() {
        let support = IVec3::new(column.x, support_y, column.y);
        if !world.is_solid(support) {
            continue;
        }

        let feet = support + IVec3::Y;
        let head = feet + IVec3::Y;
        if !world.is_loaded_at(head) {
            continue;
        }
        if world.is_solid(feet) || world.is_solid(head) {
            continue;
        }
        if world.fluid_at(feet).is_some() || world.fluid_at(head).is_some() {
            continue;
        }

        return Some(feet.y);
    }

    None
}
