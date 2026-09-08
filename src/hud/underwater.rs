use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::fluid::FluidRegistry,
    player::camera::GameplayCamera,
    voxel::world::VoxelWorld,
};

const SUBMERGED_TINT_OPACITY: f32 = 0.20;

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
    fluids: Res<FluidRegistry>,
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

    let fluid = fluids
        .get(cell.fluid_id)
        .unwrap_or_else(|| panic!("missing fluid definition for id {}", cell.fluid_id));

    tint.0.0 = Color::srgba(
        fluid.color.r,
        fluid.color.g,
        fluid.color.b,
        SUBMERGED_TINT_OPACITY,
    );
    *tint.1 = Visibility::Visible;
}
