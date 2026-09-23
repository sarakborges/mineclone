use bevy::prelude::*;

use crate::{
    app::{
        game_state::GameState,
        keybinds::{KeybindAction, Keybinds},
        pause_state::PauseState,
    },
    gameplay::availability::world_interaction_available,
    player::{character_info::CharacterInfoState, inventory::InventoryState},
    tools::BrushPaletteState,
    voxel::world::VoxelWorld,
};
use cursor::{capture_cursor, handle_cursor_grab, handle_window_focus, release_cursor};
use look::{MouseLookInputState, drain_or_apply_mouse_look};

type PlayerCameraAnchor<'w, 's> = Single<
    'w,
    's,
    (&'static Transform, &'static GameplayCamera),
    (With<crate::player::PlayerEntity>, Without<GameplayWorldCamera>),
>;
type WorldCameraTransform<'w, 's> = Single<
    'w,
    's,
    &'static mut Transform,
    (With<GameplayWorldCamera>, Without<crate::player::PlayerEntity>),
>;

mod cursor;
pub(crate) mod look;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseLookInputState>()
            .init_resource::<CameraPerspective>()
            .add_systems(OnEnter(GameState::Gameplay), reset_camera_perspective)
            .add_systems(
                OnEnter(GameState::Gameplay),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(OnExit(GameState::Gameplay), release_cursor)
            .add_systems(
                OnEnter(PauseState::Paused),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(InventoryState::Open),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(CharacterInfoState::Open),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(InventoryState::Closed),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(CharacterInfoState::Closed),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Open),
                release_cursor.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Closed),
                capture_cursor.run_if(world_interaction_available),
            )
            .add_systems(
                Update,
                handle_window_focus.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                handle_cursor_grab.run_if(world_interaction_available),
            )
            .add_systems(
                Update,
                (
                    toggle_camera_perspective.run_if(world_interaction_available),
                    drain_or_apply_mouse_look,
                    sync_perspective_camera,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

pub(crate) const MAX_CAMERA_PITCH: f32 = 1.54;

#[derive(Component, Default, Clone, Copy)]
pub struct GameplayCamera {
    pub yaw: f32,
    pub pitch: f32,
}

impl GameplayCamera {
    pub(crate) fn restored(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch: pitch.clamp(-MAX_CAMERA_PITCH, MAX_CAMERA_PITCH) }
    }

    pub(crate) fn rotation(self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }
}


const THIRD_PERSON_MAX_DISTANCE: f32 = 4.0;
const THIRD_PERSON_COLLISION_STEP: f32 = 0.1;
const THIRD_PERSON_MIN_DISTANCE: f32 = 0.35;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraPerspective {
    #[default]
    FirstPerson,
    ThirdPersonBack,
    ThirdPersonFront,
}

impl CameraPerspective {
    pub(crate) const fn is_third_person(self) -> bool {
        matches!(self, Self::ThirdPersonBack | Self::ThirdPersonFront)
    }

    pub(crate) fn horizontal_movement_axes(self, yaw: f32) -> (Vec3, Vec3) {
        let view_yaw = match self {
            Self::ThirdPersonFront => yaw + std::f32::consts::PI,
            Self::FirstPerson | Self::ThirdPersonBack => yaw,
        };
        let input_sign = if matches!(self, Self::ThirdPersonFront) {
            -1.0
        } else {
            1.0
        };
        let rotation = Quat::from_rotation_y(view_yaw);
        (
            rotation * Vec3::NEG_Z * input_sign,
            rotation * Vec3::X * input_sign,
        )
    }

    fn next(self) -> Self {
        match self {
            Self::FirstPerson => Self::ThirdPersonBack,
            Self::ThirdPersonBack => Self::ThirdPersonFront,
            Self::ThirdPersonFront => Self::FirstPerson,
        }
    }
}

#[derive(Component)]
pub(crate) struct GameplayWorldCamera;

fn reset_camera_perspective(mut perspective: ResMut<CameraPerspective>) {
    *perspective = CameraPerspective::FirstPerson;
}

fn toggle_camera_perspective(
    keys: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut perspective: ResMut<CameraPerspective>,
) {
    if keys.just_pressed(keybinds.key_code(KeybindAction::ChangePerspective)) {
        *perspective = perspective.next();
    }
}

fn sync_perspective_camera(
    perspective: Res<CameraPerspective>,
    world: Res<VoxelWorld>,
    player: PlayerCameraAnchor,
    camera: WorldCameraTransform,
) {
    let (player_transform, gameplay_camera) = *player;
    let mut camera_transform = camera.into_inner();

    let (local_direction, local_rotation) = match *perspective {
        CameraPerspective::FirstPerson => {
            if camera_transform.translation != Vec3::ZERO {
                camera_transform.translation = Vec3::ZERO;
            }
            if camera_transform.rotation != Quat::IDENTITY {
                camera_transform.rotation = Quat::IDENTITY;
            }
            return;
        }
        CameraPerspective::ThirdPersonBack => (Vec3::Z, Quat::IDENTITY),
        CameraPerspective::ThirdPersonFront => {
            (Vec3::NEG_Z, Quat::from_rotation_y(std::f32::consts::PI))
        }
    };

    let rotation = gameplay_camera.rotation();
    let world_direction = rotation * local_direction;
    let distance = unobstructed_camera_distance(
        &world,
        player_transform.translation,
        world_direction,
        THIRD_PERSON_MAX_DISTANCE,
    );
    let local_translation = local_direction * distance;
    if camera_transform.translation != local_translation {
        camera_transform.translation = local_translation;
    }
    if camera_transform.rotation != local_rotation {
        camera_transform.rotation = local_rotation;
    }
}

fn unobstructed_camera_distance(
    world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    maximum: f32,
) -> f32 {
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return THIRD_PERSON_MIN_DISTANCE;
    }

    let steps = (maximum / THIRD_PERSON_COLLISION_STEP).ceil() as i32;
    for step in 1..=steps {
        let distance = (step as f32 * THIRD_PERSON_COLLISION_STEP).min(maximum);
        let position = origin + direction * distance;
        if world.is_solid(position.floor().as_ivec3()) {
            return (distance - THIRD_PERSON_COLLISION_STEP)
                .max(THIRD_PERSON_MIN_DISTANCE);
        }
    }

    maximum
}


#[cfg(test)]
mod tests {
    use super::CameraPerspective;
    use bevy::prelude::*;

    #[test]
    fn perspective_cycles_first_back_front_first() {
        assert_eq!(
            CameraPerspective::FirstPerson.next(),
            CameraPerspective::ThirdPersonBack
        );
        assert_eq!(
            CameraPerspective::ThirdPersonBack.next(),
            CameraPerspective::ThirdPersonFront
        );
        assert_eq!(
            CameraPerspective::ThirdPersonFront.next(),
            CameraPerspective::FirstPerson
        );
    }

    #[test]
    fn front_preview_keeps_world_movement_axes_stable() {
        let yaw = 0.73;
        let (back_forward, back_right) =
            CameraPerspective::ThirdPersonBack.horizontal_movement_axes(yaw);
        let (front_forward, front_right) =
            CameraPerspective::ThirdPersonFront.horizontal_movement_axes(yaw);

        assert!(back_forward.abs_diff_eq(front_forward, 1.0e-5));
        assert!(back_right.abs_diff_eq(front_right, 1.0e-5));
    }
}
