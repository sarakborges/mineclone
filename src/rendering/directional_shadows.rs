use bevy::{
    ecs::system::SystemParam,
    light::{CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::{
        day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry, sky::SkyRegistry,
    },
    voxel::chunk::CHUNK_SIZE,
    world::{
        day_night::DayNightClock, dimension::CurrentDimension,
        render_distance::RenderDistanceSettings,
    },
};

use super::celestial_path::celestial_direction;

const SHADOW_MAP_SIZE: usize = 1024;
const SHADOW_CASCADES: usize = 3;
const SHADOW_DISTANCE_CHUNK_CAP: i32 = 8;
const FIRST_CASCADE_FAR_BOUND: f32 = CHUNK_SIZE as f32;
const SHADOW_DEPTH_BIAS: f32 = 0.02;
const SHADOW_NORMAL_BIAS: f32 = 0.8;
const BASE_SUN_ILLUMINANCE: f32 = 10_000.0;

type SunShadowLights<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut DirectionalLight,
        &'static mut CascadeShadowConfig,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    With<SunShadowLight>,
>;

pub struct DirectionalShadowsPlugin;

impl Plugin for DirectionalShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DirectionalLightShadowMap {
            size: SHADOW_MAP_SIZE,
        })
        .add_systems(OnEnter(GameState::Gameplay), spawn_sun_shadow_light)
        .add_systems(
            Update,
            update_sun_shadow_light.run_if(in_state(GameState::Gameplay)),
        );
    }
}

#[derive(Component)]
struct SunShadowLight;

#[derive(SystemParam)]
struct SunShadowScene<'w> {
    current_dimension: Res<'w, CurrentDimension>,
    dimensions: Res<'w, DimensionRegistry>,
    skies: Res<'w, SkyRegistry>,
    cycles: Res<'w, DayNightCycleRegistry>,
    clock: Res<'w, DayNightClock>,
}

fn spawn_sun_shadow_light(
    mut commands: Commands,
    render_distance: Res<RenderDistanceSettings>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 0.0,
            shadow_maps_enabled: true,
            shadow_depth_bias: SHADOW_DEPTH_BIAS,
            shadow_normal_bias: SHADOW_NORMAL_BIAS,
            ..default()
        },
        shadow_config(render_distance.chunks()),
        Transform::default(),
        Visibility::Hidden,
        SunShadowLight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_sun_shadow_light(
    scene: SunShadowScene,
    render_distance: Res<RenderDistanceSettings>,
    mut lights: SunShadowLights,
) {
    if render_distance.is_changed() {
        let config = shadow_config(render_distance.chunks());
        for (_, mut cascades, _, _) in &mut lights {
            *cascades = config.clone();
        }
    }

    let Some(dimension) = scene.dimensions.get(&scene.current_dimension.id) else {
        hide_lights(&mut lights);
        return;
    };
    let Some(sky) = scene.skies.get(&dimension.sky) else {
        hide_lights(&mut lights);
        return;
    };
    let Some(cycle) = scene.cycles.get(&dimension.day_night_cycle) else {
        hide_lights(&mut lights);
        return;
    };
    let Some(sun_direction) = celestial_direction(&sky.sun, cycle, scene.clock.normalized_time) else {
        hide_lights(&mut lights);
        return;
    };

    let sample = cycle.sample(scene.clock.normalized_time);
    let rotation = shadow_light_rotation(sun_direction);

    for (mut light, _, mut transform, mut visibility) in &mut lights {
        light.color = sky.sun.tint.to_color();
        light.illuminance = BASE_SUN_ILLUMINANCE * sample.sky_light_factor;
        transform.rotation = rotation;
        *visibility = Visibility::Visible;
    }
}

fn hide_lights(lights: &mut SunShadowLights) {
    for (mut light, _, _, mut visibility) in lights.iter_mut() {
        light.illuminance = 0.0;
        *visibility = Visibility::Hidden;
    }
}

fn shadow_config(horizontal_chunks: i32) -> CascadeShadowConfig {
    let shadow_chunks = horizontal_chunks.clamp(1, SHADOW_DISTANCE_CHUNK_CAP);
    let maximum_distance = ((shadow_chunks + 1) * CHUNK_SIZE as i32) as f32;

    CascadeShadowConfigBuilder {
        num_cascades: SHADOW_CASCADES,
        first_cascade_far_bound: FIRST_CASCADE_FAR_BOUND,
        maximum_distance,
        ..default()
    }
    .build()
}

fn shadow_light_rotation(sun_direction: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::NEG_Z, -sun_direction.normalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_light_points_from_sun_toward_world() {
        let sun_direction = Vec3::new(0.4, 0.8, -0.2).normalize();
        let rotation = shadow_light_rotation(sun_direction);
        let light_forward = rotation * Vec3::NEG_Z;

        assert!((light_forward + sun_direction).length() <= 0.0001);
    }

    #[test]
    fn directional_shadow_budget_caps_distance_and_uses_three_cascades() {
        let config = shadow_config(12);

        assert_eq!(config.bounds.len(), SHADOW_CASCADES);
        assert_eq!(
            config.bounds.last().copied(),
            Some((SHADOW_DISTANCE_CHUNK_CAP + 1) as f32 * CHUNK_SIZE as f32),
        );
    }
}
