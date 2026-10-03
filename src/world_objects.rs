use std::{
    f32::consts::FRAC_PI_2,
    time::{Duration, Instant},
};

use bevy::{asset::AssetId, ecs::system::SystemParam, platform::collections::HashMap, prelude::*};

use crate::{
    app::{
        crash_log::{log_diagnostic, log_gameplay_event, log_gameplay_warn},
        game_state::GameState,
        resource_systems::reset_resource,
    },
    content::{
        block::BlockRegistry,
        block_id::intern_block_id,
        block_orientation::BlockOrientation,
        item::ItemRegistry,
        item_id::intern_item_id,
        layer::LayerRegistry,
        layer_id::intern_layer_id,
        object::{ObjectDefinition, ObjectPlacementFace, ObjectRegistry},
        object_id::intern_object_id,
        tool::ToolRegistry,
        tool_id::intern_tool_id,
    },
    gameplay::random::next_unit_f32,
    player::{
        camera::GameplayCamera,
        item_stack::{ItemStack, MAX_STACK_SIZE},
    },
    voxel::{
        cell::VoxelCell, coordinates::chunk_coord_from_position, log_variant::is_hollow_log_id,
        microblock::HOLLOW_LOG_WALL_THICKNESS, object::ObjectCell,
        texture_rotation::TextureRotation, world::VoxelWorld,
    },
    world::{
        WorldFrameWorkBudget,
        chunk_rendering::ChunkRenderPool,
        deterministic::{hash_signed, hash_string, mix_u32_components},
        render_distance::{RenderDistanceSettings, chunk_visibility_radii},
        tick::WorldTickClock,
    },
    world_items::WorldItemSpawnRequest,
};

mod batch;

use batch::{BuiltWorldObjectChunk, WorldObjectBatchAssets, build_world_object_chunk};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct WorldObjectKey {
    pub(crate) support: IVec3,
    pub(crate) object: ObjectCell,
}

impl WorldObjectKey {
    pub(crate) const fn new(support: IVec3, object: ObjectCell) -> Self {
        Self { support, object }
    }
}

struct MaterializedWorldObjectChunk {
    entities: Vec<Entity>,
    object_count: usize,
    batch_count: usize,
}

#[derive(Resource, Default)]
pub(crate) struct WorldObjectStore {
    by_chunk: HashMap<IVec3, MaterializedWorldObjectChunk>,
    synced_chunk_revisions: HashMap<IVec3, u64>,
    synced_world_revision: u64,
    synced_render_pool_revision: u64,
    materialized_center: Option<IVec2>,
    materialized_show_radius: i32,
    materialized_hide_radius: i32,
}

impl WorldObjectStore {
    pub(crate) fn materialized_object_count(&self) -> usize {
        self.by_chunk.values().map(|chunk| chunk.object_count).sum()
    }

    pub(crate) fn materialized_chunk_count(&self) -> usize {
        self.by_chunk.len()
    }

    pub(crate) fn materialized_batch_count(&self) -> usize {
        self.by_chunk.values().map(|chunk| chunk.batch_count).sum()
    }

    fn replace_chunk(&mut self, coord: IVec3, built: BuiltWorldObjectChunk) -> Vec<Entity> {
        let retired = self
            .by_chunk
            .remove(&coord)
            .map(|chunk| chunk.entities)
            .unwrap_or_default();
        if built.object_count > 0 {
            self.by_chunk.insert(
                coord,
                MaterializedWorldObjectChunk {
                    entities: built.entities,
                    object_count: built.object_count,
                    batch_count: built.batch_count,
                },
            );
        }
        retired
    }

    fn take_chunk_entities(&mut self, coord: IVec3) -> Vec<Entity> {
        self.by_chunk
            .remove(&coord)
            .map(|chunk| chunk.entities)
            .unwrap_or_default()
    }
}

#[derive(Resource, Default)]
pub(crate) struct TargetedWorldObject(pub(crate) Option<WorldObjectKey>);

#[derive(Message)]
pub(crate) struct WorldObjectPlaceRequest {
    pub(crate) support: IVec3,
    pub(crate) object: ObjectCell,
}

