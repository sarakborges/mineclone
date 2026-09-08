use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    world::{biome::CurrentBiome, dimension::CurrentDimension},
};

pub struct WorldHudPlugin;

impl Plugin for WorldHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_world_hud)
            .add_systems(Update, update_world_hud.run_if(in_state(GameState::Gameplay)));
    }
}

#[derive(Component)]
struct WorldHudText;

fn spawn_world_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: percent(50),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-0.5, 0.0, 0.0)),
        BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.82)),
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Text::new("- - -\nX 0 | Z 0 | Y 0"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::new_with_justify(Justify::Center),
            WorldHudText,
        )],
    ));
}

fn update_world_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    dimension: Res<CurrentDimension>,
    biome: Res<CurrentBiome>,
    mut world_text: Single<&mut Text, With<WorldHudText>>,
) {
    let position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let block_position = position.floor().as_ivec3();

    world_text.0 = format!(
        "{} - {}\nX {} | Z {} | Y {}",
        dimension.name,
        biome.name,
        block_position.x,
        block_position.z,
        block_position.y,
    );
}
