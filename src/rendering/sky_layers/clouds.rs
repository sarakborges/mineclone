use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

use crate::{app::game_state::GameState, player::camera::GameplayCamera};

use super::state::SkyLayerVisualState;

const MAX_CLOUDS: usize = 24;
const CLOUD_PARTS: usize = 3;
const CLOUD_SPAN: f32 = 180.0;
const CLOUD_SPEED: f32 = 1.6;

#[derive(Resource)]
pub(super) struct CloudAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub(super) struct CloudPart {
    cloud_index: usize,
    base: Vec2,
    altitude: f32,
    offset: Vec3,
    scale: Vec3,
}

pub(super) fn setup_cloud_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.78),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        ..default()
    });

    commands.insert_resource(CloudAssets { mesh, material });
}

pub(super) fn spawn_clouds(mut commands: Commands, assets: Res<CloudAssets>) {
    for cloud_index in 0..MAX_CLOUDS {
        let seed = cloud_index as u32;
        let base = Vec2::new(
            hash_signed(seed.wrapping_mul(17).wrapping_add(3)) * CLOUD_SPAN * 0.5,
            hash_signed(seed.wrapping_mul(29).wrapping_add(11)) * CLOUD_SPAN * 0.5,
        );
        let altitude = 34.0 + hash01(seed.wrapping_mul(37).wrapping_add(5)) * 14.0;

        for part_index in 0..CLOUD_PARTS {
            let (offset, scale) = cloud_part_shape(seed, part_index);
            commands.spawn((
                Mesh3d(assets.mesh.clone()),
                MeshMaterial3d(assets.material.clone()),
                Transform::default(),
                Visibility::Hidden,
                NotShadowCaster,
                NotShadowReceiver,
                CloudPart {
                    cloud_index,
                    base,
                    altitude,
                    offset,
                    scale,
                },
                DespawnOnExit(GameState::Gameplay),
            ));
        }
    }
}

pub(super) fn update_clouds(
    time: Res<Time>,
    visuals: Res<SkyLayerVisualState>,
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    assets: Res<CloudAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clouds: Query<(&CloudPart, &mut Transform, &mut Visibility)>,
) {
    let visible_count = (visuals.cloud_density * MAX_CLOUDS as f32).round() as usize;
    let camera_position = camera.translation();
    let drift = time.elapsed_secs() * CLOUD_SPEED;
    let half_span = CLOUD_SPAN * 0.5;

    if let Some(material) = materials.get_mut(&assets.material) {
        let color = visuals.cloud_color;
        material.base_color = Color::srgba(color.r, color.g, color.b, 0.78);
    }

    for (cloud, mut transform, mut visibility) in &mut clouds {
        if cloud.cloud_index >= visible_count || visuals.cloud_density <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }

        let local_x = (cloud.base.x + drift + half_span).rem_euclid(CLOUD_SPAN) - half_span;
        let local_z = cloud.base.y;
        transform.translation = Vec3::new(
            camera_position.x + local_x,
            cloud.altitude,
            camera_position.z + local_z,
        ) + cloud.offset;
        transform.scale = cloud.scale;
        *visibility = Visibility::Visible;
    }
}

fn cloud_part_shape(seed: u32, part_index: usize) -> (Vec3, Vec3) {
    let width = 9.0 + hash01(seed.wrapping_mul(43).wrapping_add(7)) * 8.0;
    let depth = 4.0 + hash01(seed.wrapping_mul(53).wrapping_add(13)) * 5.0;

    match part_index {
        0 => (Vec3::ZERO, Vec3::new(width, 0.7, depth)),
        1 => (
            Vec3::new(width * 0.28, 0.15, depth * 0.35),
            Vec3::new(width * 0.55, 0.7, depth * 0.75),
        ),
        _ => (
            Vec3::new(-width * 0.32, -0.05, -depth * 0.28),
            Vec3::new(width * 0.42, 0.7, depth * 0.62),
        ),
    }
}

fn hash01(mut value: u32) -> f32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
}

fn hash_signed(value: u32) -> f32 {
    hash01(value) * 2.0 - 1.0
}
