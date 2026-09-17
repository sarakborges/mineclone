use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        block::BlockRegistry,
        creature::{CreatureCollider, CreatureRegistry},
        structure::StructureRegistry,
    },
    creatures::{CreatureInstance, spawn_creature_at},
    localization::ActiveLanguage,
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, camera::GameplayCamera},
    voxel::{
        cell::VoxelCell, collision::collides_aabb, edit::VoxelTopologyRuntime,
        texture_rotation::TextureRotation, world::VoxelWorld,
    },
};

const DISPLACEMENT_MARGIN: i32 = 3;
const DISPLACEMENT_HEIGHTS: [i32; 5] = [0, 1, -1, 2, -2];

type ExistingCreatures<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, &'static CreatureCollider),
    (With<CreatureInstance>, Without<GameplayCamera>),
>;

#[derive(SystemParam)]
pub(super) struct ChatPlacementContext<'w, 's> {
    definitions: Res<'w, CreatureRegistry>,
    structures: Res<'w, StructureRegistry>,
    blocks: Res<'w, BlockRegistry>,
    assets: Res<'w, AssetServer>,
    language: Res<'w, ActiveLanguage>,
    runtime: VoxelTopologyRuntime<'w>,
    player: Query<'w, 's, &'static mut Transform, (With<GameplayCamera>, Without<CreatureInstance>)>,
    existing: ExistingCreatures<'w, 's>,
}

fn overlaps(left: (Vec3, Vec3), right: (Vec3, Vec3)) -> bool {
    left.0.x < right.1.x && left.1.x > right.0.x
        && left.0.y < right.1.y && left.1.y > right.0.y
        && left.0.z < right.1.z && left.1.z > right.0.z
}

fn player_bounds(eye: Vec3) -> (Vec3, Vec3) {
    let feet_y = eye.y - PLAYER_EYE_HEIGHT;
    (
        Vec3::new(eye.x - PLAYER_HALF_WIDTH, feet_y, eye.z - PLAYER_HALF_WIDTH),
        Vec3::new(eye.x + PLAYER_HALF_WIDTH, feet_y + PLAYER_HEIGHT, eye.z + PLAYER_HALF_WIDTH),
    )
}

fn clear_destination(
    world: &VoxelWorld,
    eye: Vec3,
    blocked: (Vec3, Vec3),
    existing: &ExistingCreatures<'_, '_>,
    reserved: &[(Vec3, CreatureCollider)],
) -> bool {
    let bounds = player_bounds(eye);
    if overlaps(bounds, blocked)
        || existing
            .iter()
            .any(|(transform, collider)| overlaps(bounds, collider.bounds(transform.translation)))
        || reserved
            .iter()
            .any(|(feet, collider)| overlaps(bounds, collider.bounds(*feet)))
    {
        return false;
    }

    let minimum = bounds.0.floor().as_ivec3();
    let maximum = (bounds.1 - Vec3::splat(0.0001)).floor().as_ivec3();
    for y in minimum.y..=maximum.y {
        for z in minimum.z..=maximum.z {
            for x in minimum.x..=maximum.x {
                let voxel = IVec3::new(x, y, z);
                if !world.is_loaded_at(voxel)
                    || world.is_solid(voxel)
                    || world.fluid_at(voxel).is_some()
                {
                    return false;
                }
            }
        }
    }
    let below = eye - Vec3::Y * 0.08;
    let support = player_bounds(below);
    world.is_loaded_at(support.0.floor().as_ivec3())
        && world.is_loaded_at(support.1.floor().as_ivec3())
        && collides_aabb(world, support.0, support.1)
}

/// Search the perimeter outside the *entire* occupied volume. No edits happen
/// until a dry, loaded and supported destination for the whole player is found.
fn displaced_eye(
    world: &VoxelWorld,
    initial_eye: Vec3,
    blocked: (Vec3, Vec3),
    existing: &ExistingCreatures<'_, '_>,
    reserved: &[(Vec3, CreatureCollider)],
) -> Option<Vec3> {
    let min = blocked.0.floor().as_ivec3() - IVec3::splat(DISPLACEMENT_MARGIN);
    let max = blocked.1.ceil().as_ivec3() + IVec3::splat(DISPLACEMENT_MARGIN);
    let original_feet_y = (initial_eye.y - PLAYER_EYE_HEIGHT).floor() as i32;
    let mut candidates = Vec::new();
    for x in min.x..=max.x {
        for z in min.z..=max.z {
            for delta_y in DISPLACEMENT_HEIGHTS {
                let eye = Vec3::new(
                    x as f32 + 0.5,
                    (original_feet_y + delta_y) as f32 + PLAYER_EYE_HEIGHT,
                    z as f32 + 0.5,
                );
                if !overlaps(player_bounds(eye), blocked) {
                    candidates.push(eye);
                }
            }
        }
    }
    candidates.sort_by(|a, b| {
        a.distance_squared(initial_eye)
            .total_cmp(&b.distance_squared(initial_eye))
    });
    candidates.into_iter().find(|eye| {
        clear_destination(world, *eye, blocked, existing, reserved)
    })
}

