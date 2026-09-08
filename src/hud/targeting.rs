use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    targeting::block::TargetedBlock,
    ui::{surface, typography},
};

pub struct TargetHudPlugin;

impl Plugin for TargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_target_hud)
            .add_systems(Update, update_target_hud.run_if(in_state(GameState::Gameplay)));
    }
}

#[derive(Component)]
struct TargetHudRoot;

#[derive(Component)]
struct TargetBlockText;

fn spawn_target_hud(mut commands: Commands) {
    commands
        .spawn((
            TargetHudRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                right: px(16),
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn(surface::hud_panel()).with_children(|panel| {
                panel.spawn((typography::hud(""), TargetBlockText));
            });
        });
}

fn update_target_hud(
    targeted: Res<TargetedBlock>,
    blocks: Res<BlockRegistry>,
    root_visibility: Single<&mut Visibility, With<TargetHudRoot>>,
    mut target_text: Single<&mut Text, With<TargetBlockText>>,
) {
    let mut root_visibility = root_visibility.into_inner();

    let Some(hit) = targeted.0 else {
        *root_visibility = Visibility::Hidden;
        return;
    };

    *root_visibility = Visibility::Visible;

    let block_name = blocks
        .get(hit.block_id)
        .map_or(hit.block_id, |block| block.name.as_str());

    target_text.0 = format!(
        "Block: {block_name}\nPosition: X: {} | Y: {} | Z: {}",
        hit.voxel.x, hit.voxel.y, hit.voxel.z
    );
}
