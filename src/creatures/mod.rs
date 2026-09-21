mod motion;
mod visual;

use std::io;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    entity::EntityHealth,
    app::{game_state::GameState, pause_state::PauseState, resource_systems::reset_resource},
    content::{biome::BiomeRegistry, creature::CreatureRegistry},
    localization::{ActiveLanguage, Language},
    player::camera::GameplayCamera,
    world::{biome::CurrentBiome, dimension::{CurrentDimension, DimensionEntityCounts}},
    voxel::world::VoxelWorld,
};

pub(crate) use motion::CreatureMotion;
use motion::move_creatures;
use visual::{CreatureModel, attach_loaded_models, sync_creature_animations, sync_creature_facing};
pub(crate) use visual::CreatureAnimationState;

/// The entity root owns position and collision; only its visual child is animated or rotated.
#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

#[derive(Component)]
pub(crate) struct CreatureDeathTimer(pub(crate) Timer);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedCreature {
    pub(crate) definition_id: String,
    pub(crate) position: [f32; 3],
    pub(crate) health: f32,
}

impl SavedCreature {
    pub(crate) fn validate(&self, definitions: &CreatureRegistry) -> io::Result<()> {
        if definitions.get(&self.definition_id).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown saved creature definition: {}", self.definition_id),
            ));
        }
        if self.position.iter().any(|value| !value.is_finite())
            || !self.health.is_finite()
            || self.health <= 0.0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "saved creature position or health is invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Resource, Default)]
pub(crate) struct PendingCreatureRestores {
    creatures: Vec<SavedCreature>,
}

impl PendingCreatureRestores {
    pub(crate) fn new(creatures: Vec<SavedCreature>) -> Self {
        Self { creatures }
    }

    pub(crate) fn saved(&self) -> &[SavedCreature] {
        &self.creatures
    }
}

pub(crate) struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::TintedCreatureMaterials>()
            .init_resource::<PendingCreatureRestores>()
            .add_systems(
                OnEnter(GameState::StartingScreen),
                (
                    reset_resource::<PendingCreatureRestores>,
                    reset_resource::<visual::TintedCreatureMaterials>,
                ),
            )
            .add_systems(
                Update,
                restore_saved_creatures
                    .run_if(in_state(GameState::Gameplay))
                    .before(natural_spawn_creatures),
            )
            .add_systems(Update, natural_spawn_creatures.run_if(in_state(GameState::Gameplay)).run_if(in_state(PauseState::Running)))
            .add_systems(Update, despawn_dead_creatures.run_if(in_state(GameState::Gameplay)))
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
        saved_health.map_or_else(
            || EntityHealth::new(definition.health),
            |health| EntityHealth::restored(definition.health, health),
        ),
        definition.collider,
        Transform::from_translation(feet),
        Visibility::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
    Ok(name)
}

