use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::biome::BiomeRegistry,
    player::camera::GameplayCamera,
    voxel::world::VoxelWorld,
    world::biome_field::BiomeField,
};

#[derive(Component)]
struct UnderwaterTint;

pub struct UnderwaterTintPlugin;

impl Plugin for UnderwaterTintPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_underwater_tint)
            .add_systems(
                Update,
                update_underwater_tint.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_underwater_tint(mut commands: Commands) {
    commands.spawn((
        UnderwaterTint,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            left: px(0),
            top: px(0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Visibility::Hidden,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_underwater_tint(
    camera: Single<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    mut tint: Single<(&mut BackgroundColor, &mut Visibility), With<UnderwaterTint>>,
) {
    let eye = camera.translation;
    let voxel = IVec3::new(
        eye.x.floor() as i32,
        eye.y.floor() as i32,
        eye.z.floor() as i32,
    );
    let local_height = eye.y - voxel.y as f32;

    let Some(cell) = world.fluid_at(voxel) else {
        *tint.1 = Visibility::Hidden;
        return;
    };

    if local_height >= cell.height() {
        *tint.1 = Visibility::Hidden;
        return;
    }

    let biome_tint = biome_field.underwater_tint(Vec2::new(eye.x, eye.z), &biomes);

    tint.0.0 = Color::srgba(
        biome_tint.color.r,
        biome_tint.color.g,
        biome_tint.color.b,
        biome_tint.opacity,
    );
    *tint.1 = Visibility::Visible;
}
