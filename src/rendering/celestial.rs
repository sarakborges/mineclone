use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        day_night_cycle::DayNightCycleRegistry,
        day_night_phase::DayNightPhase,
        dimension::DimensionRegistry,
        sky::SkyRegistry,
    },
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
    rise_phase: DayNightPhase,
    set_phase: DayNightPhase,
    rise_azimuth_radians: f32,
    set_azimuth_radians: f32,
    max_altitude_radians: f32,
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
    );
    spawn_body(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        &sky.moon,
    );
}

fn spawn_body(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    definition: &crate::content::sky::CelestialBodyDefinition,
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
            rise_phase: definition.rise_phase,
            set_phase: definition.set_phase,
            rise_azimuth_radians: definition.rise_azimuth_degrees.to_radians(),
            set_azimuth_radians: definition.set_azimuth_degrees.to_radians(),
            max_altitude_radians: definition.max_altitude_degrees.to_radians(),
        },
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_celestial_bodies(
    clock: Res<DayNightClock>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    mut bodies: Query<(&CelestialBody, &mut Transform, &mut Visibility)>,
) {
    let Some(dimension) = dimensions.get(&current_dimension.id) else {
        return;
    };
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        return;
    };
    let camera_position = camera.translation();

    for (body, mut transform, mut visibility) in &mut bodies {
        let Some(progress) = cycle.progress_between_phases(
            clock.normalized_time,
            body.rise_phase,
            body.set_phase,
        ) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let azimuth = lerp_angle(
            body.rise_azimuth_radians,
            body.set_azimuth_radians,
            progress,
        );
        let altitude = (progress * std::f32::consts::PI).sin() * body.max_altitude_radians;
        let horizontal_radius = body.orbit_radius * altitude.cos();
        let offset = Vec3::new(
            azimuth.sin() * horizontal_radius,
            altitude.sin() * body.orbit_radius,
            -azimuth.cos() * horizontal_radius,
        );

        transform.translation = camera_position + offset;
        transform.look_at(camera_position, Vec3::Y);
        *visibility = Visibility::Visible;
    }
}

fn lerp_angle(start: f32, end: f32, t: f32) -> f32 {
    let delta = (end - start + std::f32::consts::PI)
        .rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    start + delta * t
}
