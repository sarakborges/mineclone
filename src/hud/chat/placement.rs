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
    world::{
        generation::{fit_structure_to_ground, surface_layer_placements},
        seed::WorldSeed,
    },
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

/// Inspect the *loaded* world instead of predicting an old procedural height:
/// structures must fit the terrain currently present, including player edits.
/// Return the first empty level directly above the highest solid voxel.
fn loaded_surface_level(world: &VoxelWorld, position: IVec2) -> Option<i32> {
    let highest = world.highest_loaded_world_y_in_column(position.x, position.y)?;
    let ground_y = (0..=highest).rev().find(|&y| {
        world.is_solid(IVec3::new(position.x, y, position.y))
    })?;
    let above = IVec3::new(position.x, ground_y + 1, position.y);
    (world.is_loaded_at(above)
        && !world.is_solid(above)
        && world.fluid_at(above).is_none())
    .then_some(ground_y + 1)
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
        id: &str,
        reserved: &[(Vec3, CreatureCollider)],
    ) -> String {
        let Some(structure) = self.structures.get(id) else {
            return format!("Unknown structure id: {id}");
        };
        let Ok(mut player) = self.player.single_mut() else {
            return "Cannot place structure: player is unavailable.".to_owned();
        };
        let feet = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
        let anchor = feet.floor().as_ivec3().xz();
        let voxels = structure.voxels();
        let world = self.runtime.world();
        // Reuse the world generator's footprint and slope-fitting rule. Unlike
        // worldgen's density-based ground voxel, the live-world sample returns
        // the first empty level above terrain so existing blocks are preserved.
        let Some(origin_y) = fit_structure_to_ground(
            anchor,
            structure.support_offsets(),
            structure.min_y_offset(),
            structure.restrictions.max_slope,
            |position| loaded_surface_level(world, position),
        ) else {
            return format!("not enough space to place {id}");
        };
        let origin = IVec3::new(anchor.x, origin_y, anchor.y);
        let min = voxels
            .iter()
            .fold(IVec3::splat(i32::MAX), |min, voxel| min.min(origin + voxel.offset));
        let max = voxels
            .iter()
            .fold(IVec3::splat(i32::MIN), |max, voxel| max.max(origin + voxel.offset));
        let blocked = (min.as_vec3(), (max + IVec3::ONE).as_vec3());
        let all_loaded_and_clear = voxels.iter().all(|voxel| {
            let position = origin + voxel.offset;
            let voxel_bounds = (position.as_vec3(), position.as_vec3() + Vec3::ONE);
            world.is_loaded_at(position)
                && !world.is_solid(position)
                && world.fluid_at(position).is_none()
                && !self.existing.iter().any(|(other, collider)| {
                    overlaps(voxel_bounds, collider.bounds(other.translation))
                })
                && !reserved.iter().any(|(other, collider)| {
                    overlaps(voxel_bounds, collider.bounds(*other))
                })
        });
        // Every bottom-layer structure voxel must be supported. On a slope
        // with no safe flush fit we reject rather than leave floating blocks.
        let has_foundation = voxels.iter().filter(|voxel| origin.y + voxel.offset.y == min.y)
            .all(|voxel| world.is_solid(origin + voxel.offset - IVec3::Y));
        if !all_loaded_and_clear || !has_foundation {
            return format!("not enough space to place {id}");
        }
        let destination = if !overlaps(player_bounds(player.translation), blocked) {
            Some(player.translation)
        } else {
            displaced_eye(
                world, player.translation, blocked, &self.existing, reserved, true,
            )
        };
        let Some(destination) = destination else {
            return format!("not enough space to place {id}");
        };

        // Every target cell is vacant, dry and loaded before the first edit.
        // Mutations cannot return None under this preflight, so no partial
        // structure can be silently reported as a successful placement.
        for voxel in voxels {
            let position = origin + voxel.offset;
            let block = self.blocks.get(voxel.block_id).expect("validated structure block");
            let rotation = TextureRotation::for_position(position, block.rotate_texture.any());
            let cell = VoxelCell::oriented(voxel.block_id, rotation, voxel.orientation);
            self.runtime
                .set_block(position, Some(cell))
                .expect("preflight guarantees a loaded empty structure voxel");
            for (face, layer) in surface_layer_placements(
                self.seed.0,
                structure,
                voxel,
                position,
            ) {
                self.runtime
                    .add_layer(position, face, layer)
                    .expect("placed structure layer must attach to its freshly placed support");
            }
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
