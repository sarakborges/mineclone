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
    voxel::{
        coordinates::chunk_coord_from_position,
        world::VoxelWorld,
    },
    world::{
        biome_field::BiomeField,
        dimension::{CurrentDimension, DimensionEntityCounts},
        game_rules::GameRules,
        generation_region::{generation_region_coord, generation_region_world_bounds},
        world_feature_fields::WorldFeatureFields,
    },
};

use crate::gameplay::random::next_u32;

use super::{CreatureInstance, spawn_creature_at};

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
    biome_field: Res<'w, BiomeField>,
    feature_fields: Res<'w, WorldFeatureFields>,
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
    let Some((feet, creature_id)) = find_natural_spawn(
        &context,
        dimension_definition.max_entities,
        &mut state.random_state,
    ) else {
        return;
    };
    let _ = spawn_creature_at(
        &mut commands,
        &context.definitions,
        &context.asset_server,
        context.language.get(),
        creature_id,
        feet,
    );
}

fn find_natural_spawn<'a>(
    context: &'a NaturalSpawnContext<'_, '_>,
    max_entities: usize,
    random_state: &mut u32,
) -> Option<(Vec3, &'a str)> {
    for _ in 0..NATURAL_SPAWN_ATTEMPTS {
        let Some(feet) = random_natural_spawn_position(
            &context.world,
            context.player.translation,
            random_state,
        ) else {
            continue;
        };
        let biome_id = natural_spawn_biome_id(
            &context.biome_field,
            &context.feature_fields,
            feet - Vec3::Y * 0.5,
        );
        let Some(biome) = context.biomes.get(biome_id) else {
            continue;
        };
        let Some(rule) = select_natural_spawn_rule(
            &biome.creature_spawns,
            &context.definitions,
            &context.entity_counts,
            max_entities,
            random_state,
        ) else {
            continue;
        };

        let light = context.world.light_at(feet.floor().as_ivec3());
        let light_level = light.sky().max(light.block());
        if light_level < rule.light_min || light_level > rule.light_max {
            continue;
        }
        if context.creatures.iter().any(|(instance, transform, health)| {
            !health.is_dead()
                && instance.definition_id == rule.creature
                && transform.translation.distance(feet) < rule.spacing
        }) {
            continue;
        }
        if !context.world.is_loaded_at(feet.floor().as_ivec3())
            || context.world.fluid_at(feet.floor().as_ivec3()).is_some()
        {
            continue;
        }

        return Some((feet, rule.creature.as_str()));
    }

    None
}

fn random_natural_spawn_position(
    world: &VoxelWorld,
    player_position: Vec3,
    random_state: &mut u32,
) -> Option<Vec3> {
    let angle = next_u32(random_state) as f32 / u32::MAX as f32 * std::f32::consts::TAU;
    let distance = NATURAL_SPAWN_MIN_DISTANCE
        + (next_u32(random_state) as f32 / u32::MAX as f32)
            * (NATURAL_SPAWN_MAX_DISTANCE - NATURAL_SPAWN_MIN_DISTANCE);
    let position =
        player_position + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance);
    let column = IVec2::new(position.x.floor() as i32, position.z.floor() as i32);
    let feet_y = natural_spawn_feet_y(world, column)?;

    Some(Vec3::new(
        column.x as f32 + 0.5,
        feet_y as f32,
        column.y as f32 + 0.5,
    ))
}

fn natural_spawn_biome_id<'a>(
    biome_field: &'a BiomeField,
    feature_fields: &WorldFeatureFields,
    position: Vec3,
) -> &'a str {
    let chunk_coord = chunk_coord_from_position(position);
    let region_coord = generation_region_coord(chunk_coord);
    let volume_region = feature_fields.volume_biome_region(region_coord, || {
        let (minimum, maximum) = generation_region_world_bounds(region_coord);
        biome_field.volume_region_in_bounds(minimum, maximum)
    });
    if let Some(selection) =
        biome_field.volume_selection_in_region(position, volume_region.as_ref())
    {
        return biome_field.volume_biome_id(selection);
    }

    biome_field
        .sample_surface(Vec2::new(position.x, position.z))
        .primary_id
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
