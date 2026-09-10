use bevy::prelude::*;

use crate::{
    app::game_state::GameState, content::biome::BiomeRegistry, player::camera::GameplayCamera,
    voxel::world::VoxelWorld, world::biome::CurrentBiome,
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
        GlobalZIndex(-10),
        Pickable::IGNORE,
        Visibility::Hidden,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_underwater_tint(
    camera: Single<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    biomes: Res<BiomeRegistry>,
    current_biome: Res<CurrentBiome>,
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

    let mut red = 0.0;
    let mut green = 0.0;
    let mut blue = 0.0;
    let mut opacity = 0.0;

    for influence in &current_biome.influences {
        let Some(biome) = biomes.get(&influence.id) else {
            continue;
        };
        let biome_tint = biome.visuals.underwater_tint;

        red += biome_tint.color.r * influence.weight;
        green += biome_tint.color.g * influence.weight;
        blue += biome_tint.color.b * influence.weight;
        opacity += biome_tint.opacity * influence.weight;
    }

    tint.0.0 = Color::srgba(red, green, blue, opacity);
    *tint.1 = Visibility::Visible;
}
