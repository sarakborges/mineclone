use std::{collections::HashMap, time::Duration};

use bevy::{
    animation::RepeatAnimation,
    camera::visibility::RenderLayers,
    ecs::system::SystemParam,
    light::NotShadowCaster,
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    app::{
        game_state::GameState,
        pause_state::PauseState,
        settings_state::SettingsState,
    },
    content::{
        player::PlayerDefinition,
    },
    entity::EntityHealth,
    gameplay::modal::GameplayModalState,
    player::{
        PLAYER_EYE_HEIGHT, PlayerEntity, apply_player_skin_material,
        camera::{CameraPerspective, GameplayCamera},
        held_sprite::{HeldSpriteAssets, HeldSpriteContent, spawn_held_sprite},
        hotbar::PlayerHotbar,
        movement::{gravity::GravityState, walking::WalkingState},
        viewmodel::ViewModelAnimation,
    },
    rendering::{
        block_model::{
            BlockModel, BlockModelMaterials, BlockModelMeshes, apply_block_display_shading,
            block_face_material_data, maximum_block_model_layers, set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_visual_content::BlockVisualContent,
    },
    targeting::block::BlockTargetingSet,
    voxel::block_face::BlockFace,
};

const HURT_HOLD_SECONDS: f32 = 0.38;
const MOVING_START_SPEED_SQUARED: f32 = 0.01;
const MOVING_STOP_SPEED_SQUARED: f32 = 0.0025;
const THIRD_PERSON_HELD_BLOCK_SCALE: f32 = 0.22;
pub(crate) const PLAYER_MODEL_HUD_RENDER_LAYER: usize = 3;
pub(crate) const PLAYER_MODEL_CHARACTER_INFO_RENDER_LAYER: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerModelRenderScope {
    Gameplay,
    HudPreview,
    CharacterInfoPreview,
}

fn player_model_render_layers(
    scope: PlayerModelRenderScope,
    third_person: bool,
) -> RenderLayers {
    match scope {
        PlayerModelRenderScope::Gameplay if third_person => RenderLayers::layer(0),
        PlayerModelRenderScope::Gameplay => RenderLayers::from_layers(&[]),
        PlayerModelRenderScope::HudPreview => {
            RenderLayers::layer(PLAYER_MODEL_HUD_RENDER_LAYER)
        }
        PlayerModelRenderScope::CharacterInfoPreview => {
            RenderLayers::layer(PLAYER_MODEL_CHARACTER_INFO_RENDER_LAYER)
        }
    }
}

#[derive(Component)]
pub(crate) struct PlayerModelRoot;

#[derive(Component, Clone, Copy)]
struct PlayerModelSceneScope(PlayerModelRenderScope);

#[derive(Component)]
struct PlayerModelRenderable(PlayerModelRenderScope);

#[derive(Component)]
struct PlayerModel(Handle<Gltf>);

#[derive(Component)]
struct PlayerModelVisualAttached;

#[derive(Component)]
struct PlayerModelHead;

#[derive(Component)]
struct PlayerModelHand(PlayerModelRenderScope);

#[derive(Component)]
struct ThirdPersonHeldBlockRoot;

#[derive(Component)]
struct ThirdPersonHeldBlockFace {
    face: BlockFace,
    layer_index: usize,
}

#[derive(Default)]
struct ThirdPersonHeldBlockVisualCache {
    tint_cell: Option<IVec2>,
    tint: Option<Color>,
}

#[derive(Component, Clone)]
struct PlayerModelAppearance {
    graph: Option<Handle<AnimationGraph>>,
    nodes: HashMap<String, AnimationNodeIndex>,
}

#[derive(Component)]
struct PlayerModelAnimationLink {
    nodes: HashMap<String, AnimationNodeIndex>,
    current_state: String,
    current_revision: u64,
}

#[derive(Component)]
struct PlayerModelAnimationState {
    name: String,
    revision: u64,
    action_revision: u64,
    playback_speed: f32,
    hold_seconds: f32,
    last_health: Option<f32>,
}

type PlayerModelRootQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut PlayerModelAnimationState,
    ),
    (With<PlayerModelRoot>, Without<GameplayCamera>),
>;

type PlayerModelHeadQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (
        With<PlayerModelHead>,
        Without<PlayerModelRoot>,
        Without<GameplayCamera>,
    ),
>;

type ThirdPersonHeldBlockRootQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut BlockModel, &'static mut Visibility),
    (With<ThirdPersonHeldBlockRoot>, Without<ThirdPersonHeldBlockFace>),
>;

#[derive(SystemParam)]
struct PlayerSceneVisuals<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    mesh_entities: Query<'w, 's, &'static Mesh3d>,
    meshes: Res<'w, Assets<Mesh>>,
    mesh_materials: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

#[derive(SystemParam)]
struct PlayerSceneQueries<'w, 's> {
    descendants: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    appearances: Query<'w, 's, &'static PlayerModelAppearance>,
    scene_scopes: Query<'w, 's, &'static PlayerModelSceneScope>,
    players: Query<'w, 's, (Entity, &'static mut AnimationPlayer)>,
}

#[derive(SystemParam)]
struct ThirdPersonHeldBlockAssets<'w> {
    block_meshes: Res<'w, BlockModelMeshes>,
    block_materials: ResMut<'w, BlockModelMaterials>,
    materials: ResMut<'w, Assets<BlockModelMaterial>>,
}

#[derive(SystemParam)]
struct ThirdPersonHeldBlockView<'w, 's> {
    materials: ResMut<'w, Assets<BlockModelMaterial>>,
    roots: ThirdPersonHeldBlockRootQuery<'w, 's>,
    faces: Query<
        'w,
        's,
        (
            &'static ThirdPersonHeldBlockFace,
            &'static MeshMaterial3d<BlockModelMaterial>,
            &'static mut Visibility,
        ),
        Without<ThirdPersonHeldBlockRoot>,
    >,
}

impl Default for PlayerModelAnimationState {
    fn default() -> Self {
        Self {
            name: "idle".to_owned(),
            revision: 0,
            action_revision: 0,
            playback_speed: 1.0,
            hold_seconds: 0.0,
            last_health: None,
        }
    }
}

impl PlayerModelAnimationState {
    fn set(&mut self, name: &str) {
        if self.name == name && self.playback_speed == 1.0 {
            return;
        }
        self.name = name.to_owned();
        self.playback_speed = 1.0;
        self.revision = self.revision.wrapping_add(1);
    }

    fn trigger(&mut self, name: &str, hold_seconds: f32) {
        self.trigger_at_speed(name, hold_seconds, 1.0);
    }

    fn trigger_at_speed(&mut self, name: &str, hold_seconds: f32, playback_speed: f32) {
        self.name = name.to_owned();
        self.playback_speed = playback_speed;
        self.revision = self.revision.wrapping_add(1);
        self.hold_seconds = hold_seconds;
    }
}

pub(crate) struct PlayerModelPlugin;

