use bevy::{ecs::system::SystemParam, prelude::*};

use super::{
    highlight::TargetHighlightPlugin, interaction::BlockInteractionPlugin,
    mining::BlockMiningPlugin, mining_visual::BlockMiningVisualPlugin,
    placement_orientation::PlacementOrientationPlugin,
    placement_preview::PlacementPreviewPlugin,
};
use crate::{
    app::game_state::GameState,
    creatures::{CreatureInstance, CreatureTargetCollider},
    entity::EntityHealth,
    gameplay::availability::WorldInteractionState,
    player::camera::GameplayWorldCamera,
    voxel::{raycast::{VoxelHit, raycast_voxels}, world::VoxelWorld},
    world_items::{InteractPickup, TargetedWorldItem, WorldItem, target_bounds},
};

const TARGET_RANGE: f32 = 8.0;

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

type InteractWorldItemQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform),
    (With<WorldItem>, With<InteractPickup>),
>;

#[derive(SystemParam)]
struct TargetCandidates<'w, 's> {
    creatures: TargetedCreatureQuery<'w, 's>,
    world_items: InteractWorldItemQuery<'w, 's>,
}

#[derive(SystemParam)]
struct TargetSelection<'w> {
    block: ResMut<'w, TargetedBlock>,
    creature: ResMut<'w, TargetedCreature>,
    world_item: ResMut<'w, TargetedWorldItem>,
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
    // Re-evaluate moving creature colliders every frame; camera/voxel caching
    // alone would leave stale targets when only a creature moves.
    let creature_hit = candidates.creatures
        .iter()
        .filter_map(|(entity, transform, collider, health)| {
            if health.is_dead() {
                return None;
            }
            let (min, max) = collider.0.bounds(transform.translation);
            ray_box_distance(origin, direction, min, max)
                .filter(|distance| *distance <= TARGET_RANGE && *distance <= block_distance)
                .map(|distance| (entity, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1));
    let world_item_hit = candidates.world_items
        .iter()
        .filter_map(|(entity, transform)| {
            let (min, max) = target_bounds(transform.translation);
            ray_box_distance(origin, direction, min, max)
                .filter(|distance| *distance <= TARGET_RANGE && *distance <= block_distance)
                .map(|distance| (entity, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1));

    let (next_creature, next_world_item) = match (creature_hit, world_item_hit) {
        (Some((creature, creature_distance)), Some((item, item_distance))) => {
            if creature_distance <= item_distance {
                (Some(creature), None)
            } else {
                (None, Some(item))
            }
        }
        (Some((creature, _)), None) => (Some(creature), None),
        (None, Some((item, _))) => (None, Some(item)),
        (None, None) => (None, None),
    };
    let next_block = if next_creature.is_none() && next_world_item.is_none() {
        block_hit
    } else {
        None
    };
    if targets.block.0 != next_block {
        targets.block.0 = next_block;
    }
    if targets.creature.0 != next_creature {
        targets.creature.0 = next_creature;
    }
    if targets.world_item.0 != next_world_item {
        targets.world_item.0 = next_world_item;
    }
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
