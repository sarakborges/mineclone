use bevy::{
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::{
        day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry,
        sky::SkyRegistry,
    },
    voxel::chunk::CHUNK_SIZE,
    world::{
        day_night::DayNightClock,
        dimension::CurrentDimension,
        render_distance::RENDER_DISTANCE_RADIUS,
    },
};

use super::{
    celestial_path::celestial_direction,
    environment::EnvironmentVisualState,
};

const SHADOW_MAP_SIZE: usize = 2048;
const SHADOW_CASCADE_COUNT: usize = 4;
const FIRST_CASCADE_DISTANCE: f32 = 48.0;
const SHADOW_DISTANCE_MULTIPLIER: f32 = 2.25;
const SHADOW_CASCADE_OVERLAP: f32 = 0.25;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DirectionalLightShadowMap {
            size: SHADOW_MAP_SIZE,
        })
        .add_systems(OnEnter(GameState::Gameplay), spawn_sun)
        .add_systems(
            PostUpdate,
            update_lighting.run_if(in_state(GameState::Gameplay)),
        );
    }
}

#[derive(Component)]
struct SunLight;

fn spawn_sun(mut commands: Commands) {
    let shadow_distance = RENDER_DISTANCE_RADIUS as f32
        * CHUNK_SIZE as f32
        * SHADOW_DISTANCE_MULTIPLIER;

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.20,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: SHADOW_CASCADE_COUNT,
            maximum_distance: shadow_distance,
            first_cascade_far_bound: FIRST_CASCADE_DISTANCE,
            overlap_proportion: SHADOW_CASCADE_OVERLAP,
            ..default()
        }
        .build(),
        Transform::default(),
        SunLight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_lighting(
    visuals: Res<EnvironmentVisualState>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    skies: Res<SkyRegistry>,
    clock: Res<DayNightClock>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    mut sun: Single<(&mut DirectionalLight, &mut Transform), With<SunLight>>,
) {
    ambient_light.color = visuals.ambient_color;
    ambient_light.brightness = visuals.ambient_brightness;

    sun.0.color = visuals.sun_color;

    let Some(dimension) = dimensions.get(&current_dimension.id) else {
        sun.0.illuminance = 0.0;
        return;
    };
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        sun.0.illuminance = 0.0;
        return;
    };
    let Some(sky) = skies.get(&dimension.sky) else {
        sun.0.illuminance = 0.0;
        return;
    };
    let Some(direction_to_sun) = celestial_direction(&sky.sun, cycle, clock.normalized_time) else {
        sun.0.illuminance = 0.0;
        return;
    };

    sun.0.illuminance = visuals.sun_illuminance
        * horizon_light_factor(direction_to_sun, sky.sun.light_fade_altitude_degrees);

    let light_direction = -direction_to_sun;
    sun.1.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, light_direction);
}

fn horizon_light_factor(direction_to_sun: Vec3, fade_altitude_degrees: f32) -> f32 {
    if fade_altitude_degrees <= 0.0 {
        return 1.0;
    }

    let fade_height = fade_altitude_degrees.to_radians().sin();
    if fade_height <= f32::EPSILON {
        return 1.0;
    }

    let t = (direction_to_sun.y.max(0.0) / fade_height).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
