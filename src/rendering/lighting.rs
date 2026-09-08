use bevy::{
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    voxel::chunk::CHUNK_SIZE,
    world::render_distance::RENDER_DISTANCE_RADIUS,
};

use super::environment::EnvironmentVisualState;

const SHADOW_MAP_SIZE: usize = 2048;
const SHADOW_CASCADE_COUNT: usize = 4;
const FIRST_CASCADE_DISTANCE: f32 = 48.0;
const SHADOW_DISTANCE_MULTIPLIER: f32 = 2.25;
const SHADOW_CASCADE_OVERLAP: f32 = 0.25;
const SHADOW_DEPTH_BIAS: f32 = 0.02;
const SHADOW_NORMAL_BIAS: f32 = 0.0;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DirectionalLightShadowMap {
            size: SHADOW_MAP_SIZE,
        })
        .add_systems(OnEnter(GameState::Gameplay), spawn_sky_light)
        .add_systems(
            PostUpdate,
            update_sky_light.run_if(in_state(GameState::Gameplay)),
        );
    }
}

#[derive(Component)]
struct SkyLight;

fn spawn_sky_light(mut commands: Commands) {
    let shadow_distance = RENDER_DISTANCE_RADIUS as f32
        * CHUNK_SIZE as f32
        * SHADOW_DISTANCE_MULTIPLIER;
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::NEG_Y);

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            shadow_depth_bias: SHADOW_DEPTH_BIAS,
            shadow_normal_bias: SHADOW_NORMAL_BIAS,
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
        Transform::from_rotation(rotation),
        SkyLight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_sky_light(
    visuals: Res<EnvironmentVisualState>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    mut sky_light: Single<&mut DirectionalLight, With<SkyLight>>,
) {
    ambient_light.brightness = 0.0;
    sky_light.color = visuals.light_color;
    sky_light.illuminance = visuals.light_illuminance;
}
