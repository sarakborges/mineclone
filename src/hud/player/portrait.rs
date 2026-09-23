use bevy::{
    camera::{CameraOutputMode, Viewport, visibility::RenderLayers},
    ecs::query::QueryFilter,
    prelude::*,
    render::render_resource::BlendState,
    window::PrimaryWindow,
};

use crate::{
    app::{
        game_state::GameState,
        pause_state::PauseState,
        settings_state::SettingsState,
    },
    player::{
        character_info::CharacterInfoState,
        model::{
            PLAYER_MODEL_CHARACTER_INFO_RENDER_LAYER,
            PLAYER_MODEL_HUD_RENDER_LAYER,
            PlayerModelRoot,
        },
    },
    rendering::camera_stack::UI_CAMERA_ORDER,
};

const HUD_PREVIEW_CAMERA_ORDER: isize = UI_CAMERA_ORDER + 1;
const CHARACTER_PREVIEW_CAMERA_ORDER: isize = UI_CAMERA_ORDER + 2;
const PLAYER_PREVIEW_CENTER_Y: f32 = 0.90;
const PLAYER_PREVIEW_CAMERA_DISTANCE: f32 = 3.15;

#[derive(Component)]
pub(super) struct PlayerHudPreviewCamera;

#[derive(Component)]
pub(super) struct CharacterInfoPreviewCamera;

#[derive(Component)]
pub(crate) struct PlayerHudPreviewViewport;

#[derive(Component)]
pub(crate) struct CharacterInfoPreviewViewport;

#[derive(Resource, Default)]
pub(crate) struct CharacterPreviewOrbit {
    yaw: f32,
}

impl CharacterPreviewOrbit {
    pub(crate) fn rotate(&mut self, delta_yaw: f32) {
        self.yaw += delta_yaw;
    }
}

type HudPreviewCameraQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Camera, &'static mut Transform),
    (
        With<PlayerHudPreviewCamera>,
        Without<CharacterInfoPreviewCamera>,
    ),
>;

type CharacterPreviewCameraQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Camera, &'static mut Transform),
    (
        With<CharacterInfoPreviewCamera>,
        Without<PlayerHudPreviewCamera>,
    ),
>;

pub(super) fn spawn_player_preview_cameras(mut commands: Commands) {
    commands.spawn((
        PlayerHudPreviewCamera,
        Camera3d::default(),
        Camera {
            order: HUD_PREVIEW_CAMERA_ORDER,
            output_mode: CameraOutputMode::Skip,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(PLAYER_MODEL_HUD_RENDER_LAYER),
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        CharacterInfoPreviewCamera,
        Camera3d::default(),
        Camera {
            order: CHARACTER_PREVIEW_CAMERA_ORDER,
            output_mode: CameraOutputMode::Skip,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(PLAYER_MODEL_CHARACTER_INFO_RENDER_LAYER),
        DespawnOnExit(GameState::Gameplay),
    ));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sync_player_preview_cameras(
    window: Single<&Window, With<PrimaryWindow>>,
    model: Query<&GlobalTransform, With<PlayerModelRoot>>,
    hud_viewport: Query<
        (&ComputedNode, &UiGlobalTransform),
        With<PlayerHudPreviewViewport>,
    >,
    character_viewport: Query<
        (&ComputedNode, &UiGlobalTransform),
        With<CharacterInfoPreviewViewport>,
    >,
    pause: Res<State<PauseState>>,
    settings: Res<State<SettingsState>>,
    character_info: Res<State<CharacterInfoState>>,
    orbit: Res<CharacterPreviewOrbit>,
    mut hud_camera: HudPreviewCameraQuery,
    mut character_camera: CharacterPreviewCameraQuery,
) {
    let Some(model_transform) = model.iter().next() else {
        skip_cameras(&mut hud_camera);
        skip_cameras(&mut character_camera);
        return;
    };

    let hud_visible =
        *pause.get() == PauseState::Running && *settings.get() == SettingsState::Closed;
    let hud_rect = hud_visible
        .then(|| hud_viewport.iter().next())
        .flatten()
        .and_then(|(node, transform)| viewport_from_ui(node, transform, &window));
    sync_camera(
        &mut hud_camera,
        hud_rect,
        model_transform,
        PLAYER_PREVIEW_CENTER_Y,
        PLAYER_PREVIEW_CAMERA_DISTANCE,
        0.0,
    );

    let character_rect = (*character_info.get() == CharacterInfoState::Open)
        .then(|| character_viewport.iter().next())
        .flatten()
        .and_then(|(node, transform)| viewport_from_ui(node, transform, &window));
    sync_camera(
        &mut character_camera,
        character_rect,
        model_transform,
        PLAYER_PREVIEW_CENTER_Y,
        PLAYER_PREVIEW_CAMERA_DISTANCE,
        orbit.yaw,
    );
}

fn sync_camera<F: QueryFilter>(
    cameras: &mut Query<(&mut Camera, &mut Transform), F>,
    viewport: Option<Viewport>,
    model: &GlobalTransform,
    center_y: f32,
    distance: f32,
    orbit_yaw: f32,
) {
    let Some(viewport) = viewport else {
        skip_cameras(cameras);
        return;
    };

    let center = model.translation() + Vec3::Y * center_y;
    let orbit = Quat::from_rotation_y(-orbit_yaw);
    let offset = model.rotation() * orbit * Vec3::Z * distance;
    for (mut camera, mut transform) in cameras.iter_mut() {
        camera.viewport = Some(viewport.clone());
        // Every active preview writes its own scissored viewport. Character Info
        // stays later in the camera stack because that is the last known-good
        // ordering for the large preview; the HUD viewport anchor is now fixed
        // independently and no longer needs to be the final preview camera.
        camera.output_mode = CameraOutputMode::Write {
            blend_state: Some(BlendState::ALPHA_BLENDING),
            clear_color: ClearColorConfig::None,
        };
        *transform = Transform::from_translation(center + offset).looking_at(center, Vec3::Y);
    }
}

fn skip_cameras<F: QueryFilter>(cameras: &mut Query<(&mut Camera, &mut Transform), F>) {
    for (mut camera, _) in cameras.iter_mut() {
        camera.output_mode = CameraOutputMode::Skip;
        camera.viewport = None;
    }
}

fn viewport_from_ui(
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    window: &Window,
) -> Option<Viewport> {
    if node.is_empty() {
        return None;
    }

    let half_size = node.size() * 0.5;
    let window_size = UVec2::new(window.physical_width(), window.physical_height());
    let min = (transform.translation - half_size)
        .round()
        .max(Vec2::ZERO)
        .min(window_size.as_vec2());
    let max = (transform.translation + half_size)
        .round()
        .max(Vec2::ZERO)
        .min(window_size.as_vec2());
    let physical_size = (max - min).as_uvec2();
    if physical_size.x == 0 || physical_size.y == 0 {
        return None;
    }

    Some(Viewport {
        physical_position: min.as_uvec2(),
        physical_size,
        ..default()
    })
}
