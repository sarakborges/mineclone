use bevy::{
    camera::CameraOutputMode,
    prelude::*,
    render::render_resource::BlendState,
    ui::IsDefaultUiCamera,
};

use crate::{
    app::game_state::GameState,
    hud::GameplayUiCamera,
    localization::{ActiveLanguage, UiLocalization},
    rendering::camera_stack::UI_CAMERA_ORDER,
    ui::{
        cosmic_background::{self, STAR_FIELD},
        theme, typography,
    },
};

/// Phase-1 loading shell.
///
/// The legacy loading-progress UI has been removed together with the legacy
/// world bootstrap pipeline. Phase 10/11 will install the new structured
/// progress model and its presentation on this screen.
pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), setup_loading_screen);
    }
}

fn setup_loading_screen(
    mut commands: Commands,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    commands.spawn((
        GameplayUiCamera,
        Camera2d,
        Camera {
            order: UI_CAMERA_ORDER,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        BoxShadowSamples(8),
        IsDefaultUiCamera,
        DespawnOnExit(GameState::Gameplay),
    ));

    let language = language.get();
    commands
        .spawn((
            DespawnOnExit(GameState::Loading),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
        ))
        .with_children(|screen| {
            for &spec in STAR_FIELD {
                screen.spawn(cosmic_background::star(spec));
            }
            screen.spawn(typography::title(
                localization.text(language, "loading.world").to_owned(),
            ));
            screen.spawn(typography::muted(
                localization
                    .text(language, "loading.summary.pending")
                    .to_owned(),
            ));
        });
}
