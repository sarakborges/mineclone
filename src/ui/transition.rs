use bevy::prelude::*;

use crate::app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState};

use super::theme;

const TRANSITION_HALF_SECONDS: f32 = 0.16;

#[derive(Clone, Copy, Default)]
pub struct ScreenTransitionTarget {
    game_state: Option<GameState>,
    pause_state: Option<PauseState>,
    settings_state: Option<SettingsState>,
}

impl ScreenTransitionTarget {
    pub fn game(state: GameState) -> Self {
        Self {
            game_state: Some(state),
            ..default()
        }
    }

    pub fn pause(state: PauseState) -> Self {
        Self {
            pause_state: Some(state),
            ..default()
        }
    }

    pub fn settings(state: SettingsState) -> Self {
        Self {
            settings_state: Some(state),
            ..default()
        }
    }

    pub fn with_pause(mut self, state: PauseState) -> Self {
        self.pause_state = Some(state);
        self
    }
}

#[derive(Resource, Default)]
pub struct ScreenTransition {
    phase: ScreenTransitionPhase,
    progress: f32,
    target: Option<ScreenTransitionTarget>,
}

impl ScreenTransition {
    pub fn request(&mut self, target: ScreenTransitionTarget) {
        if self.is_active() {
            return;
        }

        self.target = Some(target);
        self.progress = 0.0;
        self.phase = ScreenTransitionPhase::FadingOut;
    }

    pub fn is_active(&self) -> bool {
        self.phase != ScreenTransitionPhase::Idle
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum ScreenTransitionPhase {
    #[default]
    Idle,
    FadingOut,
    FadingIn,
}

#[derive(Component)]
pub struct ScreenTransitionOverlay;

pub fn spawn_transition_overlay(mut commands: Commands) {
    commands.spawn((
        ScreenTransitionOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(theme::SCREEN_BACKGROUND.with_alpha(0.0)),
        GlobalZIndex(10_000),
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

pub fn animate_screen_transition(
    time: Res<Time<Real>>,
    mut transition: ResMut<ScreenTransition>,
    mut overlay: Query<(&mut BackgroundColor, &mut Visibility), With<ScreenTransitionOverlay>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_pause_state: ResMut<NextState<PauseState>>,
    mut next_settings_state: ResMut<NextState<SettingsState>>,
) {
    let Ok((mut background, mut visibility)) = overlay.single_mut() else {
        return;
    };

    if transition.phase == ScreenTransitionPhase::Idle {
        *visibility = Visibility::Hidden;
        *background = theme::SCREEN_BACKGROUND.with_alpha(0.0).into();
        return;
    }

    *visibility = Visibility::Visible;
    let step = time.delta_secs() / TRANSITION_HALF_SECONDS;

    match transition.phase {
        ScreenTransitionPhase::Idle => {}
        ScreenTransitionPhase::FadingOut => {
            transition.progress = (transition.progress + step).min(1.0);
            let alpha = ease_in_out_cubic(transition.progress);
            *background = theme::SCREEN_BACKGROUND.with_alpha(alpha).into();

            if transition.progress >= 1.0 {
                if let Some(target) = transition.target {
                    if let Some(state) = target.game_state {
                        next_game_state.set(state);
                    }
                    if let Some(state) = target.pause_state {
                        next_pause_state.set(state);
                    }
                    if let Some(state) = target.settings_state {
                        next_settings_state.set(state);
                    }
                }

                transition.phase = ScreenTransitionPhase::FadingIn;
            }
        }
        ScreenTransitionPhase::FadingIn => {
            transition.progress = (transition.progress - step).max(0.0);
            let alpha = ease_in_out_cubic(transition.progress);
            *background = theme::SCREEN_BACKGROUND.with_alpha(alpha).into();

            if transition.progress <= 0.0 {
                transition.phase = ScreenTransitionPhase::Idle;
                transition.target = None;
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn ease_in_out_cubic(value: f32) -> f32 {
    if value < 0.5 {
        4.0 * value * value * value
    } else {
        1.0 - (-2.0 * value + 2.0).powi(3) / 2.0
    }
}
