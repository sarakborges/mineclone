use bevy::{
    camera::{CameraOutputMode, Viewport, visibility::RenderLayers},
    ecs::{query::QueryFilter, system::SystemParam},
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

#[derive(SystemParam)]
struct PlayerPreviewLayout<'w, 's> {
    window: Single<'w, 's, &'static Window, With<PrimaryWindow>>,
    hud_viewport: Query<
        'w,
        's,
        (&'static ComputedNode, &'static UiGlobalTransform),
        With<PlayerHudPreviewViewport>,
    >,
    character_viewport: Query<
        'w,
        's,
        (&'static ComputedNode, &'static UiGlobalTransform),
        With<CharacterInfoPreviewViewport>,
    >,
}

impl PlayerPreviewLayout<'_, '_> {
    fn hud_viewport(&self) -> Option<Viewport> {
        self.hud_viewport
            .iter()
            .next()
            .and_then(|(node, transform)| viewport_from_ui(node, transform, &self.window))
    }

    fn character_viewport(&self) -> Option<Viewport> {
        self.character_viewport
            .iter()
            .next()
            .and_then(|(node, transform)| viewport_from_ui(node, transform, &self.window))
    }
}

#[derive(SystemParam)]
struct PlayerPreviewState<'w> {
    pause: Res<'w, State<PauseState>>,
    settings: Res<'w, State<SettingsState>>,
    character_info: Res<'w, State<CharacterInfoState>>,
    orbit: Res<'w, CharacterPreviewOrbit>,
}

impl PlayerPreviewState<'_> {
    fn hud_visible(&self) -> bool {
        *self.pause.get() == PauseState::Running
            && *self.settings.get() == SettingsState::Closed
    }

    fn character_visible(&self) -> bool {
        *self.character_info.get() == CharacterInfoState::Open
    }

    fn character_yaw(&self) -> f32 {
        self.orbit.yaw
    }
}

pub(super) fn spawn_player_preview_cameras(mut commands: Commands) {
    commands.spawn((
        PlayerHudPreviewCamera,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: HUD_PREVIEW_CAMERA_ORDER,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderLayers::layer(PLAYER_MODEL_HUD_RENDER_LAYER),
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        CharacterInfoPreviewCamera,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: CHARACTER_PREVIEW_CAMERA_ORDER,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderLayers::layer(PLAYER_MODEL_CHARACTER_INFO_RENDER_LAYER),
        DespawnOnExit(GameState::Gameplay),
    ));
}

pub(super) fn sync_player_preview_cameras(
    model: Query<&GlobalTransform, With<PlayerModelRoot>>,
    layout: PlayerPreviewLayout,
    state: PlayerPreviewState,
    mut hud_camera: HudPreviewCameraQuery,
    mut character_camera: CharacterPreviewCameraQuery,
) {
    let Some(model_transform) = model.iter().next() else {
        deactivate_cameras(&mut hud_camera);
        deactivate_cameras(&mut character_camera);
        return;
    };

    let hud_rect = state.hud_visible().then(|| layout.hud_viewport()).flatten();
    sync_camera(
        &mut hud_camera,
        hud_rect,
        model_transform,
        PLAYER_PREVIEW_CENTER_Y,
        PLAYER_PREVIEW_CAMERA_DISTANCE,
        0.0,
    );

    let character_rect = state
        .character_visible()
        .then(|| layout.character_viewport())
        .flatten();
    sync_camera(
        &mut character_camera,
        character_rect,
        model_transform,
        PLAYER_PREVIEW_CENTER_Y,
        PLAYER_PREVIEW_CAMERA_DISTANCE,
        state.character_yaw(),
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
        deactivate_cameras(cameras);
        return;
    };

    let center = model.translation() + Vec3::Y * center_y;
    let orbit = Quat::from_rotation_y(-orbit_yaw);
    let offset = model.rotation() * orbit * Vec3::Z * distance;
    for (mut camera, mut transform) in cameras.iter_mut() {
        camera.is_active = true;
        camera.viewport = Some(viewport.clone());
        *transform = Transform::from_translation(center + offset).looking_at(center, Vec3::Y);
    }
}

fn deactivate_cameras<F: QueryFilter>(
    cameras: &mut Query<(&mut Camera, &mut Transform), F>,
) {
    for (mut camera, _) in cameras.iter_mut() {
        camera.is_active = false;
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
