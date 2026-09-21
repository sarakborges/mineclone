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
    world::{generation::surface_layer_placements, WorldSeed},
};

const DISPLACEMENT_MARGIN: i32 = 3;
const DISPLACEMENT_HEIGHTS: [i32; 5] = [0, 1, -1, 2, -2];
const SUPPORT_PROBE: f32 = 0.08;
const BOUNDS_EPSILON: f32 = 0.0001;

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
    seed: Res<'w, WorldSeed>,
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

/// A collider occupies only the voxels it actually intersects, not the
/// adjacent voxel when a maximum bound lies exactly on a grid boundary.
/// Every intersected voxel must be loaded and empty; player relocation also
/// requires dry space, whereas creature spawn may take place in fluids.
fn clear_volume(world: &VoxelWorld, bounds: (Vec3, Vec3), require_dry: bool) -> bool {
    let minimum = (bounds.0 + Vec3::splat(BOUNDS_EPSILON)).floor().as_ivec3();
    let maximum = (bounds.1 - Vec3::splat(BOUNDS_EPSILON)).floor().as_ivec3();
    for y in minimum.y..=maximum.y {
        for z in minimum.z..=maximum.z {
            for x in minimum.x..=maximum.x {
                let voxel = IVec3::new(x, y, z);
                if !world.is_loaded_at(voxel)
                    || world.is_solid(voxel)
                    || (require_dry && world.fluid_at(voxel).is_some())
                {
                    return false;
                }
            }
        }
    }
    true
}

fn has_support(world: &VoxelWorld, bounds: (Vec3, Vec3)) -> bool {
    let offset = Vec3::Y * SUPPORT_PROBE;
    collides_aabb(world, bounds.0 - offset, bounds.1 - offset)
}

fn clear_destination(
    world: &VoxelWorld,
    eye: Vec3,
    blocked: (Vec3, Vec3),
    existing: &ExistingCreatures<'_, '_>,
    reserved: &[(Vec3, CreatureCollider)],
    require_support: bool,
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
    clear_volume(world, bounds, true) && (!require_support || has_support(world, bounds))
}

/// Search the perimeter outside the *entire* occupied volume. No edits happen
/// until a dry, loaded destination for the whole player is found. Spawning in
/// midair must not require the displaced player to be standing on the ground.
fn displaced_eye(
    world: &VoxelWorld,
    initial_eye: Vec3,
    blocked: (Vec3, Vec3),
    existing: &ExistingCreatures<'_, '_>,
    reserved: &[(Vec3, CreatureCollider)],
    require_support: bool,
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
        clear_destination(world, *eye, blocked, existing, reserved, require_support)
    })
}

/// Inspect the *loaded* world instead of procedural terrain. Manual /place
/// deliberately ignores terrain type, slope and fluid restrictions; it only
/// needs a loaded solid surface to anchor the structure above.
fn loaded_surface_level(world: &VoxelWorld, position: IVec2) -> Option<i32> {
    let highest = world.highest_loaded_world_y_in_column(position.x, position.y)?;
    let ground_y = (0..=highest)
        .rev()
        .find(|&y| world.is_solid(IVec3::new(position.x, y, position.y)))?;
    let above = IVec3::new(position.x, ground_y + 1, position.y);
    world.is_loaded_at(above).then_some(ground_y + 1)
}

