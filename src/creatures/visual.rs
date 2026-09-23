use std::{collections::{HashMap, HashSet}, time::Duration};

use bevy::{
    animation::RepeatAnimation, asset::AssetId, ecs::system::SystemParam, gltf::GltfMaterialName,
    prelude::*, world_serialization::WorldInstanceReady,
};

use crate::content::{color::Hsi, creature::CreatureRegistry};

use super::{
    CreatureInstance,
    material::apply_creature_material_overrides,
    motion::CreatureMotion,
};

/// The glTF asset belongs to the visual loader; the root owns physics/position.
#[derive(Component)]
pub(super) struct CreatureModel(pub Handle<Gltf>);

#[derive(Component)]
pub(super) struct VisualAttached;

#[derive(Component)]
pub(super) struct CreatureAppearance {
    owner: Entity,
    material_tints: HashMap<String, Hsi>,
    material_textures: HashMap<String, Handle<Image>>,
    unlit_materials: HashSet<String>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: HashMap<String, AnimationNodeIndex>,
}

#[derive(Component)]
pub(super) struct CreatureAnimationLink {
    owner: Entity,
    nodes: HashMap<String, AnimationNodeIndex>,
    current_state: String,
    current_revision: u64,
}

/// A gameplay system may change this on the physics root. Clip names are
/// resolved solely from the corresponding creature JSON, not from Rust.
#[derive(Component)]
pub(crate) struct CreatureAnimationState {
    pub(crate) name: String,
    pub(crate) revision: u64,
}

impl CreatureAnimationState {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            revision: 0,
        }
    }

    pub(crate) fn trigger(&mut self, name: &str) {
        self.name = name.to_owned();
        self.revision = self.revision.wrapping_add(1);
    }
}

/// The same glTF material may need a different texture and/or tint per species.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CreatureMaterialCacheKey {
    material: AssetId<StandardMaterial>,
    tint_bits: Option<[u32; 3]>,
    texture: Option<AssetId<Image>>,
    unlit: bool,
}

#[derive(Resource, Default)]
pub(super) struct TintedCreatureMaterials(
    HashMap<CreatureMaterialCacheKey, Handle<StandardMaterial>>,
);

/// Only roots still waiting for a glTF are visited; animated meshes and
/// material assets are never rebuilt every frame.
pub(super) fn attach_loaded_models(
    mut commands: Commands,
    roots: Query<(Entity, &CreatureInstance, &CreatureModel), Without<VisualAttached>>,
    definitions: Res<CreatureRegistry>,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (root, instance, model) in &roots {
        let Some(gltf) = gltfs.get(&model.0) else {
            continue;
        };
        let Some(definition) = definitions.get(&instance.definition_id) else {
            warn!("creature definition {} disappeared", instance.definition_id);
            commands.entity(root).insert(VisualAttached);
            continue;
        };
        let Some(scene) = gltf.default_scene.clone() else {
            warn!(
                "creature {} model {} has no default scene",
                definition.id, definition.model
            );
            commands.entity(root).insert(VisualAttached);
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
                warn!(
                    "creature {} missing animation {clip_name} ({state}) in {}",
                    definition.id, definition.model
                );
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
        let tints = definition.material_tints.clone();
        let unlit_materials = definition.unlit_materials.clone();
        let textures = definition
            .textures
            .iter()
            .map(|(material, path)| (material.clone(), asset_server.load::<Image>(path.clone())))
            .collect();
        commands
            .entity(root)
            .insert((VisualAttached, CreatureAnimationState::new("idle")));
        commands.entity(root).with_children(|parent| {
            parent
                .spawn((
                    WorldAssetRoot(scene),
                    Transform::default(),
                    CreatureAppearance {
                        owner: root,
                        material_tints: tints,
                        material_textures: textures,
                        unlit_materials,
                        graph,
                        nodes,
                    },
                ))
                .observe(configure_loaded_scene);
        });
    }
}

#[derive(SystemParam)]
struct CreatureTintAssets<'w> {
    materials: ResMut<'w, Assets<StandardMaterial>>,
    cache: ResMut<'w, TintedCreatureMaterials>,
}

