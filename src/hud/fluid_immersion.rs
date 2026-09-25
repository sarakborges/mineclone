use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::fluid::FluidRegistry,
    player::camera::GameplayCamera,
    rendering::biome_visuals::CurrentBiomeVisuals,
    voxel::world::VoxelWorld,
};

#[derive(Component)]
struct FluidImmersionTintOverlay;

pub struct FluidImmersionTintPlugin;

impl Plugin for FluidImmersionTintPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_fluid_immersion_tint)
            .add_systems(
                Update,
                update_fluid_immersion_tint.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_fluid_immersion_tint(mut commands: Commands) {
    commands.spawn((
        FluidImmersionTintOverlay,
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

fn update_fluid_immersion_tint(
    camera: Single<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    fluids: Res<FluidRegistry>,
    biome_visuals: CurrentBiomeVisuals,
    tint: Single<(&mut BackgroundColor, &mut Visibility), With<FluidImmersionTintOverlay>>,
    mut active_fluid: Local<Option<crate::content::fluid::FluidId>>,
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
        hide_immersion_tint(&mut visibility, &mut active_fluid);
        return;
    };

    if local_height >= cell.height() {
        hide_immersion_tint(&mut visibility, &mut active_fluid);
        return;
    }

    let entering_fluid = *visibility != Visibility::Visible;
    let fluid_changed = *active_fluid != Some(cell.fluid_id);
    if entering_fluid || fluid_changed || biome_visuals.inputs_changed() || fluids.is_changed() {
        let definition = fluids
            .get(cell.fluid_id)
            .unwrap_or_else(|| panic!("missing fluid definition for id {}", cell.fluid_id));
        let (color, opacity) = if let Some(tint) = definition.immersion_tint {
            (tint.color, tint.opacity)
        } else {
            (
                biome_visuals.blend_hsi(|biome| biome.visuals().underwater_tint.color),
                biome_visuals
                    .weighted_scalar(|biome| biome.visuals().underwater_tint.opacity)
                    .clamp(0.0, 1.0),
            )
        };
        let [red, green, blue] = color.to_srgb();
        background.0 = Color::srgba(red, green, blue, opacity);
    }

    *active_fluid = Some(cell.fluid_id);
    if entering_fluid {
        *visibility = Visibility::Visible;
    }
}

fn hide_immersion_tint(
    visibility: &mut Visibility,
    active_fluid: &mut Option<crate::content::fluid::FluidId>,
) {
    *active_fluid = None;
    if *visibility != Visibility::Hidden {
        *visibility = Visibility::Hidden;
    }
}
