use bevy::prelude::*;

use super::block::TargetedBlock;
use crate::app::game_state::GameState;

const HIGHLIGHT_SCALE: f32 = 1.002;

pub struct TargetHighlightPlugin;

impl Plugin for TargetHighlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_highlight)
            .add_systems(
                Update,
                update_highlight.run_if(in_state(GameState::Gameplay)),
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
    mut highlight: Single<(&mut Transform, &mut Visibility), With<TargetHighlight>>,
) {
    if let Some(hit) = targeted.0 {
        highlight.0.translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
        *highlight.1 = Visibility::Visible;
    } else {
        *highlight.1 = Visibility::Hidden;
    }
}
