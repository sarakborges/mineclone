use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    localization::{ActiveLanguage, UiLocalization},
    ui::typography,
    world::current_context::DayNightContext,
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
    commands.spawn((
        typography::hud(format!(
            "{} 1\n00:00",
            localization.text(language.get(), "hud.day")
        )),
        TextLayout::justify(Justify::Center),
        typography::tooltip_shadow(),
        TimeHudText,
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
            ..default()
        },
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_time_hud(
    day_night: DayNightContext,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut time_text: Single<&mut Text, With<TimeHudText>>,
) {
    let (hour, minute) = day_night.world_time().unwrap_or((0, 0));
    let next_text = format!(
        "{} {}\n{:02}:{:02}",
        localization.text(language.get(), "hud.day"),
        day_night.clock().day,
        hour,
        minute
    );

    if time_text.0 != next_text {
        time_text.0 = next_text;
    }
}
