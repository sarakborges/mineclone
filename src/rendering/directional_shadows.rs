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
        render_distance::MAX_RENDER_DISTANCE_CHUNKS,
    },
};

use super::celestial_path::celestial_direction;

const SHADOW_MAP_SIZE: usize = 2048;
const FIRST_CASCADE_FAR_BOUND: f32 = CHUNK_SIZE as f32;
const MAXIMUM_SHADOW_DISTANCE: f32 = ((MAX_RENDER_DISTANCE_CHUNKS + 1) * CHUNK_SIZE as i32) as f32;
const BASE_SUN_ILLUMINANCE: f32 = 10_000.0;

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

fn spawn_sun_shadow_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 0.0,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            first_cascade_far_bound: FIRST_CASCADE_FAR_BOUND,
            maximum_distance: MAXIMUM_SHADOW_DISTANCE,
            ..default()
        }
        .build(),
        Transform::default(),
        Visibility::Hidden,
        SunShadowLight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_sun_shadow_light(
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    skies: Res<SkyRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    clock: Res<DayNightClock>,
    mut lights: Query<
        (&mut DirectionalLight, &mut Transform, &mut Visibility),
        With<SunShadowLight>,
    >,
) {
    let Some(dimension) = dimensions.get(&current_dimension.id) else {
        hide_lights(&mut lights);
        return;
    };
    let Some(sky) = skies.get(&dimension.sky) else {
        hide_lights(&mut lights);
        return;
    };
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        hide_lights(&mut lights);
        return;
    };
    let Some(sun_direction) = celestial_direction(&sky.sun, cycle, clock.normalized_time) else {
        hide_lights(&mut lights);
        return;
    };

    let sample = cycle.sample(clock.normalized_time);
    let rotation = shadow_light_rotation(sun_direction);

    for (mut light, mut transform, mut visibility) in &mut lights {
        light.color = sky.sun.tint.to_color();
        light.illuminance = BASE_SUN_ILLUMINANCE * sample.sky_light_factor;
        transform.rotation = rotation;
        *visibility = Visibility::Visible;
    }
}

fn hide_lights(
    lights: &mut Query<
        (&mut DirectionalLight, &mut Transform, &mut Visibility),
        With<SunShadowLight>,
    >,
) {
    for (mut light, _, mut visibility) in lights.iter_mut() {
        light.illuminance = 0.0;
        *visibility = Visibility::Hidden;
    }
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
}
