use bevy::{
    camera::{CameraOutputMode, RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
};

use crate::player::model::{
    PLAYER_MODEL_PREVIEW_RENDER_LAYER,
    PlayerModelRoot,
};

const PLAYER_PREVIEW_WIDTH: u32 = 384;
const PLAYER_PREVIEW_HEIGHT: u32 = 512;
const PLAYER_PREVIEW_CENTER_Y: f32 = 0.9;
const PLAYER_PREVIEW_CAMERA_DISTANCE: f32 = 3.15;

#[derive(Resource, Default)]
pub(crate) struct PlayerPreviewImages {
    image: Option<Handle<Image>>,
}

impl PlayerPreviewImages {
    pub(crate) fn portrait(&self) -> Option<Handle<Image>> {
        self.image.clone()
    }
}

#[derive(Component)]
pub(super) struct PlayerPreviewCamera;

pub(super) fn spawn_player_preview_renderer(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut preview_images: ResMut<PlayerPreviewImages>,
) {
    if preview_images.image.is_some() {
        return;
    }

    let image = images.add(Image::new_target_texture(
        PLAYER_PREVIEW_WIDTH,
        PLAYER_PREVIEW_HEIGHT,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    preview_images.image = Some(image.clone());

    commands.spawn((
        PlayerPreviewCamera,
        Camera3d::default(),
        Camera {
            order: -2,
            output_mode: CameraOutputMode::Skip,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Image(image.into()),
        Projection::Perspective(PerspectiveProjection {
            aspect_ratio: PLAYER_PREVIEW_WIDTH as f32 / PLAYER_PREVIEW_HEIGHT as f32,
            ..default()
        }),
        Transform::default(),
        RenderLayers::layer(PLAYER_MODEL_PREVIEW_RENDER_LAYER),
    ));
}

pub(super) fn render_player_preview(
    models: Query<&GlobalTransform, With<PlayerModelRoot>>,
    mut cameras: Query<(&mut Camera, &mut Transform), With<PlayerPreviewCamera>>,
) {
    let Some(model_transform) = models.iter().next() else {
        for (mut camera, _) in &mut cameras {
            camera.output_mode = CameraOutputMode::Skip;
        }
        return;
    };

    let model_translation = model_transform.translation();
    let model_rotation = model_transform.rotation();
    let center = model_translation + Vec3::Y * PLAYER_PREVIEW_CENTER_Y;
    let offset = model_rotation * Vec3::Z * PLAYER_PREVIEW_CAMERA_DISTANCE;

    for (mut camera, mut transform) in &mut cameras {
        *transform = Transform::from_translation(center + offset).looking_at(center, Vec3::Y);
        camera.output_mode = CameraOutputMode::Write {
            blend_state: None,
            clear_color: ClearColorConfig::Custom(Color::NONE),
        };
    }
}
