use bevy::prelude::*;

use crate::{
    app::{crash_log::log_gameplay_event, game_state::GameState},
    content::creature::CreatureRegistry,
    entity::EntityHealth,
    localization::{ActiveLanguage, Language},
    voxel::world::VoxelWorld,
};

use super::{
    CreatureDespawnGrace, CreatureInstance, CreatureTargetCollider, EntityMetaTags,
    PendingCreatureRestores, motion::CreatureMotion, particles::CreatureParticleEmitter,
    visual::CreatureModel,
};

struct CreatureSpawnState {
    health: Option<f32>,
    meta_tags: EntityMetaTags,
}

pub(crate) fn spawn_creature_at(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    language: Language,
    id: &str,
    feet: Vec3,
) -> Result<String, String> {
    spawn_creature_at_with_tags(
        commands,
        definitions,
        asset_server,
        language,
        id,
        feet,
        EntityMetaTags::default(),
    )
}

pub(crate) fn spawn_creature_at_with_tags(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    language: Language,
    id: &str,
    feet: Vec3,
    meta_tags: EntityMetaTags,
) -> Result<String, String> {
    spawn_creature_with_state(
        commands,
        definitions,
        asset_server,
        language,
        id,
        feet,
        CreatureSpawnState {
            health: None,
            meta_tags,
        },
    )
}

fn spawn_creature_with_state(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    language: Language,
    id: &str,
    feet: Vec3,
    state: CreatureSpawnState,
) -> Result<String, String> {
    let definition = definitions
        .get(id)
        .ok_or_else(|| format!("Unknown creature id: {id}"))?;
    let name = definition.name.text(language).to_owned();
    let meta_tags_for_log = state.meta_tags.clone();
    let health_for_log = state.health.unwrap_or(definition.health);
    let entity = commands
        .spawn((
            Name::new(name.clone()),
            CreatureInstance {
                definition_id: definition.id.clone(),
            },
            CreatureModel(asset_server.load(definition.model.clone())),
            CreatureMotion::default(),
            CreatureParticleEmitter::default(),
            CreatureDespawnGrace::default(),
            state.health.map_or_else(
                || EntityHealth::new(definition.health),
                |health| EntityHealth::restored(definition.health, health),
            ),
            state.meta_tags,
            definition.collider,
            CreatureTargetCollider(definition.target_collider()),
            Transform::from_translation(feet),
            Visibility::default(),
            DespawnOnExit(GameState::Gameplay),
        ))
        .id();
    log_gameplay_event(format!(
        "entity.spawn entity={:?} type=creature id={} name={} position={:?} health={:.3} meta_tags={:?}",
        entity,
        definition.id,
        name,
        feet,
        health_for_log,
        meta_tags_for_log
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

        if let Err(error) = spawn_creature_with_state(
            &mut commands,
            &definitions,
            &asset_server,
            language.get(),
            &creature.definition_id,
            feet,
            CreatureSpawnState {
                health: Some(creature.health),
                meta_tags: creature.meta_tags,
            },
        ) {
            warn!(
                "Could not restore creature {}: {error}",
                creature.definition_id
            );
        }
    }
}
