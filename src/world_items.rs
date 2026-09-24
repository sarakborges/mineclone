use bevy::{ecs::system::SystemParam, light::NotShadowCaster, prelude::*};

use crate::{
    app::{
        game_state::GameState,
        keybinds::{KeybindAction, Keybinds},
        pause_state::PauseState,
        resource_systems::reset_resource,
    },
    content::{
        builtin_ids::BIOME_TINT_METADATA_KEY,
        item::ItemRegistry,
        layer::LayerRegistry,
        object::ObjectRegistry,
        tool::ToolRegistry,
    },
    gameplay::availability::world_interaction_available,
    player::{
        PLAYER_EYE_HEIGHT,
        camera::{GameplayCamera, GameplayWorldCamera},
        hotbar::PlayerHotbar,
        item_stack::ItemStack,
        movement::config::{COLLISION_STEP, GRAVITY},
    },
    rendering::{
        block_model::{
            BlockModelMeshes, block_face_material_data, maximum_block_model_layers,
            set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_visual_content::BlockVisualContent,
    },
    targeting::block::BlockTargetingSet,
    voxel::{
        block_face::BlockFace,
        collision::aabb_is_clear,
        world::VoxelWorld,
    },
};

const ITEM_HALF_EXTENT: f32 = 0.18;
const ITEM_SPRITE_SIZE: f32 = 0.46;
const BLOCK_ITEM_SCALE: f32 = 0.36;
const DROP_FORWARD_SPEED: f32 = 3.8;
const DROP_UP_SPEED: f32 = 1.25;
const DROP_SPAWN_DISTANCE: f32 = 0.9;
const DROP_PICKUP_DELAY_SECONDS: f32 = 0.65;
const PROXIMITY_PICKUP_RADIUS: f32 = 1.65;
const VISUAL_SPIN_SPEED: f32 = 0.9;

#[derive(Component)]
pub(crate) struct WorldItem {
    stack: ItemStack,
}

impl WorldItem {
    fn new(stack: ItemStack) -> Self {
        Self { stack }
    }

    fn stack(&self) -> &ItemStack {
        &self.stack
    }
}

#[derive(Component)]
pub(crate) struct InteractPickup;

#[derive(Component)]
struct ProximityPickup {
    delay_seconds: f32,
}

#[derive(Component)]
struct WorldItemMotion {
    velocity: Vec3,
}

#[derive(Component)]
struct WorldItemVisual;

#[derive(Resource, Default)]
pub(crate) struct TargetedWorldItem(pub(crate) Option<Entity>);

#[derive(Clone, Copy, Default)]
pub(crate) enum WorldItemPickup {
    #[default]
    Interact,
    Proximity,
}

#[derive(Message)]
pub(crate) struct WorldItemSpawnRequest {
    pub(crate) stack: ItemStack,
    pub(crate) position: Vec3,
    pub(crate) velocity: Vec3,
    pub(crate) pickup: WorldItemPickup,
}

impl WorldItemSpawnRequest {
    pub(crate) fn dropped(stack: ItemStack, position: Vec3) -> Self {
        Self {
            stack,
            position,
            velocity: Vec3::ZERO,
            pickup: WorldItemPickup::Proximity,
        }
    }

    fn thrown(stack: ItemStack, position: Vec3, velocity: Vec3) -> Self {
        Self {
            stack,
            position,
            velocity,
            pickup: WorldItemPickup::Proximity,
        }
    }
}

#[derive(Message)]
pub(crate) struct PlayerDropRequest {
    stack: ItemStack,
}

impl PlayerDropRequest {
    pub(crate) fn new(stack: ItemStack) -> Self {
        Self { stack }
    }
}

#[derive(Resource)]
struct WorldItemVisualAssets {
    sprite_mesh: Handle<Mesh>,
    fallback_mesh: Handle<Mesh>,
}

pub(crate) struct WorldItemsPlugin;

impl Plugin for WorldItemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedWorldItem>()
            .add_message::<WorldItemSpawnRequest>()
            .add_message::<PlayerDropRequest>()
            .add_systems(Startup, setup_world_item_visual_assets)
            .add_systems(
                OnExit(GameState::Gameplay),
                reset_resource::<TargetedWorldItem>,
            )
            .add_systems(
                Update,
                drop_selected_item.run_if(world_interaction_available),
            )
            .add_systems(
                Update,
                (move_world_items, pickup_proximity_items, animate_world_item_visuals).chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                Update,
                pickup_interact_item
                    .after(BlockTargetingSet::Raycast)
                    .before(BlockTargetingSet::Interaction)
                    .run_if(world_interaction_available),
            )
            .add_systems(
                PostUpdate,
                (resolve_player_drop_requests, spawn_world_items)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn setup_world_item_visual_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(WorldItemVisualAssets {
        sprite_mesh: meshes.add(Rectangle::new(ITEM_SPRITE_SIZE, ITEM_SPRITE_SIZE)),
        fallback_mesh: meshes.add(Cuboid::new(
            ITEM_HALF_EXTENT * 2.0,
            ITEM_HALF_EXTENT * 2.0,
            ITEM_HALF_EXTENT * 2.0,
        )),
    });
}

fn drop_selected_item(
    keys: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut hotbar: ResMut<PlayerHotbar>,
    mut drops: MessageWriter<PlayerDropRequest>,
) {
    if !keys.just_pressed(keybinds.key_code(KeybindAction::DropItem)) {
        return;
    }
    if let Some(stack) = hotbar.take_selected_stack() {
        drops.write(PlayerDropRequest::new(stack));
    }
}

fn resolve_player_drop_requests(
    camera: Single<&GlobalTransform, With<GameplayWorldCamera>>,
    mut requests: MessageReader<PlayerDropRequest>,
    mut spawns: MessageWriter<WorldItemSpawnRequest>,
) {
    let forward = camera.forward().as_vec3();
    let position = camera.translation() + forward * DROP_SPAWN_DISTANCE - Vec3::Y * 0.25;
    let velocity = forward * DROP_FORWARD_SPEED + Vec3::Y * DROP_UP_SPEED;
    for request in requests.read() {
        spawns.write(WorldItemSpawnRequest::thrown(
            request.stack.clone(),
            position,
            velocity,
        ));
    }
}

#[derive(SystemParam)]
struct WorldItemSpawnContent<'w> {
    visual_assets: Res<'w, WorldItemVisualAssets>,
    block_meshes: Res<'w, BlockModelMeshes>,
    block_content: BlockVisualContent<'w>,
    items: Res<'w, ItemRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
}

