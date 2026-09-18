mod motion;
mod visual;

use bevy::prelude::*;

use crate::{
    entity::EntityHealth,
    app::{game_state::GameState, pause_state::PauseState},
    content::{biome::BiomeRegistry, creature::CreatureRegistry},
    localization::{ActiveLanguage, Language},
    player::camera::GameplayCamera,
    world::biome::CurrentBiome,
    voxel::world::VoxelWorld,
};

use motion::{CreatureMotion, move_creatures};
use visual::{CreatureModel, attach_loaded_models, sync_creature_animations, sync_creature_facing};
pub(crate) use visual::CreatureAnimationState;

/// The entity root owns position and collision; only its visual child is animated or rotated.
#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

pub(crate) struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::TintedCreatureMaterials>()
            .add_systems(Update, natural_spawn_creatures.run_if(in_state(GameState::Gameplay)).run_if(in_state(PauseState::Running)))
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
        EntityHealth::new(definition.health),
        definition.collider,
        Transform::from_translation(feet),
        Visibility::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
    Ok(name)
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
    creatures: Query<(&CreatureInstance, &Transform)>,
    mut commands: Commands,
    mut state: Local<(f32, u32)>,
) {
    state.0 -= time.delta_secs();
    if state.0 > 0.0 {
        return;
    }
    state.0 = NATURAL_SPAWN_INTERVAL;
    if state.1 == 0 { state.1 = player.translation.x.to_bits() ^ player.translation.z.to_bits().rotate_left(13) ^ 0x9E37_79B9; }

    let total = creatures.iter().count();
    let Some(biome_definition) = biomes.get(&biome.id) else { return; };
    let candidates: Vec<_> = biome_definition
        .creature_spawns
        .iter()
        .filter(|rule| rule.weight > 0.0 && definitions.get(&rule.creature).is_some())
        .filter(|rule| {
            let type_count = creatures
                .iter()
                .filter(|(instance, _)| instance.definition_id == rule.creature)
                .count();
            type_count < rule.max_per_type && total < rule.max_in_dimension
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
        if creatures.iter().any(|(instance, transform)| instance.definition_id == rule.creature && transform.translation.distance(feet) < rule.spacing) { continue; }
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