impl Plugin for PlayerModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_player_model)
            .add_systems(
                Update,
                (
                    attach_loaded_player_model,
                    sync_player_preview_model_visibility,
                    spawn_third_person_held_block,
                    spawn_third_person_held_sprite,
                    sync_player_model,
                    sync_player_model_animations,
                    sync_third_person_held_block,
                )
                    .chain()
                    .after(BlockTargetingSet::Interaction)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_player_model(
    mut commands: Commands,
    definition: Res<PlayerDefinition>,
    asset_server: Res<AssetServer>,
) {
    let Some(model_path) = definition.model.as_ref() else {
        warn!("player definition has no model configured");
        return;
    };

    commands.spawn((
        Name::new("Player Model"),
        PlayerModelRoot,
        PlayerModel(asset_server.load::<Gltf>(model_path.clone())),
        PlayerModelAnimationState::default(),
        Transform::default(),
        Visibility::Visible,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn attach_loaded_player_model(
    mut commands: Commands,
    roots: Query<(Entity, &PlayerModel), Without<PlayerModelVisualAttached>>,
    definition: Res<PlayerDefinition>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (root, model) in &roots {
        let Some(gltf) = gltfs.get(&model.0) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone() else {
            warn!("player model has no default scene");
            commands.entity(root).insert(PlayerModelVisualAttached);
            continue;
        };

        let mut mapped: Vec<_> = definition.animations.iter().collect();
        mapped.sort_by(|a, b| a.0.cmp(b.0));
        let mut states = Vec::new();
        let mut clips = Vec::new();
        for (state, clip_name) in mapped {
            if let Some(clip) = gltf.named_animations.get(clip_name.as_str()) {
                states.push(state.clone());
                clips.push(clip.clone());
            } else {
                warn!("player model missing animation {clip_name} ({state})");
            }
        }

        let (graph, nodes) = if clips.is_empty() {
            (None, HashMap::new())
        } else {
            let (graph, indexes) = AnimationGraph::from_clips(clips);
            (
                Some(graphs.add(graph)),
                states.into_iter().zip(indexes).collect(),
            )
        };

        commands.entity(root).insert(PlayerModelVisualAttached);
        commands.entity(root).with_children(|parent| {
            for (scope, visibility) in [
                (PlayerModelRenderScope::Gameplay, Visibility::Inherited),
                (PlayerModelRenderScope::HudPreview, Visibility::Inherited),
                (
                    PlayerModelRenderScope::CharacterInfoPreview,
                    Visibility::Hidden,
                ),
            ] {
                parent
                    .spawn((
                        WorldAssetRoot(scene.clone()),
                        Transform::default(),
                        visibility,
                        PlayerModelSceneScope(scope),
                        PlayerModelAppearance {
                            graph: graph.clone(),
                            nodes: nodes.clone(),
                        },
                    ))
                    .observe(configure_loaded_player_scene);
            }
        });
    }
}

fn configure_loaded_player_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    mut scene: PlayerSceneQueries,
    mut visuals: PlayerSceneVisuals,
) {
    let Ok(appearance) = scene.appearances.get(ready.entity) else {
        return;
    };
    let Ok(scene_scope) = scene.scene_scopes.get(ready.entity) else {
        return;
    };
    let scope = scene_scope.0;

    for descendant in scene.descendants.iter_descendants(ready.entity) {
        if configure_player_mesh(&mut commands, &visuals, descendant, scope) {
            continue;
        }
        configure_player_material(&mut commands, &mut visuals, descendant);
        tag_player_model_part(&mut commands, &scene.names, descendant, scope);
        configure_player_animation(
            &mut commands,
            &mut scene.players,
            descendant,
            appearance,
        );
    }
}

fn configure_player_mesh(
    commands: &mut Commands,
    visuals: &PlayerSceneVisuals,
    descendant: Entity,
    scope: PlayerModelRenderScope,
) -> bool {
    let Ok(mesh_handle) = visuals.mesh_entities.get(descendant) else {
        return false;
    };

    if visuals
        .meshes
        .get(mesh_handle.id())
        .is_some_and(|mesh| mesh.get_vertex_buffer_size() == 0)
    {
        commands
            .entity(descendant)
            .remove::<Mesh3d>()
            .remove::<MeshMaterial3d<StandardMaterial>>();
        return true;
    }

    commands.entity(descendant).insert((
        PlayerModelRenderable(scope),
        player_model_render_layers(scope, false),
    ));
    false
}

fn configure_player_material(
    commands: &mut Commands,
    visuals: &mut PlayerSceneVisuals,
    descendant: Entity,
) {
    let Ok(material_handle) = visuals.mesh_materials.get(descendant) else {
        return;
    };

    if let Some(mut material) = visuals.materials.get_mut(material_handle.id()) {
        apply_player_skin_material(&mut material, &visuals.asset_server);
        material.reflectance = 0.0;
        material.emissive = LinearRgba::BLACK;
        material.emissive_texture = None;
    }
    commands.entity(descendant).insert(NotShadowCaster);
}

fn tag_player_model_part(
    commands: &mut Commands,
    names: &Query<&Name>,
    descendant: Entity,
    scope: PlayerModelRenderScope,
) {
    let Ok(name) = names.get(descendant) else {
        return;
    };

    match name.as_str() {
        "HeadPivot" => {
            commands.entity(descendant).insert(PlayerModelHead);
        }
        "RightArmPivot" => {
            commands.entity(descendant).insert(PlayerModelHand(scope));
        }
        _ => {}
    }
}

fn configure_player_animation(
    commands: &mut Commands,
    players: &mut Query<(Entity, &mut AnimationPlayer)>,
    descendant: Entity,
    appearance: &PlayerModelAppearance,
) {
    let Ok((player_entity, mut player)) = players.get_mut(descendant) else {
        return;
    };
    let Some(graph) = &appearance.graph else {
        return;
    };

    let mut transitions = AnimationTransitions::new();
    if let Some(index) = appearance.nodes.get("idle").copied() {
        transitions.play(&mut player, index, Duration::ZERO).repeat();
    }
    commands.entity(player_entity).insert((
        AnimationGraphHandle(graph.clone()),
        transitions,
        PlayerModelAnimationLink {
            nodes: appearance.nodes.clone(),
            current_state: "idle".to_owned(),
            current_revision: 0,
        },
    ));
}

fn sync_player_preview_model_visibility(
    pause: Res<State<PauseState>>,
    settings: Res<State<SettingsState>>,
    modal: Res<State<GameplayModalState>>,
    mut scene_roots: Query<(&PlayerModelSceneScope, &mut Visibility)>,
) {
    for (scene_scope, mut visibility) in &mut scene_roots {
        let next = match scene_scope.0 {
            PlayerModelRenderScope::Gameplay => Visibility::Inherited,
            PlayerModelRenderScope::HudPreview => {
                if *pause.get() == PauseState::Running
                    && *settings.get() == SettingsState::Closed
                {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                }
            }
            PlayerModelRenderScope::CharacterInfoPreview => {
                if *modal.get() == GameplayModalState::CharacterInfo {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                }
            }
        };
        if *visibility != next {
            *visibility = next;
        }
    }
}

fn sync_player_model(
    time: Res<Time>,
    perspective: Res<CameraPerspective>,
    viewmodel_animation: Res<ViewModelAnimation>,
    player: Single<
        (
            &Transform,
            &GameplayCamera,
            &WalkingState,
            &GravityState,
            &EntityHealth,
        ),
        With<PlayerEntity>,
    >,
    mut models: PlayerModelRootQuery,
    mut heads: PlayerModelHeadQuery,
    mut renderables: Query<(&PlayerModelRenderable, &mut RenderLayers)>,
) {
    let (player_transform, camera, walking, gravity, health) = *player;

    for (mut model_transform, mut state) in &mut models {
        model_transform.translation =
            player_transform.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
        model_transform.rotation = Quat::from_rotation_y(camera.yaw + std::f32::consts::PI);

        state.hold_seconds = (state.hold_seconds - time.delta_secs()).max(0.0);

        let current_health = health.current();
        let hurt = state
            .last_health
            .is_some_and(|previous| current_health < previous);
        state.last_health = Some(current_health);

        if health.is_dead() {
            if state.name != "death" {
                state.trigger("death", 0.0);
            }
            continue;
        }
        if hurt {
            state.trigger("hurt", HURT_HOLD_SECONDS);
            continue;
        }
        if let Some(action) = viewmodel_animation.action_name() {
            let action_revision = viewmodel_animation.revision();
            if state.name != action || state.action_revision != action_revision {
                state.action_revision = action_revision;
                state.trigger_at_speed(action, 0.0, viewmodel_animation.playback_speed());
            }
            continue;
        }

        if state.hold_seconds > 0.0 {
            continue;
        }

        let horizontal_speed_squared = walking.horizontal_speed_squared();
        let locomotion = if !gravity.grounded() {
            if gravity.vertical_velocity() > 0.05 {
                "jump"
            } else {
                "fall"
            }
        } else if matches!(state.name.as_str(), "walk" | "run") {
            if horizontal_speed_squared > MOVING_STOP_SPEED_SQUARED {
                if walking.is_running() { "run" } else { "walk" }
            } else {
                "idle"
            }
        } else if horizontal_speed_squared > MOVING_START_SPEED_SQUARED {
            if walking.is_running() { "run" } else { "walk" }
        } else {
            "idle"
        };
        state.set(locomotion);
    }

    for mut head in &mut heads {
        head.rotation = Quat::from_rotation_x(-camera.pitch);
    }

    for (renderable, mut render_layers) in &mut renderables {
        let layers =
            player_model_render_layers(renderable.0, perspective.is_third_person());
        if *render_layers != layers {
            *render_layers = layers;
        }
    }
}

fn sync_player_model_animations(
    states: Query<&PlayerModelAnimationState, With<PlayerModelRoot>>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut PlayerModelAnimationLink,
    )>,
) {
    let Some(state) = states.iter().next() else {
        return;
    };

    for (mut player, mut transitions, mut link) in &mut players {
        if link.current_state == state.name && link.current_revision == state.revision {
            continue;
        }
        let Some(index) = link.nodes.get(&state.name).copied() else {
            continue;
        };

        // Let AnimationTransitions own the handoff between clips. Resetting
        // animated transforms manually here produced a visible one-frame rest
        // pose between states.
        let transition = if matches!(state.name.as_str(), "break" | "hit" | "place") {
            Duration::ZERO
        } else {
            Duration::from_millis(80)
        };
        let animation = transitions.play(&mut player, index, transition);
        animation.set_speed(state.playback_speed);
        if matches!(
            state.name.as_str(),
            "idle" | "walk" | "run" | "fall"
        ) {
            animation.repeat();
        } else {
            animation.set_repeat(RepeatAnimation::Count(1));
        }

        link.current_state.clone_from(&state.name);
        link.current_revision = state.revision;
    }
}

fn spawn_third_person_held_block(
    mut commands: Commands,
    hands: Query<(Entity, &PlayerModelHand), Added<PlayerModelHand>>,
    definitions: BlockVisualContent,
    hotbar: Res<PlayerHotbar>,
    player: Single<&Transform, With<PlayerEntity>>,
    assets: ThirdPersonHeldBlockAssets,
) {
    let ThirdPersonHeldBlockAssets {
        block_meshes,
        mut block_materials,
        mut materials,
    } = assets;
    let selected_block_id = hotbar
        .item_at(hotbar.selected_slot())
        .filter(|block_id| definitions.blocks.get(block_id).is_some());
    let block_model = selected_block_id
        .map(|block_id| BlockModel::world(block_id, 1.0))
        .unwrap_or_else(|| BlockModel::empty_world(1.0));
    let tint_position = Vec2::new(player.translation.x, player.translation.z);
    let root_visibility = third_person_item_visibility(selected_block_id);

    for (hand_entity, hand_scope) in &hands {
        commands.entity(hand_entity).with_children(|hand| {
            hand.spawn((
                ThirdPersonHeldBlockRoot,
                block_model,
                third_person_held_block_transform(),
                root_visibility,
            ))
            .with_children(|held| {
                let selected_block = selected_block_id.and_then(|block_id| {
                    definitions
                        .blocks
                        .get(block_id)
                        .map(|block| (block_id, block))
                });
                let tint = selected_block
                    .and_then(|(block_id, _)| definitions.tint_at(block_id, tint_position));

                for &face in block_model.faces() {
                    let layer_count = maximum_block_model_layers(&definitions.blocks, face);
                    let layer_materials =
                        block_materials.held_for_face(face, layer_count, &mut materials);

                    for (layer_index, material) in layer_materials.into_iter().enumerate() {
                        let mut visibility = Visibility::Hidden;
                        if let Some((_, block)) = selected_block
                            && let Some(face_material) = block_face_material_data(
                                face,
                                layer_index,
                                block,
                                &definitions.asset_server,
                                block_model.opacity(),
                            )
                            && let Some(mut material_asset) = materials.get_mut(&material)
                        {
                            *material_asset = face_material;
                            apply_block_display_shading(
                                &mut material_asset,
                                face,
                                block_model.opacity(),
                            );
                            set_block_model_tint(
                                &mut material_asset,
                                tint.unwrap_or(Color::WHITE),
                            );
                            visibility = Visibility::Inherited;
                        }

                        held.spawn((
                            ThirdPersonHeldBlockFace { face, layer_index },
                            Mesh3d(block_meshes.world_face(face)),
                            MeshMaterial3d(material),
                            visibility,
                            NotShadowCaster,
                            PlayerModelRenderable(hand_scope.0),
                            player_model_render_layers(hand_scope.0, false),
                        ));
                    }
                }
            });
        });
    }
}

fn spawn_third_person_held_sprite(
    mut commands: Commands,
    hands: Query<(Entity, &PlayerModelHand), Added<PlayerModelHand>>,
    content: HeldSpriteContent,
    mut assets: HeldSpriteAssets,
) {
    for (hand_entity, hand_scope) in &hands {
        let root_transform = Transform::from_translation(Vec3::new(0.0, -0.72, -0.06))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.35, 0.65, 0.18));
        let render_layers = player_model_render_layers(hand_scope.0, false);
        let mut renderables = [Entity::PLACEHOLDER; 2];

        commands.entity(hand_entity).with_children(|hand| {
            renderables = spawn_held_sprite(
                hand,
                root_transform,
                render_layers,
                &content,
                &mut assets,
            );
        });

        for renderable in renderables {
            commands
                .entity(renderable)
                .insert(PlayerModelRenderable(hand_scope.0));
        }
    }
}

