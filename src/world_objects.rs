use std::{
    f32::consts::FRAC_PI_2,
    time::{Duration, Instant},
};

use bevy::{
    asset::AssetId,
    ecs::system::SystemParam,
    gltf::GltfAssetLabel,
    light::{NotShadowCaster, NotShadowReceiver},
    platform::collections::{HashMap, HashSet},
    prelude::*,
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
    player::{
        camera::GameplayCamera,
        item_stack::{ItemStack, MAX_STACK_SIZE},
    },
    rendering::block_tint::block_tint_at,
    voxel::{
        cell::VoxelCell,
        chunk::CHUNK_SIZE,
        coordinates::{chunk_coord_from_position, chunk_coord_from_world},
        log_variant::is_hollow_log_id,
        microblock::HOLLOW_LOG_WALL_THICKNESS,
        object::ObjectCell,
        texture_rotation::TextureRotation,
        world::VoxelWorld,
    },
    world::{
        biome_field::BiomeField,
        deterministic::{hash_signed, hash_string, mix_u32_components},
        render_distance::{RenderDistanceSettings, chunk_visibility_radii},
        tick::WorldTickClock,
        WorldFrameWorkBudget,
    },
    world_items::WorldItemSpawnRequest,
};

mod extruded_sprite;

use extruded_sprite::{
    configure_pending_extruded_sprites, ExtrudedSpriteGeometry, PendingExtrudedSprite,
};
pub(crate) use extruded_sprite::{ExtrudedSpriteMaterialCache, ExtrudedSpriteMeshCache};

#[derive(Clone, Copy)]
struct MaterializedWorldObject {
    entity: Entity,
    object: ObjectCell,
    support_cell: Option<VoxelCell>,
}

#[derive(Resource, Default)]
pub(crate) struct WorldObjectStore {
    by_support: HashMap<IVec3, MaterializedWorldObject>,
    by_chunk: HashMap<IVec3, HashSet<IVec3>>,
    synced_chunk_revisions: HashMap<IVec3, u64>,
    synced_world_revision: u64,
    materialized_center: Option<IVec2>,
    materialized_show_radius: i32,
    materialized_hide_radius: i32,
}

impl WorldObjectStore {
    pub(crate) fn materialized_object_count(&self) -> usize {
        self.by_support.len()
    }

    pub(crate) fn materialized_chunk_count(&self) -> usize {
        self.synced_chunk_revisions.len()
    }

    fn insert(
        &mut self,
        support: IVec3,
        object: ObjectCell,
        support_cell: Option<VoxelCell>,
        entity: Entity,
    ) {
        let previous = self.by_support.insert(
            support,
            MaterializedWorldObject {
                entity,
                object,
                support_cell,
            },
        );
        debug_assert!(previous.is_none(), "world object support must be unique");
        self.by_chunk
            .entry(chunk_coord_from_world(support))
            .or_default()
            .insert(support);
    }

    fn remove_support(&mut self, support: IVec3) -> Option<Entity> {
        let entity = self.by_support.remove(&support)?.entity;
        let coord = chunk_coord_from_world(support);
        let remove_chunk_entry = if let Some(supports) = self.by_chunk.get_mut(&coord) {
            supports.remove(&support);
            supports.is_empty()
        } else {
            false
        };
        if remove_chunk_entry {
            self.by_chunk.remove(&coord);
        }
        Some(entity)
    }

    fn take_chunk_entities(&mut self, coord: IVec3) -> Vec<Entity> {
        let Some(supports) = self.by_chunk.remove(&coord) else {
            return Vec::new();
        };
        supports
            .into_iter()
            .filter_map(|support| self.by_support.remove(&support).map(|entry| entry.entity))
            .collect()
    }
}

#[derive(Resource, Default)]
pub(crate) struct TargetedWorldObject(pub(crate) Option<IVec3>);

#[derive(Message)]
pub(crate) struct WorldObjectPlaceRequest {
    pub(crate) support: IVec3,
    pub(crate) object: ObjectCell,
}

#[derive(Message)]
pub(crate) struct WorldObjectRemoveRequest {
    pub(crate) support: IVec3,
    pub(crate) drop_loot: bool,
}

#[derive(SystemParam)]
struct WorldObjectRemovalRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
    store: ResMut<'w, WorldObjectStore>,
    world_ticks: Res<'w, WorldTickClock>,
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

pub(crate) struct ObjectLootRegistries<'a> {
    blocks: &'a BlockRegistry,
    items: &'a ItemRegistry,
    layers: &'a LayerRegistry,
    objects: &'a ObjectRegistry,
    tools: &'a ToolRegistry,
}

