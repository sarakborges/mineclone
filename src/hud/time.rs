use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    localization::{ActiveLanguage, UiLocalization},
    ui::typography,
    world::current_context::DayNightContext,
};

pub struct TimeHudPlugin;

impl Plugin for TimeHudPlugin {
    fn build(&self, app: &mut App) {
        appaadd_systems(OnEnter(GameState::Gameplay), spawn_time_hud)
            aadd_systems(
                Update,
                update_time_hud.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component, Default)]
struct TimeHudText {
    presented_time: Option<(u64, u32, u32)>,
}

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
        TimeHudText::default(),
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
    time_hud: Single<(&mut Text, &mut TimeHudText)>,
) {
    let (mut time_text, mut state) = time_hud.into_inner();
    let localization_changed = language.is_changed() || localization.is_changed();

    if !day_night.inputs_changed() && !localization_changed && state.presented_time.is_some() {
        return;
    }

    let (hour, minute) = day_night.world_time().unwrap_or((0, 0));
    let presented_time = (day_night.clock().day, hour, minute);
    if state.presented_time == Some(presented_time) && !localization_changed {
        return;
    }

    let next_text = format!(
        "{} {}\n{:02}:{:02}",
        localization.text(language.get(), "hud.day"),
        presented_time.0,
        presented_time.1,
        presented_time.2
    );

    if time_text.0 != next_text {
        time_text.0 = next_text;
    }
    if state.presented_time != Some(presented_time) {
        state.presented_time = Some(presented_time);
    }
}
