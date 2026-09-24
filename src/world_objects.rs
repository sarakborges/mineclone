use std::f32::consts::FRAC_PI_2;

use bevy::{
    asset::AssetId,
    ecs::system::SystemParam,
    gltf::GltfAssetLabel,
    light::{NotShadowCaster, NotShadowReceiver},
    platform::collections::{HashMap, HashSet},
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        block_id::intern_block_id,
        block_orientation::BlockOrientation,
        item::ItemRegistry,
        item_id::intern_item_id,
        layer::LayerRegistry,
        layer_id::intern_layer_id,
        object::{ObjectDefinition, ObjectRegistry, ObjectVisualDefinition},
        object_id::intern_object_id,
        tool::ToolRegistry,
        tool_id::intern_tool_id,
    },
    gameplay::random::next_unit_f32,
    player::item_stack::{ItemStack, MAX_STACK_SIZE},
    rendering::block_tint::block_tint_at,
    voxel::{
        chunk::CHUNK_SIZE,
        coordinates::chunk_coord_from_world,
        log_variant::is_hollow_log_id,
        microblock::HOLLOW_LOG_WALL_THICKNESS,
        object::ObjectCell,
        texture_rotation::TextureRotation,
        world::VoxelWorld,
    },
    world::{
        biome_field::BiomeField,
        chunk_rendering::ChunkRenderCoord,
        deterministic::{hash_signed, hash_string, mix_u32_components},
        tick::WorldTickClock,
    },
    world_items::WorldItemSpawnRequest,
};

#[derive(Component)]
pub(crate) struct WorldObjectInstance {
    object_id: &'static str,
    support: IVec3,
    target_size: Vec3,
    target_center_offset: Vec3,
}

impl WorldObjectInstance {
    fn new(object_id: &'static str, support: IVec3, definition: &ObjectDefinition) -> Self {
        Self {
            object_id,
            support,
            target_size: Vec3::from_array(definition.target.size),
            target_center_offset: Vec3::from_array(definition.target.center_offset),
        }
    }

    pub(crate) fn object_id(&self) -> &'static str {
        self.object_id
    }


    pub(crate) fn target_bounds(&self, origin: Vec3) -> (Vec3, Vec3) {
        let center = origin + self.target_center_offset;
        let half = self.target_size * 0.5;
        (center - half, center + half)
    }
}

#[derive(Resource, Default)]
pub(crate) struct WorldObjectStore {
    by_support: HashMap<IVec3, Entity>,
    synced_chunk_revisions: HashMap<IVec3, u64>,
}

#[derive(Resource, Default)]
pub(crate) struct TargetedWorldObject(pub(crate) Option<Entity>);

#[derive(Message)]
pub(crate) struct WorldObjectPlaceRequest {
    pub(crate) support: IVec3,
    pub(crate) object: ObjectCell,
}

#[derive(Message)]
pub(crate) struct WorldObjectRemoveRequest {
    pub(crate) entity: Entity,
    pub(crate) drop_loot: bool,
}


#[derive(SystemParam)]
struct WorldObjectRemovalRuntime<'w, 's> {
    world: ResMut<'w, VoxelWorld>,
    store: ResMut<'w, WorldObjectStore>,
    world_ticks: Res<'w, WorldTickClock>,
    instances: Query<'w, 's, (&'static WorldObjectInstance, &'static Transform)>,
    drops: MessageWriter<'w, WorldItemSpawnRequest>,
}

#[derive(SystemParam)]
struct WorldObjectRemovalContent<'w> {
    blocks: Res<'w, BlockRegistry>,
    items: Res<'w, ItemRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
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
struct ObjectModelPreloads {
    _scenes: Vec<Handle<bevy::world_serialization::WorldAsset>>,
}

#[derive(Resource, Default)]
struct ObjectMaterialCache(HashMap<ObjectMaterialKey, Handle<StandardMaterial>>);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct StackedSpriteMeshKey {
    width: u32,
    depth: u32,
}

#[derive(Resource, Default)]
struct StackedSpriteMeshCache(HashMap<StackedSpriteMeshKey, Handle<Mesh>>);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct StackedSpriteMaterialKey {
    object_id: &'static str,
    tint: [u32; 4],
    unlit: bool,
    alpha_cutoff: u32,
}

#[derive(Resource, Default)]
struct StackedSpriteMaterialCache(HashMap<StackedSpriteMaterialKey, Handle<StandardMaterial>>);