impl<'a> ObjectLootRegistries<'a> {
    pub(crate) fn new(
        blocks: &'a BlockRegistry,
        items: &'a ItemRegistry,
        layers: &'a LayerRegistry,
        objects: &'a ObjectRegistry,
        tools: &'a ToolRegistry,
    ) -> Self {
        Self {
            blocks,
            items,
            layers,
            objects,
            tools,
        }
    }
}

#[derive(Component)]
struct WorldObjectAppearance {
    tint: Color,
    unlit: bool,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ObjectMaterialKey {
    material: AssetId<StandardMaterial>,
    tint: [u8; 4],
    unlit: bool,
}

#[derive(Component)]
struct PendingObjectModelMaterial {
    source: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub(crate) struct ObjectMaterialCache(HashMap<ObjectMaterialKey, Handle<StandardMaterial>>);

impl ObjectMaterialCache {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(SystemParam)]
struct WorldObjectSceneContent<'w> {
    world: Res<'w, VoxelWorld>,
    objects: Res<'w, ObjectRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    asset_server: Res<'w, AssetServer>,
}

pub(crate) struct WorldObjectsPlugin;

impl Plugin for WorldObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldObjectStore>()
            .init_resource::<TargetedWorldObject>()
            .init_resource::<ObjectMaterialCache>()
            .init_resource::<ExtrudedSpriteMeshCache>()
            .init_resource::<ExtrudedSpriteMaterialCache>()
            .add_message::<WorldObjectPlaceRequest>()
            .add_message::<WorldObjectRemoveRequest>()
            .add_systems(
                PostUpdate,
                (
                    apply_object_placement_requests,
                    apply_object_removal_requests,
                    sync_world_objects,
                    configure_pending_object_model_materials,
                    configure_pending_extruded_sprites,
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
        let Some(object) = runtime.world.object_at(request.support) else {
            continue;
        };
        let Some(definition) = content.objects.get(object.object_id) else {
            continue;
        };
        let support_cell = runtime.world.cell_at(request.support);
        let loot_position =
            world_object_position(request.support, support_cell, object, definition)
                + Vec3::Y * 0.25;
        let Some((_chunk, removed)) = runtime.world.remove_object_at(request.support) else {
            continue;
        };

        if let Some(entity) = runtime.store.remove_support(request.support) {
            commands.entity(entity).despawn();
        }
        if request.drop_loot {
            spawn_object_loot(
                definition,
                request.support,
                loot_position,
                runtime.world_ticks.current_tick(),
                &content,
                &mut runtime.drops,
            );
        }
        debug_assert_eq!(removed.object_id, object.object_id);
    }
}

const WORLD_OBJECT_SYNC_BUDGET: Duration = Duration::from_millis(2);
const MAX_WORLD_OBJECT_CHUNK_UPDATES_PER_FRAME: usize = 2;
const SLOW_WORLD_OBJECT_SYNC_WARNING: Duration = Duration::from_millis(4);

fn sync_world_objects(
    mut commands: Commands,
    content: WorldObjectSceneContent,
    player: Single<&Transform, With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    frame_budget: Res<WorldFrameWorkBudget>,
    mut store: ResMut<WorldObjectStore>,
) {
    let world_revision = content.world.object_scene_revision();
    let player_chunk = chunk_coord_from_position(player.translation);
    let center = IVec2::new(player_chunk.x, player_chunk.z);
    let (show_radius, hide_radius) = chunk_visibility_radii(render_distance.chunks());
    if store.synced_world_revision == world_revision
        && store.materialized_center == Some(center)
        && store.materialized_show_radius == show_radius
        && store.materialized_hide_radius == hide_radius
    {
        return;
    }

    let sync_started = Instant::now();
    let candidate_coords = content
        .world
        .loaded_chunk_coords_in_horizontal_radius(center, hide_radius);
    let candidate_chunk_count = candidate_coords.len();
    let retired = store
        .synced_chunk_revisions
        .keys()
        .copied()
        .filter(|coord| {
            content.world.chunk(*coord).is_none()
                || !chunk_inside_object_radius(*coord, center, hide_radius)
        })
        .collect::<Vec<_>>();
    let mut processed_chunks = 0;
    let mut deferred = false;

    for coord in retired {
        if world_object_sync_budget_exhausted(
            sync_started,
            processed_chunks,
            frame_budget.deadline(),
        ) {
            deferred = true;
            break;
        }
        despawn_chunk_objects(&mut commands, coord, &mut store);
        store.synced_chunk_revisions.remove(&coord);
        processed_chunks += 1;
    }

    if !deferred {
        for coord in candidate_coords {
            let already_materialized = store.synced_chunk_revisions.contains_key(&coord);
            let radius = if already_materialized {
                hide_radius
            } else {
                show_radius
            };
            if !chunk_inside_object_radius(coord, center, radius) {
                continue;
            }

            let revision = content
                .world
                .chunk_object_revision(coord)
                .expect("loaded chunk must expose an object revision");
            if store.synced_chunk_revisions.get(&coord).copied() == Some(revision) {
                continue;
            }
            if world_object_sync_budget_exhausted(
                sync_started,
                processed_chunks,
                frame_budget.deadline(),
            ) {
                deferred = true;
                break;
            }

            let Some(chunk) = content.world.chunk(coord) else {
                continue;
            };
            let chunk_origin = coord * CHUNK_SIZE as i32;
            let existing_supports = store
                .by_chunk
                .get(&coord)
                .cloned()
                .unwrap_or_default();
            let mut desired_supports = HashSet::new();

            for (x, y, z, object) in chunk.object_voxels() {
                let support = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
                desired_supports.insert(support);
                let support_cell = content.world.cell_at(support);
                if store.by_support.get(&support).is_some_and(|existing| {
                    existing.object == object && existing.support_cell == support_cell
                }) {
                    continue;
                }

                if let Some(entity) = store.remove_support(support) {
                    commands.entity(entity).despawn();
                }
                let Some(definition) = content.objects.get(object.object_id) else {
                    continue;
                };
                let entity = spawn_world_object(
                    &mut commands,
                    support,
                    support_cell,
                    object,
                    definition,
                    &content,
                );
                store.insert(support, object, support_cell, entity);
            }

            for support in existing_supports {
                if desired_supports.contains(&support) {
                    continue;
                }
                if let Some(entity) = store.remove_support(support) {
                    commands.entity(entity).despawn();
                }
            }

            store.synced_chunk_revisions.insert(coord, revision);
            processed_chunks += 1;
        }
    }

    if !deferred {
        store.synced_world_revision = world_revision;
        store.materialized_center = Some(center);
        store.materialized_show_radius = show_radius;
        store.materialized_hide_radius = hide_radius;
    }

    let elapsed = sync_started.elapsed();
    if elapsed >= SLOW_WORLD_OBJECT_SYNC_WARNING {
        warn!(
            "slow world-object sync: center={center:?} show_radius={show_radius} hide_radius={hide_radius} candidate_chunks={} processed_chunks={} deferred={} materialized_chunks={} materialized_objects={} elapsed_ms={:.2}",
            candidate_chunk_count,
            processed_chunks,
            deferred,
            store.materialized_chunk_count(),
            store.materialized_object_count(),
            elapsed.as_secs_f64() * 1_000.0,
        );
    }
}

fn world_object_sync_budget_exhausted(
    started: Instant,
    processed_chunks: usize,
    global_deadline: Instant,
) -> bool {
    processed_chunks >= MAX_WORLD_OBJECT_CHUNK_UPDATES_PER_FRAME
        || (processed_chunks > 0
            && (started.elapsed() >= WORLD_OBJECT_SYNC_BUDGET
                || Instant::now() >= global_deadline))
}


fn chunk_inside_object_radius(coord: IVec3, center: IVec2, radius: i32) -> bool {
    if radius < 0 {
        return false;
    }
    let delta = IVec2::new(coord.x, coord.z) - center;
    delta.length_squared() <= radius * radius
}

fn despawn_chunk_objects(
    commands: &mut Commands,
    coord: IVec3,
    store: &mut WorldObjectStore,
) {
    for entity in store.take_chunk_entities(coord) {
        commands.entity(entity).despawn();
    }
}

fn spawn_world_object(
    commands: &mut Commands,
    support: IVec3,
    support_cell: Option<VoxelCell>,
    object: ObjectCell,
    definition: &ObjectDefinition,
    content: &WorldObjectSceneContent<'_>,
) -> Entity {
    let position = world_object_position(support, support_cell, object, definition);
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
        transform,
        Visibility::Visible,
        DespawnOnExit(GameState::Gameplay),
    ));

    match &definition.visual {
        ObjectVisualDefinition::Model { path } => {
            let mesh = content.asset_server.load(
                GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(path.clone()),
            );
            let source_material: Handle<StandardMaterial> =
                content.asset_server.load(format!("{path}#Material0/std"));
            root.insert((
                Mesh3d(mesh),
                MeshMaterial3d(source_material.clone()),
                WorldObjectAppearance {
                    tint,
                    unlit: definition.unlit,
                },
                PendingObjectModelMaterial {
                    source: source_material,
                },
            ));
            apply_shadow_flags(&mut root, definition);
        }
        ObjectVisualDefinition::ExtrudedSprite {
            texture,
            base_offset,
            height,
            size,
            alpha_cutoff,
        } => {
            root.insert(PendingExtrudedSprite::new(
                content.asset_server.load(texture.clone()),
                ExtrudedSpriteGeometry {
                    size: *size,
                    height: *height,
                    base_offset: *base_offset,
                    alpha_cutoff: *alpha_cutoff,
                },
                tint,
                definition.unlit,
            ));
            apply_shadow_flags(&mut root, definition);
        }
    }

    root.id()
}


