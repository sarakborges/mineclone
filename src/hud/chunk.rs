use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    voxel::coordinates::split_dimension_position,
};

pub struct ChunkHudPlugin;

impl Plugin for ChunkHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_chunk_hud)
            .add_systems(Update, update_chunk_hud.run_if(in_state(GameState::Gameplay)));
    }
}

#[derive(Component)]
struct ChunkHudText;

fn spawn_chunk_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.82)),
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Text::new("Chunk: X 0 | Z 0 | Y 0\nLocal: X 0 | Z 0 | Y 0"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            ChunkHudText,
        )],
    ));
}

fn update_chunk_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    mut chunk_text: Single<&mut Text, With<ChunkHudText>>,
) {
    let position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let coordinates = split_dimension_position(position);
    let local_position = coordinates.local.floor().as_ivec3();

    chunk_text.0 = format!(
        "Chunk: X {} | Z {} | Y {}\nLocal: X {} | Z {} | Y {}",
        coordinates.chunk.x,
        coordinates.chunk.z,
        coordinates.chunk.y,
        local_position.x,
        local_position.z,
        local_position.y,
    );
}
