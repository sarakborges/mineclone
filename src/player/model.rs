use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{
        PLAYER_EYE_HEIGHT,
        camera::{CameraPerspective, GameplayCamera},
    },
};

const MODEL_PIXEL: f32 = 0.9 / 16.0;
const HEAD_SIZE: Vec3 = Vec3::new(8.0, 8.0, 8.0);
const TORSO_SIZE: Vec3 = Vec3::new(8.0, 12.0, 4.0);
const ARM_SIZE: Vec3 = Vec3::new(4.0, 12.0, 4.0);
const LEG_SIZE: Vec3 = Vec3::new(4.0, 12.0, 4.0);

#[derive(Component)]
struct PlayerModelRoot;

#[derive(Component)]
struct PlayerModelHead;

#[derive(Resource)]
struct PlayerModelAssets {
    head: Handle<Mesh>,
    torso: Handle<Mesh>,
    arm: Handle<Mesh>,
    leg: Handle<Mesh>,
    skin: Handle<StandardMaterial>,
    shirt: Handle<StandardMaterial>,
    pants: Handle<StandardMaterial>,
}

pub(crate) struct PlayerModelPlugin;

impl Plugin for PlayerModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player_model_assets)
            .add_systems(OnEnter(GameState::Gameplay), spawn_player_model)
            .add_systems(
                Update,
                sync_player_model.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn setup_player_model_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scaled = |size: Vec3| size * MODEL_PIXEL;
    commands.insert_resource(PlayerModelAssets {
        head: meshes.add(Cuboid::new(
            scaled(HEAD_SIZE).x,
            scaled(HEAD_SIZE).y,
            scaled(HEAD_SIZE).z,
        )),
        torso: meshes.add(Cuboid::new(
            scaled(TORSO_SIZE).x,
            scaled(TORSO_SIZE).y,
            scaled(TORSO_SIZE).z,
        )),
        arm: meshes.add(Cuboid::new(
            scaled(ARM_SIZE).x,
            scaled(ARM_SIZE).y,
            scaled(ARM_SIZE).z,
        )),
        leg: meshes.add(Cuboid::new(
            scaled(LEG_SIZE).x,
            scaled(LEG_SIZE).y,
            scaled(LEG_SIZE).z,
        )),
        skin: materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.50, 0.38),
            perceptual_roughness: 1.0,
            ..default()
        }),
        shirt: materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.52, 0.55),
            perceptual_roughness: 1.0,
            ..default()
        }),
        pants: materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.22, 0.52),
            perceptual_roughness: 1.0,
            ..default()
        }),
    });
}

fn spawn_player_model(
    mut commands: Commands,
    assets: Res<PlayerModelAssets>,
) {
    let pixel = MODEL_PIXEL;
    let leg_height = LEG_SIZE.y * pixel;
    let torso_height = TORSO_SIZE.y * pixel;
    let head_height = HEAD_SIZE.y * pixel;
    let torso_width = TORSO_SIZE.x * pixel;
    let arm_width = ARM_SIZE.x * pixel;
    let leg_width = LEG_SIZE.x * pixel;

    commands
        .spawn((
            Name::new("Player Model"),
            PlayerModelRoot,
            Transform::default(),
            Visibility::Hidden,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                Mesh3d(assets.leg.clone()),
                MeshMaterial3d(assets.pants.clone()),
                Transform::from_translation(Vec3::new(
                    -leg_width * 0.5,
                    leg_height * 0.5,
                    0.0,
                )),
            ));
            root.spawn((
                Mesh3d(assets.leg.clone()),
                MeshMaterial3d(assets.pants.clone()),
                Transform::from_translation(Vec3::new(
                    leg_width * 0.5,
                    leg_height * 0.5,
                    0.0,
                )),
            ));
            root.spawn((
                Mesh3d(assets.torso.clone()),
                MeshMaterial3d(assets.shirt.clone()),
                Transform::from_translation(Vec3::new(
                    0.0,
                    leg_height + torso_height * 0.5,
                    0.0,
                )),
            ));
            root.spawn((
                Mesh3d(assets.arm.clone()),
                MeshMaterial3d(assets.skin.clone()),
                Transform::from_translation(Vec3::new(
                    -(torso_width * 0.5 + arm_width * 0.5),
                    leg_height + torso_height * 0.5,
                    0.0,
                )),
            ));
            root.spawn((
                Mesh3d(assets.arm.clone()),
                MeshMaterial3d(assets.skin.clone()),
                Transform::from_translation(Vec3::new(
                    torso_width * 0.5 + arm_width * 0.5,
                    leg_height + torso_height * 0.5,
                    0.0,
                )),
            ));
            root.spawn((
                PlayerModelHead,
                Mesh3d(assets.head.clone()),
                MeshMaterial3d(assets.skin.clone()),
                Transform::from_translation(Vec3::new(
                    0.0,
                    leg_height + torso_height + head_height * 0.5,
                    0.0,
                )),
            ));
        });
}

fn sync_player_model(
    perspective: Res<CameraPerspective>,
    player: Single<(&Transform, &GameplayCamera)>,
    mut model: Single<
        (&mut Transform, &mut Visibility),
        (With<PlayerModelRoot>, Without<GameplayCamera>),
    >,
    mut head: Single<
        &mut Transform,
        (
            With<PlayerModelHead>,
            Without<PlayerModelRoot>,
            Without<GameplayCamera>,
        ),
    >,
) {
    let (player_transform, camera) = *player;
    let (model_transform, visibility) = &mut *model;

    let next_visibility = if perspective.is_third_person() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if **visibility != next_visibility {
        **visibility = next_visibility;
    }

    model_transform.translation =
        player_transform.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    model_transform.rotation = Quat::from_rotation_y(camera.yaw);

    head.rotation = Quat::from_rotation_x(camera.pitch);
}
