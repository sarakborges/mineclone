use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::game_state::GameState;

const MOUSE_SENSITIVITY: f32 = 0.003;
const MAX_PITCH: f32 = 1.54;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), setup_gameplay)
            .add_systems(
                Update,
                (handle_cursor_grab, look_with_mouse).run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct GameplayCamera {
    yaw: f32,
    pitch: f32,
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
        Transform::from_xyz(0.0, 0.0, 6.0),
        GameplayCamera {
            yaw: 0.0,
            pitch: 0.0,
        },
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.35, 0.55, 0.9))),
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 4.0, 4.0),
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
