use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::{day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry},
    localization::{ActiveLanguage, UiLocalization},
    ui::{surface, typography},
    world::{day_night::DayNightClock, dimension::CurrentDimension},
};

pub struct TimeHudPlugin;

impl Plugin for TimeHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_time_hud)
            .add_systems(
                Update,
                update_time_hud.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct TimeHudText;

fn spawn_time_hud(
    mut commands: Commands,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                left: px(16),
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn(surface::hud_panel()).with_children(|panel| {
                panel.spawn((
                    typography::hud(format!(
                        "{} 1\n00:00",
                        localization.text(language.get(), "hud.day")
                    )),
                    TextLayout::justify(Justify::Center),
                    TimeHudText,
                ));
            });
        });
}

fn update_time_hud(
    dimension: Res<CurrentDimension>,
    clock: Res<DayNightClock>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut time_text: Single<&mut Text, With<TimeHudText>>,
) {
    let (hour, minute) = dimensions
        .get(&dimension.id)
        .and_then(|definition| cycles.get(&definition.day_night_cycle))
        .map(|cycle| cycle.world_time(clock.normalized_time))
        .unwrap_or((0, 0));
    let next_text = format!(
        "{} {}\n{:02}:{:02}",
        localization.text(language.get(), "hud.day"),
        clock.day,
        hour,
        minute
    );

    if time_text.0 != next_text {
        time_text.0 = next_text;
    }
}
