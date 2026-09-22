use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::system::SystemParam,
    input::mouse::MouseMotion,
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::render_resource::TextureFormat,
    world_serialization::WorldInstanceReady,
};

use crate::{
    app::game_state::GameState,
    content::player::PlayerDefinition,
    player::{
        PLAYER_DISPLAY_NAME, PLAYER_SKIN_TEXTURE_PATH,
        character_info::CharacterInfoState,
    },
    ui::{surface, theme, typography},
};

const CHARACTER_PREVIEW_RENDER_LAYER: usize = 4;
const CHARACTER_PREVIEW_WIDTH: u32 = 384;
const CHARACTER_PREVIEW_HEIGHT: u32 = 512;
const CHARACTER_PREVIEW_CAMERA_Y: f32 = 0.9;
const CHARACTER_PREVIEW_CAMERA_DISTANCE: f32 = 3.15;
const CHARACTER_PREVIEW_CARD_WIDTH: f32 = 224.0;
const CHARACTER_PREVIEW_CARD_HEIGHT: f32 = 298.0;
const CHARACTER_PREVIEW_DRAG_SENSITIVITY: f32 = 0.01;

#[derive(Resource, Default)]
struct CharacterPreviewImage(Option<Handle<Image>>);

#[derive(Resource, Default)]
struct CharacterPreviewInteraction {
    dragging: bool,
    yaw: f32,
    render_frames: u8,
}

#[derive(Component)]
struct CharacterPreviewCamera;

#[derive(Component)]
struct CharacterPreviewModel {
    gltf: Handle<Gltf>,
    scene_attached: bool,
}

#[derive(Component)]
struct CharacterPreviewAppearance;

#[derive(Component)]
struct CharacterInfoRoot;

#[derive(Component)]
struct CharacterPreviewViewport;

pub(super) struct CharacterInfoHudPlugin;

impl Plugin for CharacterInfoHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterPreviewImage>()
            .init_resource::<CharacterPreviewInteraction>()
            .add_systems(PostStartup, spawn_character_preview)
            .add_systems(
                OnEnter(CharacterInfoState::Open),
                spawn_character_info.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(OnExit(CharacterInfoState::Open), stop_character_preview_drag)
            .add_systems(
                Update,
                (attach_character_preview_model, sync_character_preview_camera).chain(),
            )
            .add_systems(
                Update,
                rotate_character_preview
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(CharacterInfoState::Open)),
            );
    }
}

fn spawn_character_preview(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut preview_image: ResMut<CharacterPreviewImage>,
    definition: Res<PlayerDefinition>,
    asset_server: Res<AssetServer>,
) {
    let image = images.add(Image::new_target_texture(
        CHARACTER_PREVIEW_WIDTH,
        CHARACTER_PREVIEW_HEIGHT,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    preview_image.0 = Some(image.clone());

    commands.spawn((
        CharacterPreviewCamera,
        Camera3d::default(),
        Camera {
            order: -3,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderTarget::Image(image.into()),
        Transform::from_xyz(
            0.0,
            CHARACTER_PREVIEW_CAMERA_Y,
            CHARACTER_PREVIEW_CAMERA_DISTANCE,
        )
        .looking_at(Vec3::new(0.0, CHARACTER_PREVIEW_CAMERA_Y, 0.0), Vec3::Y),
        RenderLayers::layer(CHARACTER_PREVIEW_RENDER_LAYER),
    ));

    let Some(model_path) = definition.model.as_ref() else {
        warn!("player definition has no model configured for Character Info");
        return;
    };

    commands.spawn((
        CharacterPreviewModel {
            gltf: asset_server.load(model_path.clone()),
            scene_attached: false,
        },
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(CHARACTER_PREVIEW_RENDER_LAYER),
    ));
}

fn attach_character_preview_model(
    mut commands: Commands,
    mut previews: Query<(Entity, &mut CharacterPreviewModel)>,
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
            warn!("Character Info player model has no default glTF scene");
            preview.scene_attached = true;
            continue;
        };

        commands.entity(entity).with_children(|parent| {
            parent
                .spawn((
                    WorldAssetRoot(scene),
                    Transform::default(),
                    CharacterPreviewAppearance,
                ))
                .observe(configure_character_preview_scene);
        });
        preview.scene_attached = true;
    }
}

#[derive(SystemParam)]
struct CharacterPreviewSceneAssets<'w, 's> {
    appearances: Query<'w, 's, (), With<CharacterPreviewAppearance>>,
    mesh_entities: Query<'w, 's, &'static Mesh3d>,
    meshes: Res<'w, Assets<Mesh>>,
    mesh_materials: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

fn configure_character_preview_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    mut assets: CharacterPreviewSceneAssets,
    mut interaction: ResMut<CharacterPreviewInteraction>,
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
                RenderLayers::layer(CHARACTER_PREVIEW_RENDER_LAYER),
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

    interaction.render_frames = interaction.render_frames.max(2);
}

fn spawn_character_info(
    mut commands: Commands,
    preview_image: Res<CharacterPreviewImage>,
) {
    let Some(image) = preview_image.0.clone() else {
        warn!("Character Info opened without a preview render target");
        return;
    };

    commands
        .spawn((
            CharacterInfoRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
            DespawnOnExit(CharacterInfoState::Open),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                surface::hud_container(Node {
                    width: px(460),
                    padding: UiRect::all(px(18)),
                    border: UiRect::all(px(1)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    column_gap: px(18),
                    ..default()
                }),
                Pickable::IGNORE,
            ))
            .with_children(|card| {
                spawn_character_preview_viewport(card, image);
                card.spawn((
                    typography::hud_heading(PLAYER_DISPLAY_NAME),
                    Pickable::IGNORE,
                ));
            });
        });
}

