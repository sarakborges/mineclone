use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        biome::{BiomeRegistry, CreatureSpawnRule},
        creature::CreatureRegistry,
        dimension::DimensionRegistry,
    },
    entity::EntityHealth,
    localization::ActiveLanguage,
    player::camera::GameplayCamera,
    voxel::world::VoxelWorld,
    world::{
        biome::CurrentBiome,
        dimension::{CurrentDimension, DimensionEntityCounts},
        game_rules::GameRules,
    },
};

use super::{CreatureInstance, random::next_u32, spawn_creature_at};

const NATURAL_SPAWN_INTERVAL: f32 = 1.0;
const NATURAL_SPAWN_MIN_DISTANCE: f32 = 8.0;
const NATURAL_SPAWN_MAX_DISTANCE: f32 = 32.0;
const NATURAL_SPAWN_ATTEMPTS: usize = 8;

type NaturalSpawnCreatures<'w, 's> = Query<
    'w,
    's,
    (
        &'static CreatureInstance,
        &'static Transform,
        &'static EntityHealth,
    ),
>;

#[derive(Default)]
pub(super) struct NaturalSpawnState {
    seconds_until_attempt: f32,
    random_state: u32,
}

#[derive(SystemParam)]
pub(super) struct NaturalSpawnContext<'w, 's> {
    rules: Res<'w, GameRules>,
    world: Res<'w, VoxelWorld>,
    biome: Res<'w, CurrentBiome>,
    definitions: Res<'w, CreatureRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    language: Res<'w, ActiveLanguage>,
    asset_server: Res<'w, AssetServer>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    creatures: NaturalSpawnCreatures<'w, 's>,
    current_dimension: Res<'w, CurrentDimension>,
    entity_counts: ResMut<'w, DimensionEntityCounts>,
    dimensions: Res<'w, DimensionRegistry>,
}

pub(super) fn natural_spawn_creatures(
    time: Res<Time>,
    mut context: NaturalSpawnContext<'_, '_>,
    mut commands: Commands,
    mut state: Local<NaturalSpawnState>,
) {
    if !context.rules.spawn_creatures() {
        return;
    }

    state.seconds_until_attempt -= time.delta_secs();
    if state.seconds_until_attempt > 0.0 {
        return;
    }
    state.seconds_until_attempt = NATURAL_SPAWN_INTERVAL;
    if state.random_state == 0 {
        state.random_state = context.player.translation.x.to_bits()
            ^ context.player.translation.z.to_bits().rotate_left(13)
            ^ 0x9E37_79B9;
    }

    context.entity_counts.rebuild(
        context
            .creatures
            .iter()
            .filter(|(_, _, health)| !health.is_dead())
            .map(|(instance, _, _)| instance),
    );
    let Some(dimension_definition) = context.dimensions.get(&context.current_dimension.id) else {
        return;
    };
    let Some(biome_definition) = context.biomes.get(&context.biome.id) else {
        return;
    };
    let Some(rule) = select_natural_spawn_rule(
        &biome_definition.creature_spawns,
        &context.definitions,
        &context.entity_counts,
        dimension_definition.max_entities,
        &mut state.random_state,
    ) else {
        return;
    };

    let Some(feet) = find_natural_spawn_position(
        &context.world,
        &context.creatures,
        context.player.translation,
        rule,
        &mut state.random_state,
    ) else {
        return;
    };
    let _ = spawn_creature_at(
        &mut commands,
        &context.definitions,
        &context.asset_server,
        context.language.get(),
        &rule.creature,
        feet,
    );
}

fn find_natural_spawn_position(
    world: &VoxelWorld,
    creatures: &NaturalSpawnCreatures<'_, '_>,
    player_position: Vec3,
    rule: &CreatureSpawnRule,
    random_state: &mut u32,
) -> Option<Vec3> {
    for _ in 0..NATURAL_SPAWN_ATTEMPTS {
        let angle = next_u32(random_state) as f32 / u32::MAX as f32 * std::f32::consts::TAU;
        let distance = NATURAL_SPAWN_MIN_DISTANCE
            + (next_u32(random_state) as f32 / u32::MAX as f32)
                * (NATURAL_SPAWN_MAX_DISTANCE - NATURAL_SPAWN_MIN_DISTANCE);
        let position =
            player_position + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance);
        let column = IVec2::new(position.x.floor() as i32, position.z.floor() as i32);
        let Some(feet_y) = natural_spawn_feet_y(world, column) else {
            continue;
        };
        let feet = Vec3::new(
            column.x as f32 + 0.5,
            feet_y as f32,
            column.y as f32 + 0.5,
        );
        let light = world.light_at(feet.floor().as_ivec3());
        let light_level = light.sky().max(light.block());
        if light_level < rule.light_min || light_level > rule.light_max {
            continue;
        }
        if creatures.iter().any(|(instance, transform, health)| {
            !health.is_dead()
                && instance.definition_id == rule.creature
                && transform.translation.distance(feet) < rule.spacing
        }) {
            continue;
        }
        if !world.is_loaded_at(feet.floor().as_ivec3())
            || world.fluid_at(feet.floor().as_ivec3()).is_some()
        {
            continue;
        }
        return Some(feet);
    }
    None
}

fn natural_spawn_rule_is_eligible(
    rule: &CreatureSpawnRule,
    definitions: &CreatureRegistry,
    entity_counts: &DimensionEntityCounts,
) -> bool {
    rule.weight > 0.0
        && definitions.get(&rule.creature).is_some_and(|creature| {
            entity_counts.count(&rule.creature) < creature.max_per_type
        })
}

fn select_natural_spawn_rule<'a>(
    rules: &'a [CreatureSpawnRule],
    definitions: &CreatureRegistry,
    entity_counts: &DimensionEntityCounts,
    max_entities: usize,
    random_state: &mut u32,
) -> Option<&'a CreatureSpawnRule> {
    if entity_counts.total >= max_entities {
        return None;
    }

    let total_weight = rules
        .iter()
        .filter(|rule| natural_spawn_rule_is_eligible(rule, definitions, entity_counts))
        .map(|rule| rule.weight)
        .sum::<f32>();
    if total_weight <= 0.0 {
        return None;
    }

    let roll = next_u32(random_state) as f32 / u32::MAX as f32 * total_weight;
    let mut cursor = 0.0;
    rules
        .iter()
        .filter(|rule| natural_spawn_rule_is_eligible(rule, definitions, entity_counts))
        .find(|rule| {
            cursor += rule.weight;
            roll <= cursor
        })
}

fn natural_spawn_feet_y(world: &VoxelWorld, column: IVec2) -> Option<i32> {
    let highest = world.highest_loaded_world_y_in_column(column.x, column.y)?;
    for support_y in (0..=highest).rev() {
        let support = IVec3::new(column.x, support_y, column.y);
        let feet = support + IVec3::Y;
        let head = feet + IVec3::Y;
        if world.is_solid(support)
            && world.is_loaded_at(head)
            && !world.is_solid(feet)
            && !world.is_solid(head)
            && world.fluid_at(feet).is_none()
        {
            return Some(feet.y);
        }
    }
    None
}
