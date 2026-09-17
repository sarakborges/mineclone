mod motion;
mod visual;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::creature::{CreatureCollider, CreatureRegistry},
    localization::Language,
    player::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT},
    voxel::{collision::collides_aabb, world::VoxelWorld},
};

use motion::{CreatureMotion, move_creatures};
use visual::{CreatureModel, attach_loaded_models, sync_creature_animations, sync_creature_facing};

/// The entity root owns position and collision; only its visual child is animated or rotated.
#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

pub(crate) struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::TintedCreatureMaterials>()
            // Creatures only spawn through explicit gameplay commands, never world creation.
            .add_systems(Update, attach_loaded_models.run_if(in_state(GameState::Gameplay)))
            .add_systems(
                Update,
                move_creatures
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                PostUpdate,
                (sync_creature_facing, sync_creature_animations)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

/// The eight neighboring slots are two block centers away from the player.
/// Try each slot at the player's feet and one block above/below, requiring
/// loaded, dry, solid support and no overlap with player or any creature.
const NEIGHBORS: [IVec2; 8] = [
    IVec2::new(0, -1),
    IVec2::new(1, -1),
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
    IVec2::new(-1, 1),
    IVec2::new(-1, 0),
    IVec2::new(-1, -1),
];

fn overlaps(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> bool {
    a.0.x < b.1.x && a.1.x > b.0.x
        && a.0.y < b.1.y && a.1.y > b.0.y
        && a.0.z < b.1.z && a.1.z > b.0.z
}

fn clear_grounded_slot(world: &VoxelWorld, collider: CreatureCollider, feet: Vec3) -> bool {
    let (min, max) = collider.bounds(feet);
    let min_voxel = min.floor().as_ivec3();
    let max_voxel = (max - Vec3::splat(0.0001)).floor().as_ivec3();
    for y in min_voxel.y..=max_voxel.y {
        for z in min_voxel.z..=max_voxel.z {
            for x in min_voxel.x..=max_voxel.x {
                let cell = IVec3::new(x, y, z);
                if !world.is_loaded_at(cell) || world.fluid_at(cell).is_some() {
                    return false;
                }
            }
        }
    }
    if collides_aabb(world, min, max) {
        return false;
    }
    // A collision directly below the feet proves the candidate is supported.
    let (below_min, below_max) = collider.bounds(feet - Vec3::Y * 0.08);
    collides_aabb(world, below_min, below_max)
}

pub(crate) fn spawn_creature_near_player(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    world: &VoxelWorld,
    language: Language,
    id: &str,
    player_eye: Vec3,
    existing: &Query<(&Transform, &CreatureCollider), With<CreatureInstance>>,
    reserved: &mut Vec<(Vec3, CreatureCollider)>,
) -> Result<String, String> {
    let definition = definitions
        .get(id)
        .ok_or_else(|| format!("Unknown creature id: {id}"))?;
    let collider = definition.collider;
    let player_feet = player_eye - Vec3::Y * PLAYER_EYE_HEIGHT;
    let player_bounds = (
        player_feet + Vec3::new(-PLAYER_HALF_WIDTH, 0.0, -PLAYER_HALF_WIDTH),
        player_feet + Vec3::new(PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HALF_WIDTH),
    );
    let base = player_feet.y.floor() as i32;
    let center = player_eye.floor().as_ivec3();
    let mut destination = None;
    'positions: for offset in NEIGHBORS {
        let x = center.x as f32 + 0.5 + offset.x as f32 * 2.0;
        let z = center.z as f32 + 0.5 + offset.y as f32 * 2.0;
        for y in [base, base + 1, base - 1] {
            let feet = Vec3::new(x, y as f32, z);
            if !clear_grounded_slot(world, collider, feet) {
                continue;
            }
            let bounds = collider.bounds(feet);
            if overlaps(bounds, player_bounds)
                || existing.iter().any(|(other, other_collider)| {
                    overlaps(bounds, other_collider.bounds(other.translation))
                })
                || reserved.iter().any(|(position, other_collider)| {
                    overlaps(bounds, other_collider.bounds(*position))
                })
            {
                continue;
            }
            destination = Some(feet);
            break 'positions;
        }
    }
    let feet = destination.ok_or_else(|| {
        "Cannot spawn creature: all eight surrounding positions are obstructed, unloaded or unsupported."
            .to_owned()
    })?;

    let name = definition.name.text(language).to_owned();
    commands.spawn((
        Name::new(name.clone()),
        CreatureInstance {
            definition_id: definition.id.clone(),
        },
        CreatureModel(asset_server.load(definition.model.clone())),
        CreatureMotion::default(),
        collider,
        Transform::from_translation(feet),
        Visibility::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
    // Commands are deferred: reserve the slot for further commands this frame.
    reserved.push((feet, collider));
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_aabbs_do_not_overlap_but_intersecting_ones_do() {
        assert!(!overlaps((Vec3::ZERO, Vec3::ONE), (Vec3::X, Vec3::X + Vec3::ONE)));
        assert!(overlaps((Vec3::ZERO, Vec3::ONE), (Vec3::splat(0.5), Vec3::splat(1.5))));
        assert_eq!(NEIGHBORS.len(), 8);
    }
}
