use std::collections::HashSet;

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::render_resource::TextureFormat,
};

use crate::{
    app::game_state::GameState,
    player::model::{PlayerModelPreviewSource, PlayerModelRoot},
    rendering::block_model_material::BlockModelMaterial,
};

const PLAYER_PREVIEW_RENDER_LAYER: usize = 3;
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
struct PlayerPreviewProxy {
    source: Entity,
}

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
        Camera3d::default(),
        Camera {
            order: -2,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Image(image.into()),
        Projection::Perspective(PerspectiveProjection {
            aspect_ratio: PLAYER_PREVIEW_WIDTH as f32 / PLAYER_PREVIEW_HEIGHT as f32,
            ..default()
        }),
        Transform::from_xyz(
            0.0,
            PLAYER_PREVIEW_CENTER_Y,
            PLAYER_PREVIEW_CAMERA_DISTANCE,
        )
        .looking_at(Vec3::new(0.0, PLAYER_PREVIEW_CENTER_Y, 0.0), Vec3::Y),
        RenderLayers::layer(PLAYER_PREVIEW_RENDER_LAYER),
    ));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sync_player_preview_proxies(
    mut commands: Commands,
    roots: Query<&GlobalTransform, With<PlayerModelRoot>>,
    sources: Query<(&GlobalTransform, &Visibility), With<PlayerModelPreviewSource>>,
    standard_sources: Query<
        (Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>, &GlobalTransform, &Visibility),
        With<PlayerModelPreviewSource>,
    >,
    block_sources: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<BlockModelMaterial>,
            &GlobalTransform,
            &Visibility,
        ),
        With<PlayerModelPreviewSource>,
    >,
    mut proxies: Query<(Entity, &PlayerPreviewProxy, &mut Transform, &mut Visibility)>,
) {
    let Some(root) = roots.iter().next() else {
        for (entity, _, _, _) in &mut proxies {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut represented = HashSet::new();
    for (entity, proxy, mut transform, mut visibility) in &mut proxies {
        let Ok((source_transform, source_visibility)) = sources.get(proxy.source) else {
            commands.entity(entity).despawn();
            continue;
        };

        represented.insert(proxy.source);
        *transform = source_transform.reparented_to(root);
        *visibility = preview_visibility(*source_visibility);
    }

    for (source, mesh, material, source_transform, source_visibility) in &standard_sources {
        if represented.contains(&source) {
            continue;
        }
        commands.spawn((
            PlayerPreviewProxy { source },
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(material.0.clone()),
            source_transform.reparented_to(root),
            preview_visibility(*source_visibility),
            RenderLayers::layer(PLAYER_PREVIEW_RENDER_LAYER),
            NotShadowCaster,
            NotShadowReceiver,
            DespawnOnExit(GameState::Gameplay),
        ));
        represented.insert(source);
    }

    for (source, mesh, material, source_transform, source_visibility) in &block_sources {
        if represented.contains(&source) {
            continue;
        }
        commands.spawn((
            PlayerPreviewProxy { source },
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(material.0.clone()),
            source_transform.reparented_to(root),
            preview_visibility(*source_visibility),
            RenderLayers::layer(PLAYER_PREVIEW_RENDER_LAYER),
            NotShadowCaster,
            NotShadowReceiver,
            DespawnOnExit(GameState::Gameplay),
        ));
        represented.insert(source);
    }
}

const fn preview_visibility(source: Visibility) -> Visibility {
    match source {
        Visibility::Hidden => Visibility::Hidden,
        Visibility::Inherited | Visibility::Visible => Visibility::Visible,
    }
}
