use bevy::prelude::*;

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
};
use crate::{
    app::game_state::GameState,
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    voxel::world::VoxelWorld,
};

const HIGHLIGHT_SCALE: f32 = 1.01;

type HighlightTarget<'w, 's> = Single<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (With<TargetHighlight>, Without<GameplayCamera>),
>;

pub struct TargetHighlightPlugin;

impl Plugin for TargetHighlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_highlight)
            .add_systems(
                Update,
                update_highlight
                    .in_set(BlockTargetingSet::Visuals)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct TargetHighlight;

fn spawn_highlight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(HIGHLIGHT_SCALE)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.18),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        TargetHighlight,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_highlight(
    targeted: Res<TargetedBlock>,
    hotbar: Res<PlayerHotbar>,
    world: Res<VoxelWorld>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut highlight: HighlightTarget,
) {
    let Some(hit) = targeted.0 else {
        *highlight.1 = Visibility::Hidden;
        return;
    };

    let placement_preview_visible = hotbar.item_at(hotbar.selected_slot()).is_some()
        && placement_voxel(hit, &world, player.translation).is_some();

    if placement_preview_visible {
        *highlight.1 = Visibility::Hidden;
        return;
    }

    highlight.0.translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
    *highlight.1 = Visibility::Visible;
}
