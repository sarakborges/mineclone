use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    targeting::block::TargetedBlock,
};

pub struct TargetHudPlugin;

impl Plugin for TargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_target_hud)
            .add_systems(Update, update_target_hud.run_if(in_state(GameState::Gameplay)));
    }
}

#[derive(Component)]
struct TargetBlockText;

fn spawn_target_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            right: px(16),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.82)),
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Text::new("Block: -\nPosition: -"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TargetBlockText,
        )],
    ));
}

fn update_target_hud(
    targeted: Res<TargetedBlock>,
    blocks: Res<BlockRegistry>,
    mut target_text: Single<&mut Text, With<TargetBlockText>>,
) {
    target_text.0 = targeted.0.map_or_else(
        || "Block: -\nPosition: -".to_string(),
        |hit| {
            let block_name = blocks
                .get(hit.block_id)
                .map_or(hit.block_id, |block| block.name.as_str());

            format!(
                "Block: {block_name}\nPosition: X {} | Z {} | Y {}",
                hit.voxel.x, hit.voxel.z, hit.voxel.y
            )
        },
    );
}
