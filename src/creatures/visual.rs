use std::{collections::HashMap, time::Duration};

use bevy::{
    animation::RepeatAnimation, asset::AssetId, ecs::system::SystemParam, gltf::GltfMaterialName,
    prelude::*, world_serialization::WorldInstanceReady,
};

use crate::content::{color::Hsi, creature::CreatureRegistry};

use super::CreatureInstance;

/// The glTF asset belongs to the visual loader; the root owns physics/position.
#[derive(Component)]
pub(super) struct CreatureModel(pub Handle<Gltf>);

#[derive(Component)]
pub(super) struct VisualAttached;

#[derive(Component)]
struct CreatureAppearance {
    owner: Entity,
    material_tints: HashMap<String, Hsi>,
    material_textures: HashMap<String, Handle<Image>>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: HashMap<String, AnimationNodeIndex>,
}

#[derive(Component)]
pub(super) struct CreatureAnimationLink {
    owner: Entity,
    nodes: HashMap<String, AnimationNodeIndex>,
    current_state: String,
}

/// A gameplay system may change this on the physics root. Clip names are
/// resolved solely from the corresponding creature JSON, not from Rust.
#[derive(Component)]
pub(crate) struct CreatureAnimationState(pub String);

#[derive(Resource, Default)]
pub(super) struct TintedCreatureMaterials(
    HashMap<
        (
            AssetId<StandardMaterial>,
            Option<[u32; 3]>,
            Option<AssetId<Image>>,
        ),
        Handle<StandardMaterial>,
    >,
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
        let textures = definition
            .textures
            .iter()
            .map(|(material, path)| (material.clone(), asset_server.load::<Image>(path.clone())))
            .collect();
        commands
            .entity(root)
            .insert((VisualAttached, CreatureAnimationState("idle".to_owned())));
        commands.entity(root).with_children(|parent| {
            parent
                .spawn((
                    WorldAssetRoot(scene),
                    CreatureAppearance {
                        owner: root,
                        material_tints: tints,
                        material_textures: textures,
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

fn configure_loaded_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    appearances: Query<&CreatureAppearance>,
    mesh_materials: Query<(&MeshMaterial3d<StandardMaterial>, &GltfMaterialName)>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    mut tint_assets: CreatureTintAssets,
) {
    let Ok(appearance) = appearances.get(ready.entity) else {
        return;
    };
    for descendant in descendants.iter_descendants(ready.entity) {
        if let Ok((original, material_name)) = mesh_materials.get(descendant) {
            let name = material_name.0.as_str();
            let tint = appearance.material_tints.get(name);
            let texture = appearance.material_textures.get(name);
            if tint.is_some() || texture.is_some() {
                let rgb = tint.map(|color| color.to_srgb());
                let cache_key = (
                    original.id(),
                    rgb.map(|color| color.map(f32::to_bits)),
                    texture.map(|image| image.id()),
                );
                let replacement = if let Some(existing) = tint_assets.cache.0.get(&cache_key) {
                    Some(existing.clone())
                } else {
                    tint_assets
                        .materials
                        .get(original.id())
                        .cloned()
                        .map(|mut material| {
                            if let Some(color) = rgb {
                                let alpha = material.base_color.to_srgba().alpha;
                                material.base_color =
                                    Color::srgba(color[0], color[1], color[2], alpha);
                            }
                            if let Some(image) = texture {
                                material.base_color_texture = Some(image.clone());
                            }
                            let handle = tint_assets.materials.add(material);
                            tint_assets.cache.0.insert(cache_key, handle.clone());
                            handle
                        })
                };
                if let Some(material) = replacement {
                    commands.entity(descendant).insert(MeshMaterial3d(material));
                }
            }
        }

        if let Ok((player_entity, mut player)) = players.get_mut(descendant)
            && let Some(graph) = &appearance.graph
        {
            let mut transitions = AnimationTransitions::new();
            let initial = appearance.nodes.get("idle").copied();
            if let Some(index) = initial {
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
                },
            ));
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
        if link.current_state == state.0 {
            continue;
        }
        let Some(index) = link.nodes.get(&state.0).copied() else {
            continue;
        };
        let animation = transitions.play(&mut player, index, Duration::from_millis(80));
        if state.0 == "idle" || state.0 == "airborne" {
            animation.repeat();
        } else {
            animation.set_repeat(RepeatAnimation::Count(1));
        }
        link.current_state.clone_from(&state.0);
    }
}