fn spawn_character_preview_viewport(
    parent: &mut ChildSpawnerCommands,
    image: Handle<Image>,
) {
    parent
        .spawn((
            Button,
            CharacterPreviewViewport,
            Node {
                width: px(CHARACTER_PREVIEW_CARD_WIDTH),
                height: px(CHARACTER_PREVIEW_CARD_HEIGHT),
                flex_shrink: 0.0,
                padding: UiRect::all(px(2)),
                border: UiRect::all(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::SURFACE_INSET),
            BorderColor::all(theme::BORDER),
        ))
        .with_children(|frame| {
            frame.spawn((
                ImageNode::new(image),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

fn stop_character_preview_drag(
    mut interaction: ResMut<CharacterPreviewInteraction>,
) {
    interaction.dragging = false;
}

fn rotate_character_preview(
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    viewports: Query<&Interaction, With<CharacterPreviewViewport>>,
    mut interaction: ResMut<CharacterPreviewInteraction>,
    mut models: Query<&mut Transform, With<CharacterPreviewModel>>,
) {
    if mouse.just_released(MouseButton::Left) {
        interaction.dragging = false;
    }

    if mouse.pressed(MouseButton::Left)
        && viewports
            .iter()
            .any(|state| *state == Interaction::Pressed)
    {
        interaction.dragging = true;
    }

    let delta = mouse_motion.read().map(|motion| motion.delta).sum::<Vec2>();
    if !interaction.dragging || !mouse.pressed(MouseButton::Left) || delta.x == 0.0 {
        return;
    }

    interaction.yaw += delta.x * CHARACTER_PREVIEW_DRAG_SENSITIVITY;
    interaction.render_frames = interaction.render_frames.max(2);
    let rotation = Quat::from_rotation_y(interaction.yaw);
    for mut transform in &mut models {
        transform.rotation = rotation;
    }
}

fn sync_character_preview_camera(
    mut interaction: ResMut<CharacterPreviewInteraction>,
    mut cameras: Query<&mut Camera, With<CharacterPreviewCamera>>,
    mut models: Query<&mut Visibility, With<CharacterPreviewModel>>,
) {
    let should_render = interaction.render_frames > 0;
    let clear_color = if should_render {
        ClearColorConfig::Custom(Color::NONE)
    } else {
        ClearColorConfig::None
    };
    let visibility = if should_render {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for mut camera in &mut cameras {
        camera.clear_color = clear_color;
    }
    for mut model_visibility in &mut models {
        if *model_visibility != visibility {
            *model_visibility = visibility;
        }
    }

    if should_render {
        interaction.render_frames = interaction.render_frames.saturating_sub(1);
    }
}

