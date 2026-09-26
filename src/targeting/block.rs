use bevy::{ecs::system::SystemParam, prelude::*};

use super::{
    highlight::TargetHighlightPlugin, interaction::BlockInteractionPlugin,
    mining::BlockMiningPlugin, mining_visual::BlockMiningVisualPlugin,
    placement_orientation::PlacementOrientationPlugin,
    placement_preview::PlacementPreviewPlugin,
};
use crate::{
    app::game_state::GameState,
    content::object::ObjectRegistry,
    creatures::{CreatureInstance, CreatureTargetCollider},
    entity::EntityHealth,
    gameplay::availability::WorldInteractionState,
    player::camera::GameplayWorldCamera,
    voxel::{
        coordinates::chunk_coord_from_world,
        raycast::{VoxelHit, raycast_voxels},
        world::VoxelWorld,
    },
    world_items::{TargetedWorldItem, WorldItem, target_bounds},
    world_objects::{TargetedWorldObject, world_object_position},
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

/// Exclusive target: when a creature is in front, no block may be highlighted,
/// edited, previewed or displayed in the block HUD.
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
    creature: ResMut<'w, TargetedCreature>,
    world_item: ResMut<'w, TargetedWorldItem>,
    object: ResMut<'w, TargetedWorldObject>,
}

#[derive(Clone, Copy)]
enum TargetKind {
    Creature(Entity),
    WorldItem(Entity),
    Object(IVec3),
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
    let block_hit = raycast_voxels(&world, origin, direction, TARGET_RANGE);
    let block_distance = block_hit.as_ref().map_or(TARGET_RANGE, |hit| {
        ray_box_distance(
            origin,
            direction,
            hit.voxel.as_vec3(),
            hit.voxel.as_vec3() + Vec3::ONE,
        )
        .unwrap_or(0.0)
    });
    // Re-evaluate scene targets every frame. World objects are voxel-backed;
    // nearest-hit arbitration decides whether the object or the terrain behind
    // it receives the interaction.
    let creature_hits = candidates.creatures.iter().filter_map(
        |(entity, transform, collider, health)| {
            if health.is_dead() {
                return None;
            }
            let (min, max) = collider.0.bounds(transform.translation);
            ray_box_distance(origin, direction, min, max)
                .filter(|distance| *distance <= TARGET_RANGE && *distance <= block_distance)
                .map(|distance| (TargetKind::Creature(entity), distance))
        },
    );
    let world_item_hits = candidates.world_items.iter().filter_map(|(entity, transform)| {
        let (min, max) = target_bounds(transform.translation);
        ray_box_distance(origin, direction, min, max)
            .filter(|distance| *distance <= TARGET_RANGE && *distance <= block_distance)
            .map(|distance| (TargetKind::WorldItem(entity), distance))
    });
    let object_hit =
        closest_world_object_hit(&world, &candidates.objects, origin, direction, block_distance);
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
        Some(TargetKind::Object(support)) => Some(support),
        _ => None,
    };
    let next_block = if closest.is_none() { block_hit } else { None };
    if targets.block.0 != next_block {
        targets.block.0 = next_block;
    }
    if targets.creature.0 != next_creature {
        targets.creature.0 = next_creature;
    }
    if targets.world_item.0 != next_world_item {
        targets.world_item.0 = next_world_item;
    }
    if targets.object.0 != next_object {
        targets.object.0 = next_object;
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
                    let position =
                        world_object_position(support, support_cell, object, definition);
                    let center = position + Vec3::from_array(definition.target.center_offset);
                    let half = Vec3::from_array(definition.target.size) * 0.5;
                    let Some(distance) =
                        ray_box_distance(origin, direction, center - half, center + half)
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
                        closest = Some((TargetKind::Object(support), distance));
                    }
                }
            }
        }
    }

    closest
}

/// Ray versus a static AABB; returns the first forward intersection in blocks.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_selects_near_creature_and_rejects_creature_behind_block() {
        let origin = Vec3::ZERO;
        let ray = Vec3::Z;
        assert_eq!(ray_box_distance(origin, ray, Vec3::new(-1.0, -1.0, 2.0), Vec3::new(1.0, 1.0, 3.0)), Some(2.0));
        assert_eq!(ray_box_distance(origin, ray, Vec3::new(-1.0, -1.0, 4.0), Vec3::new(1.0, 1.0, 5.0)), Some(4.0));
        assert_eq!(ray_box_distance(origin, ray, Vec3::new(2.0, -1.0, 2.0), Vec3::new(3.0, 1.0, 3.0)), None);
    }
}
