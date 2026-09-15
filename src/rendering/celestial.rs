use bevy::{
    ecs::system::SystemParam,
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::sky::CelestialBodyDefinition,
    player::camera::GameplayCamera,
    world::current_context::{DayNightContext, SkyContext},
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

#[derive(SystemParam)]
struct CelestialSpawnAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
}

#[derive(SystemParam)]
struct CelestialRuntimeScene<'w, 's> {
    day_night: DayNightContext<'w>,
    camera: Single<'w, 's, &'static GlobalTransform, With<GameplayCamera>>,
}

fn spawn_celestial_bodies(
    mut commands: Commands,
    scene: SkyContext,
    assets: CelestialSpawnAssets,
) {
    let CelestialSpawnAssets {
        mut meshes,
        mut materials,
        asset_server,
    } = assets;
    let sky = scene
        .sky()
        .expect("current dimension must reference a loaded sky definition");

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
    let mesh = meshes.add(Rectangle::new(definition.size, definition.size));
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
        NotShadowCaster,
        NotShadowReceiver,
        CelestialBody(definition.clone()),
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_celestial_bodies(
    scene: CelestialRuntimeScene,
    mut bodies: Query<(&CelestialBody, &mut Transform, &mut Visibility)>,
    mut last_camera_position: Local<Option<Vec3>>,
) {
    let camera_position = scene.camera.translation();
    let camera_changed = last_camera_position.is_none_or(|previous| previous != camera_position);
    if !camera_changed && !scene.day_night.inputs_changed() {
        return;
    }
    *last_camera_position = Some(camera_position);

    let Some(cycle) = scene.day_night.cycle() else {
        return;
    };
    let normalized_time = scene.day_night.clock().normalized_time;

    for (body, mut transform, mut visibility) in &mut bodies {
        let Some(offset) = celestial_offset(&body.0, cycle, normalized_time) else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };

        transform.translation = camera_position + offset;
        transform.look_at(camera_position, Vec3::Y);
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
    }
}