#[derive(SystemParam)]
struct WorldItemSpawnAssets<'w> {
    standard_materials: ResMut<'w, Assets<StandardMaterial>>,
    block_materials: ResMut<'w, Assets<BlockModelMaterial>>,
}

fn spawn_world_items(
    mut commands: Commands,
    mut requests: MessageReader<WorldItemSpawnRequest>,
    content: WorldItemSpawnContent,
    mut assets: WorldItemSpawnAssets,
) {
    for request in requests.read() {
        let mut entity = commands.spawn((
            WorldItem::new(request.stack.clone()),
            WorldItemMotion {
                velocity: request.velocity,
            },
            Transform::from_translation(request.position),
            Visibility::default(),
            DespawnOnExit(GameState::Gameplay),
            Name::new(format!("World Item ({})", request.stack.id())),
        ));

        match request.pickup {
            WorldItemPickup::Interact => {
                entity.insert(InteractPickup);
            }
            WorldItemPickup::Proximity => {
                entity.insert(ProximityPickup {
                    delay_seconds: DROP_PICKUP_DELAY_SECONDS,
                });
            }
        }

        entity.with_children(|root| {
            spawn_world_item_visual(
                root,
                request,
                WorldItemVisualContent {
                    visual_assets: &content.visual_assets,
                    block_meshes: &content.block_meshes,
                    block_content: &content.block_content,
                    items: &content.items,
                    layers: &content.layers,
                    objects: &content.objects,
                    tools: &content.tools,
                },
                WorldItemVisualMaterialAssets {
                    standard: &mut assets.standard_materials,
                    block: &mut assets.block_materials,
                },
            );
        });
    }
}

