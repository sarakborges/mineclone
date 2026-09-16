use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::camera::GameplayCamera,
    rendering::biome_visuals::CurrentBiomeVisuals,
    voxel::world::VoxelWorld,
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
    biome_visuals: CurrentBiomeVisuals,
    tint: Single<(&mut BackgroundColor, &mut Visibility), With<UnderwaterTint>>,
) {
    let eye = camera.translation;
    let voxel = IVec3::new(
        eye.x.floor() as i32,
        eye.y.floor() as i32,
        eye.z.floor() as i32,
    );
    let local_height = eye.y - voxel.y as f32;
    let (mut background, mut visibility) = tint.into_inner();

    let Some(cell) = world.fluid_at(voxel) else {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    };

    if local_height >= cell.height() {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    let entering_underwater = *visibility != Visibility::Visible;
    if entering_underwater || biome_visuals.inputs_changed() {
        let color = biome_visuals.blend_hsi(|biome| biome.visuals.underwater_tint.color);
        let opacity = biome_visuals
            .weighted_scalar(|biome| biome.visuals.underwater_tint.opacity)
            .clamp(0.0, 1.0);
        let [red, green, blue] = color.to_srgb();
        background.0 = Color::srgba(red, green, blue, opacity);
    }

    if entering_underwater {
        *visibility = Visibility::Visible;
    }
}
