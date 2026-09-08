use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, dimension::DimensionRegistry},
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    ui::typography,
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
struct DimensionHudText;

#[derive(Component)]
struct BiomeHudText;

#[derive(Component)]
struct CoordinatesHudText;

fn spawn_world_hud(mut commands: Commands) {
    let shadow = TextShadow {
        offset: Vec2::new(1.5, 1.5),
        color: Color::srgba(0.0, 0.0, 0.0, 0.92),
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(14),
                left: px(0),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2),
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                typography::hud_heading("-"),
                TextLayout::justify(Justify::Center),
                shadow,
                DimensionHudText,
            ));
            root.spawn((
                typography::hud_subheading("-"),
                TextLayout::justify(Justify::Center),
                shadow,
                BiomeHudText,
            ));
            root.spawn((
                typography::hud("X: 0 | Y: 0 | Z: 0"),
                TextLayout::justify(Justify::Center),
                shadow,
                CoordinatesHudText,
            ));
        });
}

fn update_world_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    dimension: Res<CurrentDimension>,
    biome: Res<CurrentBiome>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
    mut dimension_text: Single<&mut Text, With<DimensionHudText>>,
    mut biome_text: Single<&mut Text, With<BiomeHudText>>,
    mut coordinates_text: Single<&mut Text, With<CoordinatesHudText>>,
) {
    let position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let block_position = position.floor().as_ivec3();
    let dimension_name = dimensions
        .get(&dimension.id)
        .map(|definition| definition.name.as_str())
        .unwrap_or(dimension.id.as_str());
    let biome_name = biomes
        .get(&biome.id)
        .map(|definition| definition.name.as_str())
        .unwrap_or(biome.id.as_str());

    dimension_text.0 = dimension_name.to_string();
    biome_text.0 = biome_name.to_string();
    coordinates_text.0 = format!(
        "X: {} | Y: {} | Z: {}",
        block_position.x, block_position.y, block_position.z
    );
}
