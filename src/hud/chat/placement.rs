use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::creature::{CreatureCollider, CreatureRegistry},
    creatures::{CreatureInstance, spawn_creature_at},
    localization::ActiveLanguage,
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, camera::GameplayCamera},
    voxel::{edit::VoxelTopologyRuntime, read::VoxelRead},
};

const DISPLACEMENT_MARGIN: i32 = 3;
const DISPLACEMENT_HEIGHTS: [i32; 5] = [0, 1, -1, 2, -2];
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
    assets: Res<'w, AssetServer>,
    language: Res<'w, ActiveLanguage>,
    runtime: VoxelTopologyRuntime<'w>,
    player:
        Query<'w, 's, &'static mut Transform, (With<GameplayCamera>, Without<CreatureInstance>)>,
    existing: ExistingCreatures<'w, 's>,
}

fn overlaps(left: (Vec3, Vec3), right: (Vec3, Vec3)) -> bool {
    left.0.x < right.1.x
        && left.1.x > right.0.x
        && left.0.y < right.1.y
        && left.1.y > right.0.y
        && left.0.z < right.1.z
        && left.1.z > right.0.z
}

fn player_bounds(eye: Vec3) -> (Vec3, Vec3) {
    let feet_y = eye.y - PLAYER_EYE_HEIGHT;
    (
        Vec3::new(eye.x - PLAYER_HALF_WIDTH, feet_y, eye.z - PLAYER_HALF_WIDTH),
        Vec3::new(
            eye.x + PLAYER_HALF_WIDTH,
            feet_y + PLAYER_HEIGHT,
            eye.z + PLAYER_HALF_WIDTH,
        ),
    )
}

fn clear_volume(world: &impl VoxelRead, bounds: (Vec3, Vec3), require_dry: bool) -> bool {
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

fn clear_destination(
    world: &impl VoxelRead,
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
    clear_volume(world, bounds, true)
}

fn displaced_eye(
    world: &impl VoxelRead,
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
    candidates
        .into_iter()
        .find(|eye| clear_destination(world, *eye, blocked, existing, reserved))
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
        let world = self.runtime.read();
        let occupied = !clear_volume(&world, blocked, false)
            || self
                .existing
                .iter()
                .any(|(other, collider)| overlaps(blocked, collider.bounds(other.translation)))
            || reserved
                .iter()
                .any(|(other, collider)| overlaps(blocked, collider.bounds(*other)));
        if occupied {
            return format!("not enough space to spawn {id}");
        }
        let Some(destination) = displaced_eye(
            &world,
            player.translation,
            blocked,
            &self.existing,
            reserved,
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
        _reference: &str,
        _variation: Option<usize>,
        _reserved: &[(Vec3, CreatureCollider)],
    ) -> String {
        "World generation rebuild in progress; /place structure is temporarily unavailable."
            .to_owned()
    }
}
