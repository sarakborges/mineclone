use bevy::{
    asset::AssetId,
    gltf::GltfAssetLabel,
    light::{NotShadowCaster, NotShadowReceiver},
    platform::collections::HashMap,
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{
        biome::BiomeRegistry,
        object::{ObjectDefinition, ObjectRegistry},
    },
    player::item_stack::ItemStack,
    rendering::block_tint::block_tint_at,
    voxel::{coordinates::chunk_coord_from_world, world::VoxelWorld},
    world::{biome_field::BiomeField, chunk_rendering::ChunkRenderCoord},
    world_items::WorldItemSpawnRequest,
};

#[derive(Component)]
pub(crate) struct WorldObjectInstance {
    object_id: &'static str,
    anchor: IVec3,
    target_size: Vec3,
    target_center_offset: Vec3,
}

impl WorldObjectInstance {
    fn new(object_id: &'static str, anchor: IVec3, definition: &ObjectDefinition) -> Self {
        Self {
            object_id,
            anchor,
            target_size: Vec3::from_array(definition.target.size),
            target_center_offset: Vec3::from_array(definition.target.center_offset),
        }
    }

    pub(crate) fn target_bounds(&self, origin: Vec3) -> (Vec3, Vec3) {
        let center = origin + self.target_center_offset;
        let half = self.target_size * 0.5;
        (center - half, center + half)
    }
}

#[derive(Resource, Default)]
pub(crate) struct WorldObjectStore {
    by_anchor: HashMap<IVec3, Entity>,
}

impl WorldObjectStore {
    pub(crate) fn contains(&self, anchor: IVec3) -> bool {
        self.by_anchor.contains_key(&anchor)
    }
}

#[derive(Resource, Default)]
pub(crate) struct TargetedWorldObject(pub(crate) Option<Entity>);

#[derive(Message)]
pub(crate) struct WorldObjectPlaceRequest {
    pub(crate) object_id: &'static str,
    pub(crate) anchor: IVec3,
}

#[derive(Message)]
pub(crate) struct WorldObjectRemoveRequest {
    pub(crate) entity: Entity,
    pub(crate) drop_self: bool,
}

#[derive(Component)]
struct WorldObjectAppearance {
    tint: Color,
    unlit: bool,
    casts_shadow: bool,
    receives_shadow: bool,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ObjectMaterialKey {
    material: AssetId<StandardMaterial>,
    tint: [u32; 4],
    unlit: bool,
}

#[derive(Resource, Default)]
struct ObjectMaterialCache(HashMap<ObjectMaterialKey, Handle<StandardMaterial>>);

pub(crate) struct WorldObjectsPlugin;

impl Plugin for WorldObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldObjectStore>()
            .init_resource::<TargetedWorldObject>()
            .init_resource::<ObjectMaterialCache>()
            .add_message::<WorldObjectPlaceRequest>()
            .add_message::<WorldObjectRemoveRequest>()
            .add_systems(
                PostUpdate,
                (spawn_requested_objects, remove_requested_objects)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                (
                    reset_resource::<WorldObjectStore>,
                    reset_resource::<TargetedWorldObject>,
                ),
            );
    }
}

fn spawn_requested_objects(
    mut commands: Commands,
    mut requests: MessageReader<WorldObjectPlaceRequest>,
    world: Res<VoxelWorld>,
    objects: Res<ObjectRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    asset_server: Res<AssetServer>,
    mut store: ResMut<WorldObjectStore>,
) {
    for request in requests.read() {
        if store.contains(request.anchor)
            || !world.is_loaded_at(request.anchor)
            || world.cell_at(request.anchor).is_some()
        {
            continue;
        }
        let Some(definition) = objects.get(request.object_id) else {
            continue;
        };

        let position = request.anchor.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
        let tint = block_tint_at(
            definition.tint,
            Vec2::new(position.x, position.z),
            &biome_field,
            &biomes,
        );
        let scene = asset_server.load(
            GltfAssetLabel::Scene(0).from_asset(definition.model.clone()),
        );
        let appearance = WorldObjectAppearance {
            tint,
            unlit: definition.unlit,
            casts_shadow: definition.casts_shadow,
            receives_shadow: definition.receives_shadow,
        };
        let entity = commands
            .spawn((
                Name::new(format!("World Object ({})", definition.id)),
                WorldObjectInstance::new(request.object_id, request.anchor, definition),
                Transform::from_translation(position),
                Visibility::Hidden,
                ChunkRenderCoord(chunk_coord_from_world(request.anchor)),
                DespawnOnExit(GameState::Gameplay),
            ))
            .with_children(|root| {
                root.spawn((WorldAssetRoot(scene), appearance))
                    .observe(configure_loaded_object_scene);
            })
            .id();
        store.by_anchor.insert(request.anchor, entity);
    }
}

fn remove_requested_objects(
    mut commands: Commands,
    mut requests: MessageReader<WorldObjectRemoveRequest>,
    mut store: ResMut<WorldObjectStore>,
    objects: Res<ObjectRegistry>,
    instances: Query<(&WorldObjectInstance, &Transform)>,
    mut drops: MessageWriter<WorldItemSpawnRequest>,
) {
    for request in requests.read() {
        let Ok((instance, transform)) = instances.get(request.entity) else {
            continue;
        };
        store.by_anchor.remove(&instance.anchor);
        if request.drop_self
            && objects
                .get(instance.object_id)
                .is_some_and(|definition| definition.drop_self)
        {
            drops.write(WorldItemSpawnRequest::dropped(
                ItemStack::new(instance.object_id),
                transform.translation + Vec3::Y * 0.25,
            ));
        }
        commands.entity(request.entity).despawn();
    }
}

fn configure_loaded_object_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    appearances: Query<&WorldObjectAppearance>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<ObjectMaterialCache>,
) {
    let Ok(appearance) = appearances.get(ready.entity) else {
        return;
    };
    let rgba = appearance.tint.to_srgba();
    let tint = [
        rgba.red.to_bits(),
        rgba.green.to_bits(),
        rgba.blue.to_bits(),
        rgba.alpha.to_bits(),
    ];

    for descendant in children.iter_descendants(ready.entity) {
        if !appearance.casts_shadow {
            commands.entity(descendant).insert(NotShadowCaster);
        }
        if !appearance.receives_shadow {
            commands.entity(descendant).insert(NotShadowReceiver);
        }

        let Ok(original) = mesh_materials.get(descendant) else {
            continue;
        };
        let key = ObjectMaterialKey {
            material: original.id(),
            tint,
            unlit: appearance.unlit,
        };
        let replacement = if let Some(existing) = cache.0.get(&key) {
            existing.clone()
        } else {
            let Some(mut material) = materials.get(original.id()).cloned() else {
                continue;
            };
            material.base_color = appearance.tint;
            material.unlit = appearance.unlit;
            let handle = materials.add(material);
            cache.0.insert(key, handle.clone());
            handle
        };
        commands.entity(descendant).insert(MeshMaterial3d(replacement));
    }
}
