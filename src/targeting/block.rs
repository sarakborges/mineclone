use bevy::{ecs::system::SystemParam, prelude::*};

use super::{
    highlight::TargetHighlightPlugin, interaction::BlockInteractionPlugin,
    mining::BlockMiningPlugin, mining_visual::BlockMiningVisualPlugin,
    placement_orientation::PlacementOrientationPlugin,
    placement_preview::PlacementPreviewPlugin,
};
use crate::{
    app::{crash_log::log_gameplay_event, game_state::GameState},
    content::object::{ObjectDefinition, ObjectRegistry},
    creatures::{CreatureInstance, CreatureTargetCollider},
    entity::EntityHealth,
    gameplay::availability::WorldInteractionState,
    player::camera::GameplayWorldCamera,
    voxel::{
        coordinates::chunk_coord_from_world,
        fluid::FluidCell,
        raycast::{VoxelHit, raycast_voxels},
        read::VoxelRead,
        world::VoxelWorld,
    },
    world_items::{TargetedWorldItem, WorldItem, target_bounds},
    world_objects::{TargetedWorldObject, WorldObjectKey, world_object_transform},
};

const TARGET_RANGE: f32 = 8.0;
const TARGET_OBJECT_CHUNK_MARGIN: i32 = 1;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum BlockTargetingSet {
    Raycast,
    PlacementState,
    Interaction,
    Visuals,
}

pub struct BlockTargetingPlugin;

impl Plugin for BlockTargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetedBlock>()
            .init_resource::<TargetedFluid>()
            .init_resource::<TargetedCreature>()
            .configure_sets(
                Update,
                (
                    BlockTargetingSet::Raycast,
                    BlockTargetingSet::PlacementState,
                    BlockTargetingSet::Interaction,
                    BlockTargetingSet::Visuals,
                )
                    .chain(),
            )
            .add_plugins((
                TargetHighlightPlugin,
                PlacementOrientationPlugin,
                BlockInteractionPlugin,
                BlockMiningPlugin,
                BlockMiningVisualPlugin,
                PlacementPreviewPlugin,
            ))
            .add_systems(
                Update,
                update_targets
                    .in_set(BlockTargetingSet::Raycast)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Resource, Default)]
pub struct TargetedBlock(pub Option<VoxelHit>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluidHit {
    pub voxel: IVec3,
    pub fluid: FluidCell,
    pub normal: IVec3,
}

#[derive(Resource, Default)]
pub struct TargetedFluid(pub Option<FluidHit>);

#[derive(Resource, Default)]
pub(crate) struct TargetedCreature(pub Option<Entity>);

type TargetedCreatureQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static CreatureTargetCollider,
        &'static EntityHealth,
    ),
    With<CreatureInstance>,
>;

type InteractWorldItemQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform), With<WorldItem>>;

#[derive(SystemParam)]
struct TargetCandidates<'w, 's> {
    creatures: TargetedCreatureQuery<'w, 's>,
    world_items: InteractWorldItemQuery<'w, 's>,
    objects: Res<'w, ObjectRegistry>,
}

#[derive(SystemParam)]
struct TargetSelection<'w> {
    block: ResMut<'w, TargetedBlock>,
    fluid: ResMut<'w, TargetedFluid>,
    creature: ResMut<'w, TargetedCreature>,
    world_item: ResMut<'w, TargetedWorldItem>,
    object: ResMut<'w, TargetedWorldObject>,
}

#[derive(Clone, Copy)]
enum TargetKind {
    Creature(Entity),
    WorldItem(Entity),
    Object(WorldObjectKey),
}

