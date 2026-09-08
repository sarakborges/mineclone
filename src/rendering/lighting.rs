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
    world::{day_night::DayNightClock, dimension::CurrentDimension},
};

use super::{
    celestial_path::celestial_direction,
    environment::EnvironmentVisualState,
};

const SHADOW_MAP_SIZE: usize = 4096;
const SHADOW_DISTANCE: f32 = 320.0;

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
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.20,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: SHADOW_DISTANCE,
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
    sun.0.illuminance = visuals.sun_illuminance;

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

    let light_direction = -direction_to_sun;
    sun.1.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, light_direction);
}