#[derive(SystemParam)]
struct WorldObjectSceneContent<'w> {
    world: Res<'w, VoxelWorld>,
    objects: Res<'w, ObjectRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    asset_server: Res<'w, AssetServer>,
}

#[derive(SystemParam)]
struct WorldObjectSceneAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    stacked_meshes: ResMut<'w, StackedSpriteMeshCache>,
    stacked_materials: ResMut<'w, StackedSpriteMaterialCache>,
}

pub(crate) struct WorldObjectsPlugin;

impl Plugin for WorldObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldObjectStore>()
            .init_resource::<TargetedWorldObject>()
            .init_resource::<ObjectModelPreloads>()
            .init_resource::<ObjectMaterialCache>()
            .init_resource::<StackedSpriteMeshCache>()
            .init_resource::<StackedSpriteMaterialCache>()
            .add_message::<WorldObjectPlaceRequest>()
            .add_message::<WorldObjectRemoveRequest>()
            .add_systems(PostStartup, preload_object_models)
            .add_systems(
                PostUpdate,
                (
                    apply_object_placement_requests,
                    apply_object_removal_requests,
                    sync_world_objects,
                )
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


fn preload_object_models(
    mut commands: Commands,
    objects: Res<ObjectRegistry>,
    asset_server: Res<AssetServer>,
    mut preloads: ResMut<ObjectModelPreloads>,
) {
    preloads._scenes = objects
        .iter()
        .filter_map(|definition| match &definition.visual {
            ObjectVisualDefinition::Model { path } => Some(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset(path.clone())),
            ),
            ObjectVisualDefinition::StackedSprites { .. } => None,
        })
        .collect();

    for scene in &preloads._scenes {
        commands.spawn((
            Name::new("Object Model Preload"),
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(Vec3::splat(-1_000_000.0)),
            Visibility::Hidden,
        ));
    }
}

fn apply_object_placement_requests(
    mut requests: MessageReader<WorldObjectPlaceRequest>,
    objects: Res<ObjectRegistry>,
    mut world: ResMut<VoxelWorld>,
) {
    for request in requests.read() {
        let _ = world.set_object_at(request.support, request.object, &objects);
    }
}

fn apply_object_removal_requests(
    mut commands: Commands,
    mut requests: MessageReader<WorldObjectRemoveRequest>,
    content: WorldObjectRemovalContent,
    mut runtime: WorldObjectRemovalRuntime,
) {
    for request in requests.read() {
        let Ok((instance, transform)) = runtime.instances.get(request.entity) else {
            continue;
        };
        let Some((_chunk, removed)) = runtime.world.remove_object_at(instance.support) else {
            continue;
        };

        runtime.store.by_support.remove(&instance.support);
        if request.drop_loot
            && let Some(definition) = content.objects.get(removed.object_id)
        {
            spawn_object_loot(
                definition,
                instance.support,
                transform.translation + Vec3::Y * 0.25,
                runtime.world_ticks.current_tick(),
                &content,
                &mut runtime.drops,
            );
        }
        commands.entity(request.entity).despawn();
    }
}

fn sync_world_objects(
    mut commands: Commands,
    content: WorldObjectSceneContent,
    mut assets: WorldObjectSceneAssets,
    mut store: ResMut<WorldObjectStore>,
) {
    let loaded_coords = content.world.loaded_chunk_coords().collect::<Vec<_>>();
    let loaded = loaded_coords.iter().copied().collect::<HashSet<_>>();

    let unloaded = store
        .synced_chunk_revisions
        .keys()
        .copied()
        .filter(|coord| !loaded.contains(coord))
        .collect::<Vec<_>>();
    for coord in unloaded {
        despawn_chunk_objects(&mut commands, coord, &mut store);
        store.synced_chunk_revisions.remove(&coord);
    }

    for coord in loaded_coords {
        let revision = content
            .world
            .chunk_content_revision(coord)
            .expect("loaded chunk must expose a content revision");
        if store.synced_chunk_revisions.get(&coord).copied() == Some(revision) {
            continue;
        }

        despawn_chunk_objects(&mut commands, coord, &mut store);
        let Some(chunk) = content.world.chunk(coord) else {
            continue;
        };
        let chunk_origin = coord * CHUNK_SIZE as i32;

        for (x, y, z, object) in chunk.object_voxels() {
            let support = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
            let Some(definition) = content.objects.get(object.object_id) else {
                continue;
            };
            let entity = spawn_world_object(
                &mut commands,
                support,
                object,
                definition,
                &content,
                &mut assets,
            );
            store.by_support.insert(support, entity);
        }

        store.synced_chunk_revisions.insert(coord, revision);
    }
}