pub(crate) fn world_object_position(
    support: IVec3,
    support_cell: Option<VoxelCell>,
    object: ObjectCell,
    definition: &ObjectDefinition,
) -> Vec3 {
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
    base_position + object_position_jitter(definition, support, hollow_orientation)
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
    emit_object_loot(
        definition,
        support,
        current_tick,
        ObjectLootRegistries::new(
            &content.blocks,
            &content.items,
            &content.layers,
            &content.objects,
            &content.tools,
        ),
        |stack| {
            drops.write(WorldItemSpawnRequest::dropped(stack, position));
        },
    );
}

pub(crate) fn emit_object_loot(
    definition: &ObjectDefinition,
    support: IVec3,
    current_tick: u64,
    content: ObjectLootRegistries<'_>,
    mut emit: impl FnMut(ItemStack),
) {
    if definition.loot_table.entries().is_empty() {
        if definition.drop_self {
            emit(ItemStack::new(intern_object_id(&definition.id)));
        }
        return;
    }

    let mut random_state = object_loot_random_seed(support, current_tick);
    for entry in definition.loot_table.entries() {
        if entry.chance < 1.0 && next_unit_f32(&mut random_state) >= entry.chance {
            continue;
        }
        let item_id = resolve_object_loot_item_id(&entry.item, &content);
        let mut remaining = entry.quantity;
        while remaining > 0 {
            let quantity = remaining.min(MAX_STACK_SIZE);
            remaining -= quantity;
            emit(ItemStack::new(item_id).with_quantity(quantity));
        }
    }
}