struct WorldItemVisualContent<'a, 'w> {
    visual_assets: &'a WorldItemVisualAssets,
    block_meshes: &'a BlockModelMeshes,
    block_content: &'a BlockVisualContent<'w>,
    items: &'a ItemRegistry,
    layers: &'a LayerRegistry,
    objects: &'a ObjectRegistry,
    tools: &'a ToolRegistry,
}

struct WorldItemVisualMaterialAssets<'a> {
    standard: &'a mut Assets<StandardMaterial>,
    block: &'a mut Assets<BlockModelMaterial>,
}

fn spawn_world_item_visual(
    root: &mut ChildSpawnerCommands,
    request: &WorldItemSpawnRequest,
    content: WorldItemVisualContent<'_, '_>,
    assets: WorldItemVisualMaterialAssets<'_>,
) {
    let item_id = request.stack.id();
    if let Some(block) = content.block_content.blocks.get(item_id) {
        let horizontal = Vec2::new(request.position.x, request.position.z);
        let tint = content.block_content
            .tint_at_with_override(
                item_id,
                horizontal,
                request.stack.metadata().get(BIOME_TINT_METADATA_KEY),
            )
            .unwrap_or(Color::WHITE);

        root.spawn((
            WorldItemVisual,
            Transform::from_scale(Vec3::splat(BLOCK_ITEM_SCALE)),
            Visibility::default(),
        ))
        .with_children(|model| {
            for face in BlockFace::ALL {
                let layer_count = maximum_block_model_layers(&content.block_content.blocks, face);
                for layer_index in 0..layer_count {
                    let Some(mut material) = block_face_material_data(
                        face,
                        layer_index,
                        block,
                        &content.block_content.asset_server,
                        1.0,
                    ) else {
                        continue;
                    };
                    set_block_model_tint(&mut material, tint);
                    model.spawn((
                        Mesh3d(content.block_meshes.world_face(face)),
                        MeshMaterial3d(assets.block.add(material)),
                        NotShadowCaster,
                    ));
                }
            }
        });
        return;
    }

    let icon = content.items
        .get(item_id)
        .map(|definition| definition.icon.as_str())
        .or_else(|| content.layers.get(item_id).map(|definition| definition.texture.as_str()))
        .or_else(|| content.objects.get(item_id).map(|definition| definition.icon.as_str()))
        .or_else(|| {
            content.tools
                .get(item_id)
                .and_then(|definition| (!definition.icon.is_empty()).then_some(definition.icon.as_str()))
        });

    if let Some(icon) = icon {
        let material = assets.standard.add(StandardMaterial {
            base_color_texture: Some(content.block_content.asset_server.load(icon.to_owned())),
            alpha_mode: AlphaMode::Mask(0.5),
            perceptual_roughness: 1.0,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        root.spawn((WorldItemVisual, Transform::default(), Visibility::default()))
            .with_children(|visual| {
                visual.spawn((
                    Mesh3d(content.visual_assets.sprite_mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
                    NotShadowCaster,
                ));
            });
        return;
    }

    let material = assets.standard.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.65, 0.65),
        perceptual_roughness: 1.0,
        unlit: true,
        ..default()
    });
    root.spawn((
        WorldItemVisual,
        Mesh3d(content.visual_assets.fallback_mesh.clone()),
        MeshMaterial3d(material),
        Transform::default(),
        NotShadowCaster,
    ));
}

