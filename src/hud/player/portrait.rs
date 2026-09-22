use bevy::{
    camera::{CameraOutputMode, RenderTarget, visibility::RenderLayers},
    ecs::system::SystemParam,
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::render_resource::TextureFormat,
    world_serialization::WorldInstanceReady,
};

use crate::{
    content::player::PlayerDefinition,
    player::PLAYER_SKIN_TEXTURE_PATH,
};

const PLAYER_PREVIEW_RENDER_LAYER: usize = 3;
const PLAYER_PREVIEW_SIZE: u32 = 128;
const PLAYER_PORTRAIT_CENTER_Y: f32 = 1.27;
const PLAYER_PORTRAIT_CAMERA_DISTANCE: f32 = 1.52;
const PREVIEW_RENDER_FRAMES: u8 = 6;

#[derive(Resource, Default)]
pub(crate) struct PlayerPreviewImages {
    image: Option<Handle<Image>>,
}

impl PlayerPreviewImages {
    pub(crate) fn portrait(&self) -> Option<Handle<Image>> {
        self.image.clone()
    }
}

#[derive(Resource, Default)]
pub(crate) struct PlayerPreviewRenderState {
    portrait_frames: u8,
}

impl PlayerPreviewRenderState {
    pub(crate) fn request_portrait(&mut self) {
        self.portrait_frames = self.portrait_frames.max(PREVIEW_RENDER_FRAMES);
    }
}

#[derive(Component)]
pub(super) struct PlayerPreviewCamera;

#[derive(Component)]
pub(super) struct PlayerPreviewModel {
    gltf: Handle<Gltf>,
    scene_attached: bool,
}

#[derive(Component)]
struct PlayerPreviewAppearance;

pub(super) fn spawn_player_preview_renderer(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut preview_images: ResMut<PlayerPreviewImages>,
    definition: Res<PlayerDefinition>,
    asset_server: Res<AssetServer>,
) {
    if preview_images.image.is_some() {
        return;
    }

    let image = images.add(Image::new_target_texture(
        PLAYER_PREVIEW_SIZE,
        PLAYER_PREVIEW_SIZE,
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
        Transform::from_xyz(
            0.0,
            PLAYER_PORTRAIT_CENTER_Y,
            PLAYER_PORTRAIT_CAMERA_DISTANCE,
        )
        .looking_at(
            Vec3::new(0.0, PLAYER_PORTRAIT_CENTER_Y, 0.0),
            Vec3::Y,
        ),
        RenderLayers::layer(PLAYER_PREVIEW_RENDER_LAYER),
    ));

    let Some(model_path) = definition.model.as_ref() else {
        warn!("player definition has no model configured for shared HUD preview");
        return;
    };

    commands.spawn((
        PlayerPreviewModel {
            gltf: asset_server.load(model_path.clone()),
            scene_attached: false,
        },
        Transform::default(),
        Visibility::Inherited,
        RenderLayers::layer(PLAYER_PREVIEW_RENDER_LAYER),
    ));
}

pub(super) fn attach_player_preview_model(
    mut commands: Commands,
    mut previews: Query<(Entity, &mut PlayerPreviewModel)>,
    gltfs: Res<Assets<Gltf>>,
) {
    for (entity, mut preview) in &mut previews {
        if preview.scene_attached {
            continue;
        }
        let Some(gltf) = gltfs.get(&preview.gltf) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone() else {
            warn!("shared player HUD preview model has no default scene");
            preview.scene_attached = true;
            continue;
        };

        commands.entity(entity).with_children(|parent| {
            parent
                .spawn((
                    WorldAssetRoot(scene),
                    Transform::default(),
                    PlayerPreviewAppearance,
                ))
                .observe(configure_player_preview_scene);
        });
        preview.scene_attached = true;
    }
}

#[derive(SystemParam)]
struct PlayerPreviewSceneAssets<'w, 's> {
    appearances: Query<'w, 's, (), With<PlayerPreviewAppearance>>,
    mesh_entities: Query<'w, 's, &'static Mesh3d>,
    meshes: Res<'w, Assets<Mesh>>,
    mesh_materials: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

fn configure_player_preview_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    mut assets: PlayerPreviewSceneAssets,
    mut render_state: ResMut<PlayerPreviewRenderState>,
) {
    if assets.appearances.get(ready.entity).is_err() {
        return;
    }

    for descendant in descendants.iter_descendants(ready.entity) {
        if let Ok(mesh_handle) = assets.mesh_entities.get(descendant) {
            if assets
                .meshes
                .get(mesh_handle.id())
                .is_some_and(|mesh| mesh.get_vertex_buffer_size() == 0)
            {
                commands
                    .entity(descendant)
                    .remove::<Mesh3d>()
                    .remove::<MeshMaterial3d<StandardMaterial>>();
                continue;
            }
            commands.entity(descendant).insert((
                RenderLayers::layer(PLAYER_PREVIEW_RENDER_LAYER),
                NotShadowCaster,
                NotShadowReceiver,
            ));
        }

        let Ok(original) = assets.mesh_materials.get(descendant) else {
            continue;
        };
        let Some(mut material) = assets.materials.get(original.id()).cloned() else {
            continue;
        };
        material.base_color = Color::WHITE;
        material.base_color_texture = Some(assets.asset_server.load(PLAYER_SKIN_TEXTURE_PATH));
        material.unlit = true;
        material.metallic = 0.0;
        material.perceptual_roughness = 1.0;
        material.reflectance = 0.0;
        material.emissive = LinearRgba::BLACK;
        material.emissive_texture = None;
        let material = assets.materials.add(material);
        commands
            .entity(descendant)
            .insert(MeshMaterial3d(material));
    }

    render_state.request_portrait();
}

pub(super) fn render_player_preview(
    mut render_state: ResMut<PlayerPreviewRenderState>,
    mut cameras: Query<&mut Camera, With<PlayerPreviewCamera>>,
) {
    let should_render = render_state.portrait_frames > 0;

    for mut camera in &mut cameras {
        camera.output_mode = if should_render {
            CameraOutputMode::Write {
                blend_state: None,
                clear_color: ClearColorConfig::Custom(Color::NONE),
            }
        } else {
            CameraOutputMode::Skip
        };
    }

    if should_render {
        render_state.portrait_frames = render_state.portrait_frames.saturating_sub(1);
    }
}