fn resolve_object_loot_item_id(
    item_id: &str,
    content: &ObjectLootRegistries<'_>,
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

fn configure_pending_object_model_materials(
    mut commands: Commands,
    pending: Query<(
        Entity,
        &PendingObjectModelMaterial,
        &WorldObjectAppearance,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<ObjectMaterialCache>,
) {
    for (entity, pending, appearance) in &pending {
        let (tint, tint_key) = quantized_object_tint(appearance.tint);
        let key = ObjectMaterialKey {
            material: pending.source.id(),
            tint: tint_key,
            unlit: appearance.unlit,
        };
        let replacement = if let Some(existing) = cache.0.get(&key) {
            existing.clone()
        } else {
            let Some(mut material) = materials.get(&pending.source).cloned() else {
                continue;
            };
            material.base_color = tint;
            material.unlit = appearance.unlit;
            let handle = materials.add(material);
            cache.0.insert(key, handle.clone());
            handle
        };

        commands
            .entity(entity)
            .insert(MeshMaterial3d(replacement))
            .remove::<PendingObjectModelMaterial>()
            .remove::<WorldObjectAppearance>();
    }
}

const OBJECT_TINT_RGB_LEVELS: f32 = 31.0;

fn quantized_object_tint(color: Color) -> (Color, [u8; 4]) {
    let rgba = color.to_srgba();
    let quantize_rgb = |value: f32| {
        (value.clamp(0.0, 1.0) * OBJECT_TINT_RGB_LEVELS).round() as u8
    };
    let red = quantize_rgb(rgba.red);
    let green = quantize_rgb(rgba.green);
    let blue = quantize_rgb(rgba.blue);
    let alpha = (rgba.alpha.clamp(0.0, 1.0) * 255.0).round() as u8;

    (
        Color::srgba(
            f32::from(red) / OBJECT_TINT_RGB_LEVELS,
            f32::from(green) / OBJECT_TINT_RGB_LEVELS,
            f32::from(blue) / OBJECT_TINT_RGB_LEVELS,
            f32::from(alpha) / 255.0,
        ),
        [red, green, blue, alpha],
    )
}

fn apply_shadow_flags(root: &mut EntityCommands<'_>, definition: &ObjectDefinition) {
    if !definition.casts_shadow {
        root.insert(NotShadowCaster);
    }
    if !definition.receives_shadow {
        root.insert(NotShadowReceiver);
    }
}
