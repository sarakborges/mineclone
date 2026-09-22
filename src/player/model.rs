use std::{collections::HashMap, time::Duration};

use bevy::{
    animation::RepeatAnimation,
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    app::game_state::GameState,
    content::player::PlayerDefinition,
    entity::EntityHealth,
    player::{
        PLAYER_EYE_HEIGHT, PlayerEntity,
        camera::{CameraPerspective, GameplayCamera},
        movement::{gravity::GravityState, walking::WalkingState},
        viewmodel::ViewModelAnimation,
    },
};

const HURT_HOLD_SECONDS: f32 = 0.38;
const MOVING_SPEED_SQUARED: f32 = 0.01;

#[derive(Component)]
struct PlayerModelRoot;

#[derive(Component)]
struct PlayerModel(Handle<Gltf>);

#[derive(Component)]
struct PlayerModelVisualAttached;

#[derive(Component)]
struct PlayerModelHead;

#[derive(Component)]
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
    hold_seconds: f32,
    last_health: Option<f32>,
}

impl Default for PlayerModelAnimationState {
    fn default() -> Self {
        Self {
            name: "idle".to_owned(),
            revision: 0,
            action_revision: 0,
            hold_seconds: 0.0,
            last_health: None,
        }
    }
}

impl PlayerModelAnimationState {
    fn set(&mut self, name: &str) {
        if self.name == name {
            return;
        }
        self.name = name.to_owned();
        self.revision = self.revision.wrapping_add(1);
    }

    fn trigger(&mut self, name: &str, hold_seconds: f32) {
        self.name = name.to_owned();
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
                    sync_player_model,
                    sync_player_model_animations,
                )
                    .chain()
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
        Visibility::Hidden,
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
            parent
                .spawn((
                    WorldAssetRoot(scene),
                    Transform::default(),
                    PlayerModelAppearance { graph, nodes },
                ))
                .observe(configure_loaded_player_scene);
        });
    }
}

fn configure_loaded_player_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    names: Query<&Name>,
    appearances: Query<&PlayerModelAppearance>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
) {
    let Ok(appearance) = appearances.get(ready.entity) else {
        return;
    };

    for descendant in descendants.iter_descendants(ready.entity) {
        if names
            .get(descendant)
            .is_ok_and(|name| name.as_str() == "HeadPivot")
        {
            commands.entity(descendant).insert(PlayerModelHead);
        }

        let Ok((player_entity, mut player)) = players.get_mut(descendant) else {
            continue;
        };
        let Some(graph) = &appearance.graph else {
            continue;
        };

        let mut transitions = AnimationTransitions::new();
        if let Some(index) = appearance.nodes.get("idle").copied() {
            transitions
                .play(&mut player, index, Duration::ZERO)
                .repeat();
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
    mut models: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut PlayerModelAnimationState,
        ),
        (With<PlayerModelRoot>, Without<GameplayCamera>),
    >,
    mut heads: Query<
        &mut Transform,
        (
            With<PlayerModelHead>,
            Without<PlayerModelRoot>,
            Without<GameplayCamera>,
        ),
    >,
) {
    let (player_transform, camera, walking, gravity, health) = *player;

    for (mut model_transform, mut visibility, mut state) in &mut models {
        let next_visibility = if perspective.is_third_person() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }

        model_transform.translation =
            player_transform.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
        model_transform.rotation = Quat::from_rotation_y(camera.yaw);

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
        if state.hold_seconds > 0.0 {
            continue;
        }

        if let Some(action) = viewmodel_animation.action_name() {
            let action_revision = viewmodel_animation.revision();
            if state.name != action || state.action_revision != action_revision {
                state.action_revision = action_revision;
                state.trigger(action, 0.0);
            }
            continue;
        }

        let locomotion = if !gravity.grounded() {
            if gravity.vertical_velocity() > 0.05 {
                "jump"
            } else {
                "fall"
            }
        } else if walking.horizontal_speed_squared() > MOVING_SPEED_SQUARED {
            "walk"
        } else {
            "idle"
        };
        state.set(locomotion);
    }

    for mut head in &mut heads {
        head.rotation = Quat::from_rotation_x(camera.pitch);
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

        let animation = transitions.play(&mut player, index, Duration::from_millis(80));
        if matches!(
            state.name.as_str(),
            "idle" | "walk" | "run" | "fall" | "break"
        ) {
            animation.repeat();
        } else {
            animation.set_repeat(RepeatAnimation::Count(1));
        }

        link.current_state.clone_from(&state.name);
        link.current_revision = state.revision;
    }
}
