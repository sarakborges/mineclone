use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{dimension::DimensionRegistry, sky::SkyRegistry},
    player::camera::GameplayCamera,
    world::{day_night::DayNightClock, dimension::CurrentDimension},
};

pub struct CelestialPlugin;

impl Plugin for CelestialPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_celestial_bodies)
            .add_systems(
                PostUpdate,
                update_celestial_bodies.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct CelestialBody {
    orbit_radius: f32,
    phase_offset_radians: f32,
    orbit_tilt_radians: f32,
}

fn spawn_celestial_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    skies: Res<SkyRegistry>,
) {
    let dimension = dimensions
        .get(&current_dimension.id)
        .unwrap_or_else(|| panic!("missing dimension definition: {}", current_dimension.id));
    let sky = skies
        .get(&dimension.sky)
        .unwrap_or_else(|| panic!("missing sky definition: {}", dimension.sky));

    spawn_body(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        &sky.sun,
        sky.orbit_tilt_degrees,
    );
    spawn_body(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        &sky.moon,
        sky.orbit_tilt_degrees,
    );
}

fn spawn_body(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    definition: &crate::content::sky::CelestialBodyDefinition,
    orbit_tilt_degrees: f32,
) {
    let mesh = meshes.add(Rectangle::new(definition.size, definition.size));
    let material = materials.add(StandardMaterial {
        base_color: definition.tint.to_color(),
        base_color_texture: Some(asset_server.load(definition.texture.clone())),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::default(),
        Visibility::Hidden,
        CelestialBody {
            orbit_radius: definition.orbit_radius,
            phase_offset_radians: definition.phase_offset_degrees.to_radians(),
            orbit_tilt_radians: orbit_tilt_degrees.to_radians(),
        },
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_celestial_bodies(
    clock: Res<DayNightClock>,
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    mut bodies: Query<(&CelestialBody, &mut Transform, &mut Visibility)>,
) {
    let camera_position = camera.translation();

    for (body, mut transform, mut visibility) in &mut bodies {
        let angle = clock.normalized_time * std::f32::consts::TAU + body.phase_offset_radians;
        let orbit_position = Vec3::new(
            angle.cos() * body.orbit_radius,
            angle.sin() * body.orbit_radius,
            0.0,
        );
        let offset = Quat::from_rotation_x(body.orbit_tilt_radians) * orbit_position;

        transform.translation = camera_position + offset;
        transform.look_at(camera_position, Vec3::Y);
        *visibility = if offset.y > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
