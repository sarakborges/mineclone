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
    camera: Single<'w, 's, (Entity, &'static GlobalTransform), With<GameplayCamera>>,
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
    mut last_camera: Local<Option<(Entity, Vec3)>>,
) {
    let CelestialRuntimeScene { day_night, camera } = scene;
    let (camera_entity, camera_transform) = camera.into_inner();
    let camera_position = camera_transform.translation();
    let camera_changed = last_camera.as_ref().is_none_or(|(entity, previous)| {
        *entity != camera_entity || *previous != camera_position
    });
    if !camera_changed && !day_night.inputs_changed() {
        return;
    }
    if camera_changed {
        *last_camera = Some((camera_entity, camera_position));
    }

    let Some(cycle) = day_night.cycle() else {
        return;
    };
    let normalized_time = day_night.clock().normalized_time;

    for (body, mut transform, mut visibility) in &mut bodies {
        let Some(offset) = celestial_offset(&body.0, cycle, normalized_time) else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };

        let translation = camera_position + offset;
        let mut next_transform = Transform::from_translation(translation);
        next_transform.look_at(camera_position, Vec3::Y);
        if transform.translation != translation {
            transform.translation = translation;
        }
        if transform.rotation != next_transform.rotation {
            transform.rotation = next_transform.rotation;
        }
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
    }
}
