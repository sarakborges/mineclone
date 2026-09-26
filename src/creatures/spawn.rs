use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::creature::CreatureRegistry,
    entity::EntityHealth,
    localization::{ActiveLanguage, Language},
    voxel::world::VoxelWorld,
};

use super::{
    CreatureInstance, CreatureTargetCollider, PendingCreatureRestores,
    motion::CreatureMotion,
    particles::CreatureParticleEmitter,
    visual::CreatureModel,
};

/// The chat preflights world occupancy and relocates the player before calling
/// this function. The exact feet position is preserved; this function never
/// searches neighboring slots or mutates the terrain.
pub(crate) fn spawn_creature_at(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    language: Language,
    id: &str,
    feet: Vec3,
) -> Result<String, String> {
    spawn_creature_with_health(
        commands,
        definitions,
        asset_server,
        language,
        id,
        feet,
        None,
    )
}

fn spawn_creature_with_health(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    language: Language,
    id: &str,
    feet: Vec3,
    saved_health: Option<f32>,
) -> Result<String, String> {
    let definition = definitions
        .get(id)
        .ok_or_else(|| format!("Unknown creature id: {id}"))?;
    let name = definition.name.text(language).to_owned();
    commands.spawn((
        Name::new(name.clone()),
        CreatureInstance {
            definition_id: definition.id.clone(),
        },
        CreatureModel(asset_server.load(definition.model.clone())),
        CreatureMotion::default(),
        CreatureParticleEmitter::default(),
        saved_health.map_or_else(
            || EntityHealth::new(definition.health),
            |health| EntityHealth::restored(definition.health, health),
        ),
        definition.collider,
        CreatureTargetCollider(definition.target_collider()),
        Transform::from_translation(feet),
        Visibility::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
    Ok(name)
}

pub(super) fn restore_saved_creatures(
    mut commands: Commands,
    world: Res<VoxelWorld>,
    definitions: Res<CreatureRegistry>,
    asset_server: Res<AssetServer>,
    language: Res<ActiveLanguage>,
    mut pending: ResMut<PendingCreatureRestores>,
) {
    if pending.is_empty() {
        return;
    }

    let saved = pending.take();
    for creature in saved {
        let feet = Vec3::from_array(creature.position);
        if !world.is_loaded_at(feet.floor().as_ivec3()) {
            pending.defer(creature);
            continue;
        }

        if let Err(error) = spawn_creature_with_health(
            &mut commands,
            &definitions,
            &asset_server,
            language.get(),
            &creature.definition_id,
            feet,
            Some(creature.health),
        ) {
            warn!(
                "Could not restore creature {}: {error}",
                creature.definition_id
            );
        }
    }
}