#[derive(SystemParam)]
struct CreatureSceneQueries<'w, 's> {
    descendants: Query<'w, 's, &'static Children>,
    appearances: Query<'w, 's, &'static CreatureAppearance>,
    mesh_materials: Query<
        'w,
        's,
        (
            &'static MeshMaterial3d<StandardMaterial>,
            &'static GltfMaterialName,
        ),
    >,
    players: Query<'w, 's, (Entity, &'static mut AnimationPlayer)>,
}

fn configure_loaded_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    mut scene: CreatureSceneQueries,
    mut tint_assets: CreatureTintAssets,
) {
    let Ok(appearance) = scene.appearances.get(ready.entity) else {
        return;
    };

    for descendant in scene.descendants.iter_descendants(ready.entity) {
        configure_creature_material(
            &mut commands,
            &scene.mesh_materials,
            &mut tint_assets,
            descendant,
            appearance,
        );
        configure_creature_animation(
            &mut commands,
            &mut scene.players,
            descendant,
            appearance,
        );
    }
}

fn configure_creature_material(
    commands: &mut Commands,
    mesh_materials: &Query<(&MeshMaterial3d<StandardMaterial>, &GltfMaterialName)>,
    tint_assets: &mut CreatureTintAssets,
    descendant: Entity,
    appearance: &CreatureAppearance,
) {
    let Ok((original, material_name)) = mesh_materials.get(descendant) else {
        return;
    };

    let name = material_name.0.as_str();
    let tint = appearance.material_tints.get(name);
    let texture = appearance.material_textures.get(name);
    let unlit = appearance.unlit_materials.contains(name);
    if tint.is_none() && texture.is_none() && !unlit {
        return;
    }

    let rgb = tint.map(|color| color.to_srgb());
    let cache_key = CreatureMaterialCacheKey {
        material: original.id(),
        tint_bits: rgb.map(|color| color.map(f32::to_bits)),
        texture: texture.map(|image| image.id()),
        unlit,
    };
    let replacement = if let Some(existing) = tint_assets.cache.0.get(&cache_key) {
        Some(existing.clone())
    } else {
        tint_assets
            .materials
            .get(original.id())
            .cloned()
            .map(|mut material| {
                apply_creature_material_overrides(&mut material, tint, texture, unlit);
                let handle = tint_assets.materials.add(material);
                tint_assets.cache.0.insert(cache_key, handle.clone());
                handle
            })
    };
    if let Some(material) = replacement {
        commands.entity(descendant).insert(MeshMaterial3d(material));
    }
}

fn configure_creature_animation(
    commands: &mut Commands,
    players: &mut Query<(Entity, &mut AnimationPlayer)>,
    descendant: Entity,
    appearance: &CreatureAppearance,
) {
    let Ok((player_entity, mut player)) = players.get_mut(descendant) else {
        return;
    };
    let Some(graph) = &appearance.graph else {
        return;
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
        CreatureAnimationLink {
            owner: appearance.owner,
            nodes: appearance.nodes.clone(),
            current_state: "idle".to_owned(),
            current_revision: 0,
        },
    ));
}

/// Rotate only the glTF wrapper; the root transform and its world-space
/// collider remain aligned with the movement system.
pub(super) fn sync_creature_facing(
    motions: Query<&CreatureMotion, With<CreatureInstance>>,
    mut appearances: Query<(&CreatureAppearance, &mut Transform)>,
) {
    for (appearance, mut transform) in &mut appearances {
        let Ok(motion) = motions.get(appearance.owner) else {
            continue;
        };
        let facing = Quat::from_rotation_y(motion.facing_yaw());
        if transform.rotation != facing {
            transform.rotation = facing;
        }
    }
}

pub(super) fn sync_creature_animations(
    states: Query<&CreatureAnimationState, With<CreatureInstance>>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut CreatureAnimationLink,
    )>,
) {
    for (mut player, mut transitions, mut link) in &mut players {
        let Ok(state) = states.get(link.owner) else {
            continue;
        };
        if link.current_state == state.name && link.current_revision == state.revision {
            continue;
        }
        let Some(index) = link.nodes.get(&state.name).copied() else {
            continue;
        };
        let animation = transitions.play(&mut player, index, Duration::from_millis(80));
        if state.name == "idle" || state.name == "airborne" {
            animation.repeat();
        } else {
            animation.set_repeat(RepeatAnimation::Count(1));
        }
        link.current_state.clone_from(&state.name);
        link.current_revision = state.revision;
    }
}
