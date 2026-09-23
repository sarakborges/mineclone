use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    world::current_context::SkyDayNightContext,
};

use super::celestial_path::celestial_direction;

const BASE_SUN_ILLUMINANCE: f32 = 10_000.0;

type SunLights<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut DirectionalLight,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    With<SunLight>,
>;

pub struct SunLightingPlugin;

impl Plugin for SunLightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_sun_light)
            .add_systems(
                Update,
                update_sun_light.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct SunLight;

fn sun_directional_light() -> DirectionalLight {
    DirectionalLight {
        illuminance: 0.0,
        // Minecraft-style terrain lighting is authored by propagated voxel
        // light, per-vertex AO, and fixed face shading. Keep the directional
        // light only for lit 3D entities and never allocate/project shadow maps.
        shadow_maps_enabled: false,
        ..default()
    }
}

fn spawn_sun_light(mut commands: Commands) {
    commands.spawn((
        sun_directional_light(),
        Transform::default(),
        Visibility::Hidden,
        SunLight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_sun_light(
    scene: SkyDayNightContext,
    mut lights: SunLights,
) {
    if !scene.inputs_changed() {
        return;
    }

    let (Some(sky), Some(cycle), Some(sample)) = (scene.sky(), scene.cycle(), scene.sample()) else {
        hide_lights(&mut lights);
        return;
    };
    let Some(sun_direction) = celestial_direction(&sky.sun, cycle, scene.clock().normalized_time)
    else {
        hide_lights(&mut lights);
        return;
    };

    let rotation = sun_light_rotation(sun_direction);
    let color = sky.sun.tint.to_color();
    let illuminance = BASE_SUN_ILLUMINANCE * sample.sky_light_factor;

    for (mut light, mut transform, mut visibility) in &mut lights {
        if light.color != color {
            light.color = color;
        }
        if light.illuminance != illuminance {
            light.illuminance = illuminance;
        }
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
    }
}

fn hide_lights(lights: &mut SunLights) {
    for (mut light, _, mut visibility) in lights.iter_mut() {
        if light.illuminance != 0.0 {
            light.illuminance = 0.0;
        }
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    }
}

fn sun_light_rotation(sun_direction: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::NEG_Z, -sun_direction.normalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_light_points_from_sun_toward_world() {
        let sun_direction = Vec3::new(0.4, 0.8, -0.2).normalize();
        let rotation = sun_light_rotation(sun_direction);
        let light_forward = rotation * Vec3::NEG_Z;

        assert!((light_forward + sun_direction).length() <= 0.0001);
    }

    #[test]
    fn sun_light_never_projects_shadow_maps() {
        assert!(!sun_directional_light().shadow_maps_enabled);
    }
}