#[derive(Message)]
pub(crate) struct WorldObjectRemoveRequest {
    pub(crate) key: WorldObjectKey,
    pub(crate) drop_loot: bool,
}

#[derive(SystemParam)]
struct WorldObjectRemovalRuntime<'w> {
    world: ResMut<'w, VoxelWorld>,
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

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ObjectMaterialKey {
    material: AssetId<StandardMaterial>,
    tint: [u8; 4],
    unlit: bool,
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
    biomes: Res<'w, crate::content::biome::BiomeRegistry>,
    biome_field: Res<'w, crate::world::biome_field::BiomeField>,
    asset_server: Res<'w, AssetServer>,
}

#[derive(SystemParam)]
struct WorldObjectSyncRuntime<'w> {
    render_distance: Res<'w, RenderDistanceSettings>,
    render_pool: Res<'w, ChunkRenderPool>,
    frame_budget: Res<'w, WorldFrameWorkBudget>,
    store: ResMut<'w, WorldObjectStore>,
}

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

fn apply_object_placement_requests(
    mut requests: MessageReader<WorldObjectPlaceRequest>,
    objects: Res<ObjectRegistry>,
    mut world: ResMut<VoxelWorld>,
) {
    for request in requests.read() {
        if world
            .set_object_at(request.support, request.object, &objects)
            .is_some()
        {
            log_gameplay_event(format!(
                "object.place applied object={} support={:?} face={:?} rotation={:?}",
                request.object.object_id,
                request.support,
                request.object.face,
                request.object.rotation
            ));
        } else {
            log_gameplay_warn(format!(
                "object.place failed object={} support={:?} face={:?} reason=world_mutation_rejected",
                request.object.object_id, request.support, request.object.face
            ));
        }
    }
}

fn apply_object_removal_requests(
    mut requests: MessageReader<WorldObjectRemoveRequest>,
    content: WorldObjectRemovalContent,
    mut runtime: WorldObjectRemovalRuntime,
) {
    for request in requests.read() {
        let key = request.key;
        if !runtime.world.objects_at(key.support).contains(&key.object) {
            log_gameplay_event(format!(
                "object.remove rejected object={} support={:?} reason=stale_request",
                key.object.object_id, key.support
            ));
            continue;
        }
        let Some(definition) = content.objects.get(key.object.object_id) else {
            log_gameplay_warn(format!(
                "object.remove rejected object={} support={:?} reason=missing_definition",
                key.object.object_id, key.support
            ));
            continue;
        };
        let support_cell = runtime.world.cell_at(key.support);
        let loot_position =
            world_object_position(key.support, support_cell, key.object, definition)
                + Vec3::Y * 0.25;
        let Some((_chunk, removed)) = runtime.world.remove_object_at(key.support, key.object)
        else {
            log_gameplay_warn(format!(
                "object.remove rejected object={} support={:?} reason=world_mutation_rejected",
                key.object.object_id, key.support
            ));
            continue;
        };

        if request.drop_loot {
            spawn_object_loot(
                definition,
                key.support,
                loot_position,
                runtime.world_ticks.current_tick(),
                &content,
                &mut runtime.drops,
            );
        }
        log_gameplay_event(format!(
            "object.remove applied object={} support={:?} drop_loot={}",
            removed.object_id, key.support, request.drop_loot
        ));
        debug_assert_eq!(removed, key.object);
    }
}

const WORLD_OBJECT_SYNC_BUDGET: Duration = Duration::from_millis(2);
const MAX_WORLD_OBJECT_CHUNK_UPDATES_PER_FRAME: usize = 16;
const SLOW_WORLD_OBJECT_SYNC_WARNING: Duration = Duration::from_millis(4);
const PERSISTED_SLOW_WORLD_OBJECT_SYNC_WARNING: Duration = Duration::from_millis(20);