fn despawn_chunk_objects(
    commands: &mut Commands,
    coord: IVec3,
    store: &mut WorldObjectStore,
) {
    let supports = store
        .by_support
        .keys()
        .copied()
        .filter(|support| chunk_coord_from_world(*support) == coord)
        .collect::<Vec<_>>();
    for support in supports {
        if let Some(entity) = store.by_support.remove(&support) {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_world_object(
    commands: &mut Commands,
    support: IVec3,
    object: ObjectCell,
    definition: &ObjectDefinition,
    content: &WorldObjectSceneContent<'_>,
    assets: &mut WorldObjectSceneAssets<'_>,
) -> Entity {
    let support_cell = content.world.cell_at(support);
    let hollow_orientation = support_cell
        .filter(|cell| is_hollow_log_id(cell.block_id))
        .filter(|_| object.face == crate::content::object::ObjectPlacementFace::Top)
        .map(|cell| cell.orientation);
    let base_position = if hollow_orientation.is_some() {
        support.as_vec3()
            + Vec3::new(0.5, HOLLOW_LOG_WALL_THICKNESS + 0.001, 0.5)
    } else {
        support.as_vec3()
            + Vec3::splat(0.5)
            + object.face.normal().as_vec3() * 0.5
    };
    let position =
        base_position + object_position_jitter(definition, support, hollow_orientation);
    let tint = block_tint_at(
        definition.tint,
        Vec2::new(position.x, position.z),
        &content.biome_field,
        &content.biomes,
    );
    let transform = Transform::from_translation(position)
        .with_rotation(Quat::from_rotation_y(texture_rotation_radians(object.rotation)));

    let mut root = commands.spawn((
        Name::new(format!("World Object ({})", definition.id)),
        WorldObjectInstance::new(object.object_id, support, definition),
        transform,
        Visibility::Hidden,
        ChunkRenderCoord(chunk_coord_from_world(support)),
        DespawnOnExit(GameState::Gameplay),
    ));

    match &definition.visual {
        ObjectVisualDefinition::Model { path } => {
            let scene = content
                .asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
            let appearance = WorldObjectAppearance {
                tint,
                unlit: definition.unlit,
                casts_shadow: definition.casts_shadow,
                receives_shadow: definition.receives_shadow,
            };
            root.with_children(|children| {
                children
                    .spawn((WorldAssetRoot(scene), appearance))
                    .observe(configure_loaded_object_scene);
            });
        }
        ObjectVisualDefinition::StackedSprites {
            texture,
            slices,
            base_offset,
            slice_spacing,
            size,
            alpha_cutoff,
        } => {
            let mesh_key = StackedSpriteMeshKey {
                width: size[0].to_bits(),
                depth: size[1].to_bits(),
            };
            let mesh = if let Some(existing) = assets.stacked_meshes.0.get(&mesh_key) {
                existing.clone()
            } else {
                let handle = assets.meshes.add(Rectangle::new(size[0], size[1]));
                assets.stacked_meshes.0.insert(mesh_key, handle.clone());
                handle
            };

            let rgba = tint.to_srgba();
            let material_key = StackedSpriteMaterialKey {
                object_id: object.object_id,
                tint: [
                    rgba.red.to_bits(),
                    rgba.green.to_bits(),
                    rgba.blue.to_bits(),
                    rgba.alpha.to_bits(),
                ],
                unlit: definition.unlit,
                alpha_cutoff: alpha_cutoff.to_bits(),
            };
            let material = if let Some(existing) = assets.stacked_materials.0.get(&material_key) {
                existing.clone()
            } else {
                let handle = assets.materials.add(StandardMaterial {
                    base_color: tint,
                    base_color_texture: Some(content.asset_server.load(texture.clone())),
                    alpha_mode: AlphaMode::Mask(*alpha_cutoff),
                    perceptual_roughness: 1.0,
                    unlit: definition.unlit,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                });
                assets
                    .stacked_materials
                    .0
                    .insert(material_key, handle.clone());
                handle
            };

            root.with_children(|children| {
                for slice in 0..*slices {
                    let y = *base_offset + *slice_spacing * f32::from(slice);
                    let mut slice_entity = children.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(Vec3::Y * y)
                            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                    ));
                    if !definition.casts_shadow {
                        slice_entity.insert(NotShadowCaster);
                    }
                    if !definition.receives_shadow {
                        slice_entity.insert(NotShadowReceiver);
                    }
                }
            });
        }
    }

    root.id()
}


fn object_position_jitter(
    definition: &ObjectDefinition,
    support: IVec3,
    hollow_orientation: Option<BlockOrientation>,
) -> Vec3 {
    let seed = mix_u32_components(
        hash_string(&definition.id),
        [support.x as u32, support.y as u32, support.z as u32],
    );
    let mut maximum = Vec2::from_array(definition.position_jitter);
    if let Some(orientation) = hollow_orientation {
        let half = Vec2::new(definition.target.size[0], definition.target.size[2]) * 0.5;
        let cavity_half = 0.5 - HOLLOW_LOG_WALL_THICKNESS;
        let full_half = Vec2::splat(0.5);
        let available_half = match orientation {
            BlockOrientation::X => Vec2::new(full_half.x, cavity_half),
            BlockOrientation::Y => Vec2::splat(cavity_half),
            BlockOrientation::Z => Vec2::new(cavity_half, full_half.y),
        };
        maximum = maximum.min((available_half - half).max(Vec2::ZERO));
    }
    Vec3::new(
        hash_signed(seed.rotate_left(17)) * maximum.x,
        0.0,
        hash_signed(seed.rotate_left(43)) * maximum.y,
    )
}

fn spawn_object_loot(
    definition: &ObjectDefinition,
    support: IVec3,
    position: Vec3,
    current_tick: u64,
    content: &WorldObjectRemovalContent<'_>,
    drops: &mut MessageWriter<WorldItemSpawnRequest>,
) {
    if definition.loot_table.entries().is_empty() {
        if definition.drop_self {
            drops.write(WorldItemSpawnRequest::dropped(
                ItemStack::new(intern_object_id(&definition.id)),
                position,
            ));
        }
        return;
    }

    let mut random_state = object_loot_random_seed(support, current_tick);
    for entry in definition.loot_table.entries() {
        if entry.chance < 1.0 && next_unit_f32(&mut random_state) >= entry.chance {
            continue;
        }
        let item_id = resolve_object_loot_item_id(&entry.item, content);
        let mut remaining = entry.quantity;
        while remaining > 0 {
            let quantity = remaining.min(MAX_STACK_SIZE);
            remaining -= quantity;
            drops.write(WorldItemSpawnRequest::dropped(
                ItemStack::new(item_id).with_quantity(quantity),
                position,
            ));
        }
    }
}

fn resolve_object_loot_item_id(
    item_id: &str,
    content: &WorldObjectRemovalContent<'_>,
) -> &'static str {
    if content.items.get(item_id).is_some() {
        intern_item_id(item_id)
    } else if content.blocks.get(item_id).is_some() {
        intern_block_id(item_id)
    } else if content.layers.get(item_id).is_some() {
        intern_layer_id(item_id)
    } else if content.objects.get(item_id).is_some() {
        intern_object_id(item_id)
    } else if content.tools.get(item_id).is_some() {
        intern_tool_id(item_id)
    } else {
        unreachable!("object loot references are validated during content loading: {item_id}")
    }
}

fn object_loot_random_seed(support: IVec3, current_tick: u64) -> u32 {
    let mut seed = (current_tick as u32) ^ ((current_tick >> 32) as u32).rotate_left(11);
    seed ^= (support.x as u32).wrapping_mul(0x9E37_79B9);
    seed ^= (support.y as u32).wrapping_mul(0x85EB_CA6B);
    seed ^= (support.z as u32).wrapping_mul(0xC2B2_AE35);
    if seed == 0 { 0xA341_316C } else { seed }
}

fn texture_rotation_radians(rotation: TextureRotation) -> f32 {
    match rotation {
        TextureRotation::Degrees0 => 0.0,
        TextureRotation::Degrees90 => FRAC_PI_2,
        TextureRotation::Degrees180 => FRAC_PI_2 * 2.0,
        TextureRotation::Degrees270 => FRAC_PI_2 * 3.0,
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