impl ChatPlacementContext<'_, '_> {
    pub(super) fn spawn(
        &mut self,
        commands: &mut Commands,
        id: &str,
        reserved: &mut Vec<(Vec3, CreatureCollider)>,
    ) -> String {
        let Some(definition) = self.definitions.get(id) else {
            return format!("Unknown creature id: {id}");
        };
        let Ok(mut player) = self.player.single_mut() else {
            return "Cannot spawn creature: player is unavailable.".to_owned();
        };
        let feet = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
        let blocked = definition.collider.bounds(feet);
        let occupied = collides_aabb(self.runtime.world(), blocked.0, blocked.1)
            || self.existing.iter().any(|(other, collider)| {
                overlaps(blocked, collider.bounds(other.translation))
            })
            || reserved.iter().any(|(other, collider)| {
                overlaps(blocked, collider.bounds(*other))
            });
        if occupied || !self.runtime.world().is_loaded_at(blocked.0.floor().as_ivec3())
            || !self.runtime.world().is_loaded_at(blocked.1.floor().as_ivec3())
        {
            return format!("not enough space to spawn {id}");
        }
        let Some(destination) = displaced_eye(
            self.runtime.world(), player.translation, blocked, &self.existing, reserved,
        ) else {
            return format!("not enough space to spawn {id}");
        };
        match spawn_creature_at(
            commands,
            &self.definitions,
            &self.assets,
            self.language.get(),
            id,
            feet,
        ) {
            Ok(name) => {
                player.translation = destination;
                reserved.push((feet, definition.collider));
                format!("Spawned {name} ({id}).")
            }
            Err(error) => error,
        }
    }

    pub(super) fn place(
        &mut self,
        id: &str,
        reserved: &[(Vec3, CreatureCollider)],
    ) -> String {
        let Some(structure) = self.structures.get(id) else {
            return format!("Unknown structure id: {id}");
        };
        let Ok(mut player) = self.player.single_mut() else {
            return "Cannot place structure: player is unavailable.".to_owned();
        };
        let origin = (player.translation - Vec3::Y * PLAYER_EYE_HEIGHT)
            .floor()
            .as_ivec3();
        let voxels = structure.voxels();
        if voxels.is_empty() {
            return format!("not enough space to place {id}");
        }
        let min = voxels
            .iter()
            .fold(IVec3::splat(i32::MAX), |min, voxel| min.min(origin + voxel.offset));
        let max = voxels
            .iter()
            .fold(IVec3::splat(i32::MIN), |max, voxel| max.max(origin + voxel.offset));
        let blocked = (min.as_vec3(), (max + IVec3::ONE).as_vec3());
        let world = self.runtime.world();
        let all_loaded_and_clear = voxels.iter().all(|voxel| {
            let position = origin + voxel.offset;
            let voxel_bounds = (position.as_vec3(), position.as_vec3() + Vec3::ONE);
            world.is_loaded_at(position)
                && !self.existing.iter().any(|(other, collider)| {
                    overlaps(voxel_bounds, collider.bounds(other.translation))
                })
                && !reserved.iter().any(|(other, collider)| {
                    overlaps(voxel_bounds, collider.bounds(*other))
                })
        });
        if !all_loaded_and_clear {
            return format!("not enough space to place {id}");
        }
        let Some(destination) = displaced_eye(
            world, player.translation, blocked, &self.existing, reserved,
        ) else {
            return format!("not enough space to place {id}");
        };

        // Preflight completed: all structure voxels and the player's new position
        // were verified before the first mutation. The topology runtime updates
        // lighting, fluid frontiers and remesh queues exactly like block placement.
        for voxel in voxels {
            let position = origin + voxel.offset;
            let block = self.blocks.get(voxel.block_id).expect("validated structure block");
            let rotation = TextureRotation::for_position(position, block.rotate_texture.any());
            let cell = VoxelCell::oriented(voxel.block_id, rotation, voxel.orientation);
            self.runtime.set_block(position, Some(cell));
        }
        player.translation = destination;
        format!("Placed {} ({id}).", structure.name.text(self.language.get()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_overlap_requires_positive_intersection() {
        assert!(!overlaps((Vec3::ZERO, Vec3::ONE), (Vec3::X, Vec3::X + Vec3::ONE)));
        assert!(overlaps((Vec3::ZERO, Vec3::ONE), (Vec3::splat(0.5), Vec3::splat(1.5))));
    }
}