fn sync_world_objects(
    mut commands: Commands,
    content: WorldObjectSceneContent,
    player: Single<&Transform, With<GameplayCamera>>,
    mut batch_assets: WorldObjectBatchAssets,
    runtime: WorldObjectSyncRuntime,
) {
    let WorldObjectSyncRuntime {
        render_distance,
        render_pool,
        frame_budget,
        mut store,
    } = runtime;
    let world_revision = content.world.object_scene_revision();
    let render_pool_revision = render_pool.membership_revision();
    let player_chunk = chunk_coord_from_position(player.translation);
    let center = IVec2::new(player_chunk.x, player_chunk.z);
    let (show_radius, hide_radius) = chunk_visibility_radii(render_distance.chunks());
    if store.synced_world_revision == world_revision
        && store.synced_render_pool_revision == render_pool_revision
        && store.materialized_center == Some(center)
        && store.materialized_show_radius == show_radius
        && store.materialized_hide_radius == hide_radius
    {
        return;
    }

    let sync_started = Instant::now();
    let candidate_coords = render_pool
        .active_coords()
        .filter(|coord| chunk_inside_object_radius(*coord, center, hide_radius))
        .filter(|coord| {
            store.synced_chunk_revisions.contains_key(coord)
                || content
                    .world
                    .chunk(*coord)
                    .is_some_and(|chunk| chunk.has_objects())
        })
        .collect::<Vec<_>>();
    let candidate_chunk_count = candidate_coords.len();
    let retired = store
        .synced_chunk_revisions
        .keys()
        .copied()
        .filter(|coord| {
            content.world.chunk(*coord).is_none()
                || !render_pool.contains(*coord)
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
            if !chunk.has_objects() {
                despawn_chunk_objects(&mut commands, coord, &mut store);
                store.synced_chunk_revisions.remove(&coord);
                processed_chunks += 1;
                continue;
            }

            let Some(built) =
                build_world_object_chunk(&mut commands, coord, chunk, &content, &mut batch_assets)
            else {
                deferred = true;
                break;
            };
            for entity in store.replace_chunk(coord, built) {
                commands.entity(entity).despawn();
            }
            store.synced_chunk_revisions.insert(coord, revision);
            processed_chunks += 1;
        }
    }

    if !deferred {
        store.synced_world_revision = world_revision;
        store.synced_render_pool_revision = render_pool_revision;
        store.materialized_center = Some(center);
        store.materialized_show_radius = show_radius;
        store.materialized_hide_radius = hide_radius;
    }

    let elapsed = sync_started.elapsed();
    if elapsed >= SLOW_WORLD_OBJECT_SYNC_WARNING {
        warn!(
            "slow world-object sync: center={center:?} show_radius={show_radius} hide_radius={hide_radius} candidate_chunks={} processed_chunks={} deferred={} materialized_chunks={} materialized_objects={} materialized_batches={} elapsed_ms={:.2}",
            candidate_chunk_count,
            processed_chunks,
            deferred,
            store.materialized_chunk_count(),
            store.materialized_object_count(),
            store.materialized_batch_count(),
            elapsed.as_secs_f64() * 1_000.0,
        );
    }
    if elapsed >= PERSISTED_SLOW_WORLD_OBJECT_SYNC_WARNING {
        log_diagnostic(format!(
            "world_object.sync slow center={center:?} show_radius={show_radius} hide_radius={hide_radius} candidate_chunks={} processed_chunks={} deferred={} materialized_chunks={} materialized_objects={} materialized_batches={} elapsed_ms={:.2}",
            candidate_chunk_count,
            processed_chunks,
            deferred,
            store.materialized_chunk_count(),
            store.materialized_object_count(),
            store.materialized_batch_count(),
            elapsed.as_secs_f64() * 1_000.0,
        ));
    }
}

fn world_object_sync_budget_exhausted(
    started: Instant,
    processed_chunks: usize,
    global_deadline: Instant,
) -> bool {
    processed_chunks >= MAX_WORLD_OBJECT_CHUNK_UPDATES_PER_FRAME
        || (processed_chunks > 0
            && (started.elapsed() >= WORLD_OBJECT_SYNC_BUDGET || Instant::now() >= global_deadline))
}

fn chunk_inside_object_radius(coord: IVec3, center: IVec2, radius: i32) -> bool {
    if radius < 0 {
        return false;
    }
    let delta = IVec2::new(coord.x, coord.z) - center;
    delta.length_squared() <= radius * radius
}

