use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::game_state::GameState;

const MOUSE_SENSITIVITY: f32 = 0.003;
const MAX_PITCH: f32 = 1.54;
const WALK_SPEED: f32 = 5.0;
const GRAVITY: f32 = -18.0;
const JUMP_SPEED: f32 = 7.0;
const PLAYER_EYE_HEIGHT: f32 = 1.7;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), setup_gameplay)
            .add_systems(
                Update,
                (handle_cursor_grab, look_with_mouse, move_player)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct GameplayCamera {
    yaw: f32,
    pitch: f32,
}

#[derive(Component)]
struct PlayerMovement {
    vertical_velocity: f32,
    grounded: bool,
}

fn setup_gameplay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, PLAYER_EYE_HEIGHT, 6.0),
        GameplayCamera {
            yaw: 0.0,
            pitch: 0.0,
        },
        PlayerMovement {
            vertical_velocity: 0.0,
            grounded: true,
        },
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 1.0, 40.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.22, 0.18))),
        Transform::from_xyz(0.0, -0.5, 0.0),
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.35, 0.55, 0.9))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn handle_cursor_grab(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }

    if mouse_buttons.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
}

fn look_with_mouse(
    mut mouse_motion: MessageReader<MouseMotion>,
    cursor_options: Single<&CursorOptions>,
    mut camera: Single<(&mut Transform, &mut GameplayCamera)>,
) {
    if cursor_options.grab_mode == CursorGrabMode::None {
        return;
    }

    let delta = mouse_motion.read().map(|motion| motion.delta).sum::<Vec2>();

    if delta == Vec2::ZERO {
        return;
    }

    camera.1.yaw -= delta.x * MOUSE_SENSITIVITY;
    camera.1.pitch = (camera.1.pitch - delta.y * MOUSE_SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);

    camera.0.rotation = Quat::from_euler(EulerRot::YXZ, camera.1.yaw, camera.1.pitch, 0.0);
}

fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player: Single<(&mut Transform, &GameplayCamera, &mut PlayerMovement)>,
) {
    let mut movement = Vec3::ZERO;

    let yaw_rotation = Quat::from_rotation_y(player.1.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;

    if keys.pressed(KeyCode::KeyW) {
        movement += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        movement -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        movement += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        movement -= right;
    }

    if movement.length_squared() > 0.0 {
        player.0.translation += movement.normalize() * WALK_SPEED * time.delta_secs();
    }

    if keys.just_pressed(KeyCode::Space) && player.2.grounded {
        player.2.vertical_velocity = JUMP_SPEED;
        player.2.grounded = false;
    }

    player.2.vertical_velocity += GRAVITY * time.delta_secs();
    player.0.translation.y += player.2.vertical_velocity * time.delta_secs();

    if player.0.translation.y <= PLAYER_EYE_HEIGHT {
        player.0.translation.y = PLAYER_EYE_HEIGHT;
        player.2.vertical_velocity = 0.0;
        player.2.grounded = true;
    }
}
