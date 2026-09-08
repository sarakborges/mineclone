use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry,
        sky::{CelestialBodyDefinition, SkyRegistry},
    },
    player::camera::GameplayCamera,
    world::{day_night::DayNightClock, dimension::CurrentDimension},
};

use super::celestial_path::celestial_offset;

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
struct CelestialBody(CelestialBodyDefinition);

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
    definition: &CelestialBodyDefinition,
) {
    let mesh = meshes.add(Circle::new(definition.size * 0.5));
    let material = materials.add(StandardMaterial {
        base_color: definition.tint.to_color(),
        base_color_texture: definition
            .texture
            .as_ref()
            .map(|texture| asset_server.load(texture.clone())),
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
        CelestialBody(definition.clone()),
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
        let Some(offset) = celestial_offset(&body.0, cycle, clock.normalized_time) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        transform.translation = offset;
        transform.look_at(camera_position, Vec3::Y);
        *visibility = Visibility::Visible;
    }
}
