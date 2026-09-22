use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::render_resource::TextureFormat,
    world_serialization::WorldInstanceReady,
};

use crate::{
    app::game_state::GameState,
    content::player::PlayerDefinition,
    player::PLAYER_SKIN_TEXTURE_PATH,
};

const PLAYER_PORTRAIT_RENDER_LAYER: usize = 3;
const PLAYER_PORTRAIT_CENTER_Y: f32 = 1.27;
const PLAYER_PORTRAIT_CAMERA_DISTANCE: f32 = 1.52;

#[derive(Component)]
pub(super) struct PlayerPortraitCamera;

#[derive(Component)]
struct PlayerPortraitModel {
    gltf: Handle<Gltf>,
    scene_attached: bool,
}

#[derive(Component)]
struct PlayerPortraitAppearance;

pub(super) fn spawn_player_portrait(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    definition: &PlayerDefinition,
    asset_server: &AssetServer,
) -> Handle<Image> {
    let image = images.add(Image::new_target_texture(
        128,
        128,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));

    commands.spawn((
        PlayerPortraitCamera,
        Camera3d::default(),
        Camera {
            order: -2,
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        },
        RenderTarget::Image(image.clone().into()),
        Transform::from_xyz(
            0.0,
            PLAYER_PORTRAIT_CENTER_Y,
            PLAYER_PORTRAIT_CAMERA_DISTANCE,
        )
        .looking_at(
            Vec3::new(0.0, PLAYER_PORTRAIT_CENTER_Y, 0.0),
            Vec3::Y,
        ),
        RenderLayers::layer(PLAYER_PORTRAIT_RENDER_LAYER),
        DespawnOnExit(GameState::Gameplay),
    ));

    if let Some(model_path) = definition.model.as_ref() {
        commands.spawn((
            PlayerPortraitModel {
                gltf: asset_server.load(model_path.clone()),
                scene_attached: false,
            },
            Transform::default(),
            Visibility::Inherited,
            RenderLayers::layer(PLAYER_PORTRAIT_RENDER_LAYER),
            DespawnOnExit(GameState::Gameplay),
        ));
    } else {
        warn!("player definition has no model configured for the HUD portrait");
    }

    image
}

pub(super) fn attach_player_portrait_model(
    mut commands: Commands,
    mut portraits: Query<(Entity, &mut PlayerPortraitModel)>,
    gltfs: Res<Assets<Gltf>>,
) {
    for (entity, mut portrait) in &mut portraits {
        if portrait.scene_attached {
            continue;
        }
        let Some(gltf) = gltfs.get(&portrait.gltf) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone() else {
            warn!("player HUD portrait model has no default glTF scene");
            portrait.scene_attached = true;
            continue;
        };

        commands.entity(entity).with_children(|parent| {
            parent
                .spawn((
                    WorldAssetRoot(scene),
                    Transform::default(),
                    PlayerPortraitAppearance,
                ))
                .observe(configure_player_portrait_scene);
        });
        portrait.scene_attached = true;
    }
}

fn configure_player_portrait_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    appearances: Query<(), With<PlayerPortraitAppearance>>,
    meshes: Query<(), With<Mesh3d>>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if appearances.get(ready.entity).is_err() {
        return;
    }

    for descendant in descendants.iter_descendants(ready.entity) {
        if meshes.contains(descendant) {
            commands.entity(descendant).insert((
                RenderLayers::layer(PLAYER_PORTRAIT_RENDER_LAYER),
                NotShadowCaster,
                NotShadowReceiver,
            ));
        }

        let Ok(original) = mesh_materials.get(descendant) else {
            continue;
        };
        let Some(mut material) = materials.get(original.id()).cloned() else {
            continue;
        };
        material.base_color = Color::WHITE;
        material.base_color_texture = Some(asset_server.load(PLAYER_SKIN_TEXTURE_PATH));
        material.unlit = true;
        material.metallic = 0.0;
        material.perceptual_roughness = 1.0;
        material.reflectance = 0.0;
        material.emissive = LinearRgba::BLACK;
        material.emissive_texture = None;
        let material = materials.add(material);
        commands
            .entity(descendant)
            .insert(MeshMaterial3d(material));
    }
}