fn update_targets(
    camera: Single<&GlobalTransform, With<GameplayWorldCamera>>,
    world: Res<VoxelWorld>,
    interaction: WorldInteractionState,
    candidates: TargetCandidates,
    mut targets: TargetSelection,
) {
    if !interaction.available() {
        if targets.block.0.is_some() {
            targets.block.0 = None;
        }
        if targets.fluid.0.is_some() {
            targets.fluid.0 = None;
        }
        if targets.creature.0.is_some() {
            targets.creature.0 = None;
        }
        if targets.world_item.0.is_some() {
            targets.world_item.0 = None;
        }
        if targets.object.0.is_some() {
            targets.object.0 = None;
        }
        return;
    }

    let origin = camera.translation();
    let direction = camera.forward().as_vec3();
    let block_hit = raycast_voxels(&*world, origin, direction, TARGET_RANGE);
    let block_distance = block_hit.as_ref().map_or(TARGET_RANGE, |hit| {
        ray_box_distance(
            origin,
            direction,
            hit.voxel.as_vec3(),
            hit.voxel.as_vec3() + Vec3::ONE,
        )
        .unwrap_or(0.0)
    });
    let fluid_hit = raycast_fluid_source(&*world, origin, direction, TARGET_RANGE);
    let fluid_distance = fluid_hit.as_ref().map_or(TARGET_RANGE, |(_, distance)| *distance);
    let world_distance = block_distance.min(fluid_distance);

    let creature_hits = candidates.creatures.iter().filter_map(
        |(entity, transform, collider, health)| {
            if health.is_dead() {
                return None;
            }
            let (min, max) = collider.0.bounds(transform.translation);
            ray_box_distance(origin, direction, min, max)
                .filter(|distance| *distance <= TARGET_RANGE && *distance <= world_distance)
                .map(|distance| (TargetKind::Creature(entity), distance))
        },
    );
    let world_item_hits = candidates.world_items.iter().filter_map(|(entity, transform)| {
        let (min, max) = target_bounds(transform.translation);
        ray_box_distance(origin, direction, min, max)
            .filter(|distance| *distance <= TARGET_RANGE && *distance <= world_distance)
            .map(|distance| (TargetKind::WorldItem(entity), distance))
    });
    let object_hit =
        closest_world_object_hit(&world, &candidates.objects, origin, direction, world_distance);
    let closest = creature_hits
        .chain(world_item_hits)
        .chain(object_hit)
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(target, _)| target);

    let next_creature = match closest {
        Some(TargetKind::Creature(entity)) => Some(entity),
        _ => None,
    };
    let next_world_item = match closest {
        Some(TargetKind::WorldItem(entity)) => Some(entity),
        _ => None,
    };
    let next_object = match closest {
        Some(TargetKind::Object(key)) => Some(key),
        _ => None,
    };
    let (next_block, next_fluid) = if closest.is_some() {
        (None, None)
    } else if let Some((hit, distance)) = fluid_hit
        && distance < block_distance
    {
        (None, Some(hit))
    } else {
        (block_hit, None)
    };

    if targets.block.0 != next_block {
        log_gameplay_event(format!(
            "target.block from={:?} to={:?}",
            targets.block.0.map(|hit| (hit.voxel, hit.block_id)),
            next_block.map(|hit| (hit.voxel, hit.block_id))
        ));
        targets.block.0 = next_block;
    }
    if targets.fluid.0 != next_fluid {
        log_gameplay_event(format!(
            "target.fluid from={:?} to={:?}",
            targets
                .fluid
                .0
                .map(|hit| (hit.voxel, hit.fluid.fluid_id)),
            next_fluid.map(|hit| (hit.voxel, hit.fluid.fluid_id))
        ));
        targets.fluid.0 = next_fluid;
    }
    if targets.creature.0 != next_creature {
        log_gameplay_event(format!(
            "target.entity from={:?} to={:?}",
            targets.creature.0, next_creature
        ));
        targets.creature.0 = next_creature;
    }
    if targets.world_item.0 != next_world_item {
        log_gameplay_event(format!(
            "target.item from={:?} to={:?}",
            targets.world_item.0, next_world_item
        ));
        targets.world_item.0 = next_world_item;
    }
    if targets.object.0 != next_object {
        log_gameplay_event(format!(
            "target.object from={:?} to={:?}",
            targets.object.0, next_object
        ));
        targets.object.0 = next_object;
    }
}

fn raycast_fluid_source(
    world: &impl VoxelRead,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<(FluidHit, f32)> {
    if direction.length_squared() == 0.0 || max_distance < 0.0 {
        return None;
    }

    let direction = direction.normalize();
    let mut voxel = origin.floor().as_ivec3();
    let step = IVec3::new(
        direction.x.signum() as i32,
        direction.y.signum() as i32,
        direction.z.signum() as i32,
    );
    let t_delta = Vec3::new(
        reciprocal_abs(direction.x),
        reciprocal_abs(direction.y),
        reciprocal_abs(direction.z),
    );
    let mut t_max = Vec3::new(
        first_boundary_distance(origin.x, voxel.x, direction.x),
        first_boundary_distance(origin.y, voxel.y, direction.y),
        first_boundary_distance(origin.z, voxel.z, direction.z),
    );
    let mut entry_normal = IVec3::ZERO;

    loop {
        let distance = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            let distance = t_max.x;
            voxel.x += step.x;
            entry_normal = IVec3::new(-step.x, 0, 0);
            t_max.x += t_delta.x;
            distance
        } else if t_max.y <= t_max.z {
            let distance = t_max.y;
            voxel.y += step.y;
            entry_normal = IVec3::new(0, -step.y, 0);
            t_max.y += t_delta.y;
            distance
        } else {
            let distance = t_max.z;
            voxel.z += step.z;
            entry_normal = IVec3::new(0, 0, -step.z);
            t_max.z += t_delta.z;
            distance
        };

        if distance > max_distance {
            return None;
        }
        if world.cell_at(voxel).is_some() {
            return None;
        }
        if let Some(fluid) = world.fluid_at(voxel) {
            return fluid.is_source().then_some((
                FluidHit {
                    voxel,
                    fluid,
                    normal: entry_normal,
                },
                distance,
            ));
        }
    }
}