fn despawn_chunk_objects(commands: &mut Commands, coord: IVec3, store: &mut WorldObjectStore) {
    for entity in store.take_chunk_entities(coord) {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn world_object_transform(
    support: IVec3,
    support_cell: Option<VoxelCell>,
    object: ObjectCell,
    definition: &ObjectDefinition,
) -> Transform {
    let rotation = world_object_rotation(object);
    let position =
        world_object_position_with_rotation(support, support_cell, object, definition, rotation);
    Transform::from_translation(position)
        .with_rotation(rotation)
        .with_scale(object.transform.scale())
}

pub(crate) fn world_object_position(
    support: IVec3,
    support_cell: Option<VoxelCell>,
    object: ObjectCell,
    definition: &ObjectDefinition,
) -> Vec3 {
    let rotation = world_object_rotation(object);
    world_object_position_with_rotation(support, support_cell, object, definition, rotation)
}

fn world_object_position_with_rotation(
    support: IVec3,
    support_cell: Option<VoxelCell>,
    object: ObjectCell,
    definition: &ObjectDefinition,
    rotation: Quat,
) -> Vec3 {
    let hollow_orientation = support_cell
        .filter(|cell| is_hollow_log_id(cell.block_id))
        .filter(|_| object.face == ObjectPlacementFace::Top)
        .map(|cell| cell.orientation);
    let base_position = if hollow_orientation.is_some() {
        support.as_vec3() + Vec3::new(0.5, HOLLOW_LOG_WALL_THICKNESS + 0.001, 0.5)
    } else {
        support.as_vec3() + Vec3::splat(0.5) + object.face.normal().as_vec3() * 0.5
    };
    let local_offset = object.transform.offset()
        + object_position_jitter(
            definition,
            support,
            hollow_orientation,
            object.transform.scale(),
        );
    base_position + rotation * local_offset
}

fn world_object_rotation(object: ObjectCell) -> Quat {
    object_face_rotation(object.face)
        * Quat::from_rotation_y(texture_rotation_radians(object.rotation))
}

fn object_face_rotation(face: ObjectPlacementFace) -> Quat {
    match face {
        ObjectPlacementFace::Top => Quat::IDENTITY,
        ObjectPlacementFace::Bottom => Quat::from_rotation_x(FRAC_PI_2 * 2.0),
        ObjectPlacementFace::Right => Quat::from_rotation_z(-FRAC_PI_2),
        ObjectPlacementFace::Left => Quat::from_rotation_z(FRAC_PI_2),
        ObjectPlacementFace::Front => Quat::from_rotation_x(FRAC_PI_2),
        ObjectPlacementFace::Back => Quat::from_rotation_x(-FRAC_PI_2),
    }
}

pub(crate) fn detached_object_drop_request(
    support: IVec3,
    support_cell: Option<VoxelCell>,
    object: ObjectCell,
    objects: &ObjectRegistry,
) -> Option<WorldItemSpawnRequest> {
    let definition = objects.get(object.object_id)?;
    if !definition.drop_self {
        return None;
    }

    let position =
        world_object_position(support, support_cell, object, definition) + Vec3::Y * 0.25;
    Some(WorldItemSpawnRequest::dropped(
        ItemStack::new(object.object_id),
        position,
    ))
}

fn object_position_jitter(
    definition: &ObjectDefinition,
    support: IVec3,
    hollow_orientation: Option<BlockOrientation>,
    instance_scale: Vec3,
) -> Vec3 {
    let seed = mix_u32_components(
        hash_string(&definition.id),
        [support.x as u32, support.y as u32, support.z as u32],
    );
    let mut maximum = Vec2::from_array(definition.position_jitter);
    if let Some(orientation) = hollow_orientation {
        let half = Vec2::new(
            definition.target.size[0] * instance_scale.x,
            definition.target.size[2] * instance_scale.z,
        ) * 0.5;
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

fn resolve_object_loot_item_id(item_id: &str, content: &ObjectLootRegistries<'_>) -> &'static str {
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
