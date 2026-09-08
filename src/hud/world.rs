use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry,
        day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry,
    },
    player::{camera::GameplayCamera, PLAYER_EYE_HEIGHT},
    ui::{theme, typography},
    world::{
        biome::CurrentBiome,
        day_night::DayNightClock,
        dimension::CurrentDimension,
    },
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
            left: px(0),
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Node {
                padding: UiRect::all(px(12)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(theme::HUD_SURFACE),
            children![(
                typography::hud("- - -\nDay 1 - 00:00\nX 0 | Z 0 | Y 0"),
                WorldHudText,
            )],
        )],
    ));
}

fn update_world_hud(
    player: Single<&Transform, With<GameplayCamera>>,
    dimension: Res<CurrentDimension>,
    biome: Res<CurrentBiome>,
    clock: Res<DayNightClock>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    mut world_text: Single<&mut Text, With<WorldHudText>>,
) {
    let position = player.translation - Vec3::Y * PLAYER_EYE_HEIGHT;
    let block_position = position.floor().as_ivec3();
    let dimension_definition = dimensions.get(&dimension.id);
    let dimension_name = dimension_definition
        .map(|definition| definition.name.as_str())
        .unwrap_or(dimension.id.as_str());
    let biome_name = biomes
        .get(&biome.id)
        .map(|definition| definition.name.as_str())
        .unwrap_or(biome.id.as_str());
    let (hour, minute) = dimension_definition
        .and_then(|definition| cycles.get(&definition.day_night_cycle))
        .map(|cycle| cycle.world_time(clock.normalized_time))
        .unwrap_or((0, 0));

    world_text.0 = format!(
        "{} - {}\nDay {} - {:02}:{:02}\nX {} | Z {} | Y {}",
        dimension_name,
        biome_name,
        clock.day,
        hour,
        minute,
        block_position.x,
        block_position.z,
        block_position.y,
    );
}