fn closest_world_object_hit(
    world: &VoxelWorld,
    objects: &ObjectRegistry,
    origin: Vec3,
    direction: Vec3,
    block_distance: f32,
) -> Option<(TargetKind, f32)> {
    let end = origin + direction * TARGET_RANGE;
    let start_chunk = chunk_coord_from_world(origin.floor().as_ivec3());
    let end_chunk = chunk_coord_from_world(end.floor().as_ivec3());
    let minimum = start_chunk.min(end_chunk) - IVec3::splat(TARGET_OBJECT_CHUNK_MARGIN);
    let maximum = start_chunk.max(end_chunk) + IVec3::splat(TARGET_OBJECT_CHUNK_MARGIN);
    let mut closest = None;

    for y in minimum.y.max(0)..=maximum.y {
        for z in minimum.z..=maximum.z {
            for x in minimum.x..=maximum.x {
                let coord = IVec3::new(x, y, z);
                let Some(chunk) = world.chunk(coord) else {
                    continue;
                };
                let chunk_origin = coord * crate::voxel::chunk::CHUNK_SIZE as i32;
                for (local_x, local_y, local_z, object) in chunk.object_voxels() {
                    let support = chunk_origin
                        + IVec3::new(local_x as i32, local_y as i32, local_z as i32);
                    let Some(definition) = objects.get(object.object_id) else {
                        continue;
                    };
                    let support_cell =
                        chunk.cell_at(local_x as i32, local_y as i32, local_z as i32);
                    let transform =
                        world_object_transform(support, support_cell, object, definition);
                    let (minimum, maximum) = object_target_bounds(&transform, definition);
                    let Some(distance) = ray_box_distance(origin, direction, minimum, maximum)
                        .filter(|distance| {
                            *distance <= TARGET_RANGE && *distance <= block_distance
                        })
                    else {
                        continue;
                    };
                    if closest
                        .as_ref()
                        .is_none_or(|(_, best_distance)| distance < *best_distance)
                    {
                        closest = Some((
                            TargetKind::Object(WorldObjectKey::new(support, object)),
                            distance,
                        ));
                    }
                }
            }
        }
    }

    closest
}

fn object_target_bounds(
    transform: &Transform,
    definition: &ObjectDefinition,
) -> (Vec3, Vec3) {
    let center = Vec3::from_array(definition.target.center_offset);
    let half = Vec3::from_array(definition.target.size) * 0.5;
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);

    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                let local = center + half * Vec3::new(x, y, z);
                let world = transform.translation + transform.rotation * (local * transform.scale);
                minimum = minimum.min(world);
                maximum = maximum.max(world);
            }
        }
    }

    (minimum, maximum)
}

fn ray_box_distance(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut entry = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;
    for axis in 0..3 {
        if direction[axis].abs() <= f32::EPSILON {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let near = (min[axis] - origin[axis]) / direction[axis];
        let far = (max[axis] - origin[axis]) / direction[axis];
        entry = entry.max(near.min(far));
        exit = exit.min(near.max(far));
        if exit < entry {
            return None;
        }
    }
    (exit >= 0.0).then_some(entry.max(0.0))
}

fn reciprocal_abs(value: f32) -> f32 {
    if value == 0.0 {
        f32::INFINITY
    } else {
        1.0 / value.abs()
    }
}

fn first_boundary_distance(origin: f32, voxel: i32, direction: f32) -> f32 {
    if direction > 0.0 {
        (voxel as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (origin - voxel as f32) / -direction
    } else {
        f32::INFINITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_selects_near_creature_and_rejects_creature_behind_block() {
        let origin = Vec3::ZERO;
        let ray = Vec3::Z;
        assert_eq!(
            ray_box_distance(
                origin,
                ray,
                Vec3::new(-1.0, -1.0, 2.0),
                Vec3::new(1.0, 1.0, 3.0)
            ),
            Some(2.0)
        );
        assert_eq!(
            ray_box_distance(
                origin,
                ray,
                Vec3::new(-1.0, -1.0, 4.0),
                Vec3::new(1.0, 1.0, 5.0)
            ),
            Some(4.0)
        );
        assert_eq!(
            ray_box_distance(
                origin,
                ray,
                Vec3::new(2.0, -1.0, 2.0),
                Vec3::new(3.0, 1.0, 3.0)
            ),
            None
        );
    }
}