fn move_world_items(
    time: Res<Time>,
    world: Res<VoxelWorld>,
    mut commands: Commands,
    mut items: Query<(Entity, &mut Transform, &mut WorldItemMotion)>,
) {
    let dt = time.delta_secs().min(0.05);
    for (entity, mut transform, mut motion) in &mut items {
        if !world.is_loaded_at(transform.translation.floor().as_ivec3()) {
            continue;
        }

        motion.velocity.y += GRAVITY * dt;
        let mut collided = false;
        for axis in [0, 2, 1] {
            let distance = motion.velocity[axis] * dt;
            if distance == 0.0 {
                continue;
            }
            if !advance_item_axis(&world, &mut transform.translation, axis, distance) {
                collided = true;
                break;
            }
        }

        if collided {
            commands.entity(entity).remove::<WorldItemMotion>();
        }
    }
}

fn advance_item_axis(
    world: &VoxelWorld,
    center: &mut Vec3,
    axis: usize,
    distance: f32,
) -> bool {
    let steps = (distance.abs() / COLLISION_STEP).ceil().max(1.0) as usize;
    let step = distance / steps as f32;

    for _ in 0..steps {
        let mut next = *center;
        next[axis] += step;
        let (min, max) = target_bounds(next);
        if !aabb_is_clear(world, (min, max)) {
            return false;
        }
        *center = next;
    }
    true
}


fn pickup_proximity_items(
    time: Res<Time>,
    player: Single<&GlobalTransform, With<GameplayCamera>>,
    mut hotbar: ResMut<PlayerHotbar>,
    mut commands: Commands,
    mut items: Query<(Entity, &Transform, &mut WorldItem, &mut ProximityPickup)>,
) {
    let pickup_center = player.translation() - Vec3::Y * (PLAYER_EYE_HEIGHT * 0.5);
    let radius_squared = PROXIMITY_PICKUP_RADIUS * PROXIMITY_PICKUP_RADIUS;

    for (entity, transform, mut world_item, mut pickup) in &mut items {
        if pickup.delay_seconds > 0.0 {
            pickup.delay_seconds = (pickup.delay_seconds - time.delta_secs()).max(0.0);
            continue;
        }
        if transform.translation.distance_squared(pickup_center) > radius_squared {
            continue;
        }
        if collect_world_item(&mut hotbar, &mut world_item) {
            commands.entity(entity).despawn();
        }
    }
}

fn pickup_interact_item(
    buttons: Res<ButtonInput<MouseButton>>,
    mut targeted: ResMut<TargetedWorldItem>,
    mut hotbar: ResMut<PlayerHotbar>,
    mut commands: Commands,
    mut items: Query<&mut WorldItem>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(entity) = targeted.0 else {
        return;
    };
    let Ok(mut world_item) = items.get_mut(entity) else {
        targeted.0 = None;
        return;
    };
    if collect_world_item(&mut hotbar, &mut world_item) {
        commands.entity(entity).despawn();
        targeted.0 = None;
    }
}

fn collect_world_item(hotbar: &mut PlayerHotbar, world_item: &mut WorldItem) -> bool {
    match hotbar.try_insert_stack(world_item.stack().clone()) {
        Ok(()) => true,
        Err(remaining) => {
            world_item.stack = remaining;
            false
        }
    }
}

fn animate_world_item_visuals(
    time: Res<Time>,
    mut visuals: Query<&mut Transform, With<WorldItemVisual>>,
) {
    let angle = VISUAL_SPIN_SPEED * time.delta_secs();
    for mut transform in &mut visuals {
        transform.rotate_y(angle);
    }
}

pub(crate) fn target_bounds(center: Vec3) -> (Vec3, Vec3) {
    let half = Vec3::splat(ITEM_HALF_EXTENT);
    (center - half, center + half)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_world_items_default_to_interact_pickup() {
        assert!(matches!(WorldItemPickup::default(), WorldItemPickup::Interact));
    }

    #[test]
    fn target_bounds_are_centered_on_world_item() {
        let center = Vec3::new(3.0, 4.0, 5.0);
        let (min, max) = target_bounds(center);
        assert_eq!((min + max) * 0.5, center);
        assert_eq!(max - min, Vec3::splat(ITEM_HALF_EXTENT * 2.0));
    }
}