fn sync_third_person_held_block(
    definitions: BlockVisualContent,
    hotbar: Res<PlayerHotbar>,
    player: Single<&Transform, With<PlayerEntity>>,
    mut cache: Local<ThirdPersonHeldBlockVisualCache>,
    view: ThirdPersonHeldBlockView,
) {
    let ThirdPersonHeldBlockView {
        mut materials,
        mut roots,
        mut faces,
    } = view;
    let tint_cell = IVec2::new(
        player.translation.x.floor() as i32,
        player.translation.z.floor() as i32,
    );
    let tint_cell_changed = cache.tint_cell != Some(tint_cell);
    let definitions_changed = definitions.block_definitions_changed();
    let selection_changed = hotbar.is_changed();
    let visual_inputs_changed = definitions.inputs_changed();
    if !tint_cell_changed && !selection_changed && !visual_inputs_changed {
        return;
    }
    cache.tint_cell = Some(tint_cell);

    let selected_block_id = hotbar
        .item_at(hotbar.selected_slot())
        .filter(|block_id| definitions.blocks.get(block_id).is_some());
    let visibility = third_person_item_visibility(selected_block_id);
    let tint_position = tint_cell.as_vec2() + Vec2::splat(0.5);

    for (mut held, mut held_visibility) in &mut roots {
        let block_changed = held.block_id() != selected_block_id;
        if block_changed {
            held.set_block_id(selected_block_id);
        }
        if *held_visibility != visibility {
            *held_visibility = visibility;
        }

        let Some(block_id) = selected_block_id else {
            if block_changed {
                for (_, _, mut layer_visibility) in &mut faces {
                    if *layer_visibility != Visibility::Hidden {
                        *layer_visibility = Visibility::Hidden;
                    }
                }
            }
            cache.tint = None;
            continue;
        };
        let Some(block) = definitions.blocks.get(block_id) else {
            continue;
        };

        let materials_changed = block_changed || definitions_changed;
        if materials_changed {
            for (face, material_handle, mut layer_visibility) in &mut faces {
                let Some(mut material) = materials.get_mut(&material_handle.0) else {
                    continue;
                };
                let Some(face_material) = block_face_material_data(
                    face.face,
                    face.layer_index,
                    block,
                    &definitions.asset_server,
                    held.opacity(),
                ) else {
                    if *layer_visibility != Visibility::Hidden {
                        *layer_visibility = Visibility::Hidden;
                    }
                    continue;
                };
                *material = face_material;
                apply_block_display_shading(&mut material, face.face, held.opacity());
                if *layer_visibility != Visibility::Inherited {
                    *layer_visibility = Visibility::Inherited;
                }
            }
        }

        if materials_changed || tint_cell_changed || visual_inputs_changed {
            let tint = definitions
                .tint_at(block_id, tint_position)
                .unwrap_or(Color::WHITE);
            if materials_changed || cache.tint != Some(tint) {
                for (_, material_handle, _) in &mut faces {
                    if let Some(mut material) = materials.get_mut(&material_handle.0) {
                        set_block_model_tint(&mut material, tint);
                    }
                }
            }
            cache.tint = Some(tint);
        }
    }
}

fn third_person_item_visibility(block_id: Option<&'static str>) -> Visibility {
    if block_id.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

fn third_person_held_block_transform() -> Transform {
    Transform::from_translation(Vec3::new(0.0, -0.72, -0.06))
        .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.35, 0.65, 0.18))
        .with_scale(Vec3::splat(THIRD_PERSON_HELD_BLOCK_SCALE))
}