fn manual_structure_hash(world_seed: u64, reference: &str, anchor: IVec2) -> u64 {
    let mut hash = world_seed ^ 0xcbf29ce484222325;
    for byte in reference
        .as_bytes()
        .iter()
        .copied()
        .chain(anchor.x.to_le_bytes())
        .chain(anchor.y.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

impl ChatPlacementContext<'_, '_> {
    pub(super) fn player_block_position(&mut self) -> Option<IVec3> {
        let player = self.player.single_mut().ok()?;
        Some(
            (player.translation - Vec3::Y * PLAYER_EYE_HEIGHT)
                .floor()
                .as_ivec3(),
        )
    }

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
        let world = self.runtime.world();
        let occupied = !clear_volume(world, blocked, false)
            || self.existing.iter().any(|(other, collider)| {
                overlaps(blocked, collider.bounds(other.translation))
            })
            || reserved.iter().any(|(other, collider)| {
                overlaps(blocked, collider.bounds(*other))
            });
        if occupied {
            return format!("not enough space to spawn {id}");
        }
        let Some(destination) = displaced_eye(
            world, player.translation, blocked, &self.existing, reserved, false,
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
        reference: &str,
        variation: Option<usize>,
        reserved: &[(Vec3, CreatureCollider)],
    ) -> String {
        let Some(variation_count) = self.structures.variation_count(reference) else {
            return format!("Unknown structure id or group: {reference}");
        };
        if let Some(variation) = variation {
            if variation > variation_count {
                return format!(
                    "Unknown variation {variation} for {reference}; expected 1..={variation_count}."
                );
            }
        }

        let Ok(mut player) = self.player.single_mut() else {
            return "Cannot place structure: player is unavailable.".to_owned();
        };
        let feet = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
        let anchor = feet.floor().as_ivec3().xz();
        let hash = manual_structure_hash(self.seed.0, reference, anchor);
        let structure = self
            .structures
            .select_for_manual_placement(reference, variation, hash)
            .expect("validated manual structure reference must resolve");
        let voxels = structure.voxels();
        let world = self.runtime.world();

        // /place is an explicit manual override. It does not apply biome,
        // proximity, Y-range, ground-block, dry-ground or slope restrictions.
        // The structure's lowest layer is simply anchored to the highest loaded
        // solid surface under the player.
        let Some(surface_y) = loaded_surface_level(world, anchor) else {
            return format!("no loaded ground available to place {reference}");
        };
        let origin_y = surface_y - structure.min_y_offset();
        let origin = IVec3::new(anchor.x, origin_y, anchor.y);
        let min = voxels
            .iter()
            .fold(IVec3::splat(i32::MAX), |min, voxel| min.min(origin + voxel.offset));
        let max = voxels
            .iter()
            .fold(IVec3::splat(i32::MIN), |max, voxel| max.max(origin + voxel.offset));
        let blocked = (min.as_vec3(), (max + IVec3::ONE).as_vec3());

        // Manual placement may replace terrain and fluids, but it still refuses
        // to touch unloaded chunks or place blocks through creatures requested
        // in this frame / already alive in the world.
        let all_loaded_and_entity_clear = voxels.iter().all(|voxel| {
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
        if !all_loaded_and_entity_clear {
            return format!("not enough loaded space to place {reference}");
        }

        let destination = if !overlaps(player_bounds(player.translation), blocked) {
            Some(player.translation)
        } else {
            displaced_eye(
                world, player.translation, blocked, &self.existing, reserved, true,
            )
        };
        let Some(destination) = destination else {
            return format!("not enough safe space to place {reference}");
        };

        for voxel in voxels {
            let position = origin + voxel.offset;
            let block = self.blocks.get(voxel.block_id).expect("validated structure block");
            let rotation = TextureRotation::for_position(position, block.rotate_texture.any());
            let cell = VoxelCell::oriented(voxel.block_id, rotation, voxel.orientation);
            let _ = self.runtime.set_block(position, Some(cell));
            for (face, layer) in surface_layer_placements(
                self.seed.0,
                structure,
                voxel,
                position,
            ) {
                let _ = self.runtime.add_layer(position, face, layer);
            }
        }
        player.translation = destination;
        format!(
            "Placed {} ({}).",
            structure.name.text(self.language.get()),
            structure.id
        )
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

    #[test]
    fn manual_structure_hash_is_stable_and_position_sensitive() {
        let first = manual_structure_hash(7, "asteria:tree_oak", IVec2::new(3, 4));
        assert_eq!(
            first,
            manual_structure_hash(7, "asteria:tree_oak", IVec2::new(3, 4))
        );
        assert_ne!(
            first,
            manual_structure_hash(7, "asteria:tree_oak", IVec2::new(4, 4))
        );
    }
}
