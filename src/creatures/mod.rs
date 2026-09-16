mod motion;
mod visual;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::creature::{CreatureCollider, CreatureRegistry},
    localization::{ActiveLanguage, Language},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera, find_safe_spawn_position},
    voxel::{collision::collides_aabb, world::VoxelWorld},
};

use motion::{CreatureMotion, move_creatures};
use visual::{CreatureModel, attach_loaded_models, sync_creature_animations};

/// The entity root owns position and collision; only its visual child is animated.
#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

pub(crate) struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::TintedCreatureMaterials>()
            .add_systems(OnEnter(GameState::Gameplay), spawn_preview_creatures)
            .add_systems(
                Update,
                attach_loaded_models.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                move_creatures
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                PostUpdate,
                sync_creature_animations.run_if(in_state(GameState::Gameplay)),
            );
    }
}

/// Spawns any registered creature at a player's position, retaining the exact
/// horizontal coordinates and using the player's feet as the creature origin.
/// The model, name, animation mapping and collider are owned by the JSON.
pub(crate) fn spawn_creature_at(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    world: &VoxelWorld,
    language: Language,
    id: &str,
    player_eye: Vec3,
) -> Result<String, String> {
    let definition = definitions
        .get(id)
        .ok_or_else(|| format!("Unknown creature id: {id}"))?;
    let feet = player_eye - Vec3::Y * PLAYER_EYE_HEIGHT;
    let collider: CreatureCollider = definition.collider;
    let (min, max) = collider.bounds(feet);
    if !world.is_loaded_at(min.floor().as_ivec3())
        || !world.is_loaded_at(max.floor().as_ivec3())
    {
        return Err("Cannot spawn creature: the destination is not loaded.".to_owned());
    }
    if collides_aabb(world, min, max) {
        return Err("Cannot spawn creature: the destination is obstructed.".to_owned());
    }

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
    Ok(name)
}

/// Temporary opt-in preview, not biome spawning or world persistence.
fn spawn_preview_creatures(
    mut commands: Commands,
    creatures: Res<CreatureRegistry>,
    players: Query<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    asset_server: Res<AssetServer>,
    active_language: Res<ActiveLanguage>,
) {
    let Some(player) = players.iter().next() else {
        warn!("creature previews skipped: player not spawned");
        return;
    };
    let mut previews: Vec<_> = creatures
        .iter()
        .filter(|definition| definition.preview_spawn)
        .collect();
    previews.sort_by(|left, right| left.id.cmp(&right.id));
    let base = player.translation.floor().as_ivec3();

    for (index, definition) in previews.into_iter().enumerate() {
        let preferred = IVec2::new(base.x + 3 + index as i32 * 2, base.z + 3);
        let Some(eye_position) = find_safe_spawn_position(&world, preferred, |_| true) else {
            warn!("no valid surface for creature preview {}", definition.id);
            continue;
        };
        if let Err(error) = spawn_creature_at(
            &mut commands,
            &creatures,
            &asset_server,
            &world,
            active_language.get(),
            &definition.id,
            eye_position,
        ) {
            warn!("creature preview {}: {error}", definition.id);
        }
    }
}