fn restore_saved_creatures(
    mut commands: Commands,
    world: Res<VoxelWorld>,
    definitions: Res<CreatureRegistry>,
    asset_server: Res<AssetServer>,
    language: Res<ActiveLanguage>,
    mut pending: ResMut<PendingCreatureRestores>,
) {
    if pending.creatures.is_empty() {
        return;
    }

    let saved = std::mem::take(&mut pending.creatures);
    for creature in saved {
        let feet = Vec3::from_array(creature.position);
        if !world.is_loaded_at(feet.floor().as_ivec3()) {
            pending.creatures.push(creature);
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


const NATURAL_SPAWN_INTERVAL: f32 = 1.0;
const NATURAL_SPAWN_MIN_DISTANCE: f32 = 8.0;
const NATURAL_SPAWN_MAX_DISTANCE: f32 = 32.0;

#[allow(clippy::too_many_arguments)]
fn natural_spawn_creatures(
    time: Res<Time>,
    world: Res<VoxelWorld>,
    biome: Res<CurrentBiome>,
    definitions: Res<CreatureRegistry>,
    biomes: Res<BiomeRegistry>,
    language: Res<ActiveLanguage>,
    asset_server: Res<AssetServer>,
    player: Single<&Transform, With<GameplayCamera>>,
    creatures: Query<(&CreatureInstance, &Transform, &EntityHealth)>,
    current_dimension: Res<CurrentDimension>,
    mut entity_counts: ResMut<DimensionEntityCounts>,
    dimensions: Res<crate::content::dimension::DimensionRegistry>,
    mut commands: Commands,
    mut state: Local<(f32, u32)>,
) {
    state.0 -= time.delta_secs();
    if state.0 > 0.0 {
        return;
    }
    state.0 = NATURAL_SPAWN_INTERVAL;
    if state.1 == 0 { state.1 = player.translation.x.to_bits() ^ player.translation.z.to_bits().rotate_left(13) ^ 0x9E37_79B9; }

    entity_counts.rebuild(
        creatures
            .iter()
            .filter(|(_, _, health)| !health.is_dead())
            .map(|(instance, _, _)| instance),
    );
    let Some(dimension_definition) = dimensions.get(&current_dimension.id) else { return; };
    let Some(biome_definition) = biomes.get(&biome.id) else { return; };
    let candidates: Vec<_> = biome_definition
        .creature_spawns
        .iter()
        .filter(|rule| rule.weight > 0.0 && definitions.get(&rule.creature).is_some())
        .filter(|_| entity_counts.total < dimension_definition.max_entities)
        .filter(|rule| {
            let Some(creature) = definitions.get(&rule.creature) else { return false; };
            entity_counts.count(&rule.creature) < creature.max_per_type
        })
        .collect();
    if candidates.is_empty() { return; }
    let total_weight: f32 = candidates.iter().map(|rule| rule.weight).sum();
    if total_weight <= 0.0 { return; }
    let roll = next_random(&mut state.1) as f32 / u32::MAX as f32 * total_weight;
    let mut cursor = 0.0;
    let rule = candidates
        .into_iter()
        .find(|rule| { cursor += rule.weight; roll <= cursor })
        .unwrap_or_else(|| panic!("spawn candidate selection failed"));

    for _ in 0..8 {
        let angle = next_random(&mut state.1) as f32 / u32::MAX as f32 * std::f32::consts::TAU;
        let distance = NATURAL_SPAWN_MIN_DISTANCE + (next_random(&mut state.1) as f32 / u32::MAX as f32) * (NATURAL_SPAWN_MAX_DISTANCE - NATURAL_SPAWN_MIN_DISTANCE);
        let position = player.translation + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance);
        let column = IVec2::new(position.x.floor() as i32, position.z.floor() as i32);
        let Some(feet_y) = natural_spawn_feet_y(&world, column) else { continue; };
        let feet = Vec3::new(column.x as f32 + 0.5, feet_y as f32, column.y as f32 + 0.5);
        let light = world.light_at(feet.floor().as_ivec3());
        let light_level = light.sky().max(light.block());
        if light_level < rule.light_min || light_level > rule.light_max { continue; }
        if creatures.iter().any(|(instance, transform, health)| {
            !health.is_dead()
                && instance.definition_id == rule.creature
                && transform.translation.distance(feet) < rule.spacing
        }) {
            continue;
        }
        if !world.is_loaded_at(feet.floor().as_ivec3()) || world.fluid_at(feet.floor().as_ivec3()).is_some() { continue; }
        let _ = spawn_creature_at(&mut commands, &definitions, &asset_server, language.get(), &rule.creature, feet);
        break;
    }
}

fn natural_spawn_feet_y(world: &VoxelWorld, column: IVec2) -> Option<i32> {
    let highest = world.highest_loaded_world_y_in_column(column.x, column.y)?;
    for support_y in (0..=highest).rev() {
        let support = IVec3::new(column.x, support_y, column.y);
        let feet = support + IVec3::Y;
        let head = feet + IVec3::Y;
        if world.is_solid(support) && world.is_loaded_at(head) && !world.is_solid(feet) && !world.is_solid(head) && world.fluid_at(feet).is_none() {
            return Some(feet.y);
        }
    }
    None
}

fn next_random(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}


fn despawn_dead_creatures(
    time: Res<Time>,
    mut commands: Commands,
    mut dead: Query<(Entity, &mut CreatureDeathTimer)>,
) {
    for (entity, mut timer) in &mut dead {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
