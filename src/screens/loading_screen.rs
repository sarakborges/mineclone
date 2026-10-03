use bevy::{
    camera::CameraOutputMode, ecs::system::SystemParam, prelude::*,
    render::render_resource::BlendState, ui::IsDefaultUiCamera,
};

use crate::{
    app::game_state::GameState,
    hud::GameplayUiCamera,
    localization::{ActiveLanguage, Language, UiLocalization},
    rendering::camera_stack::UI_CAMERA_ORDER,
    ui::{
        cosmic_background::{self, STAR_FIELD},
        surface, theme, typography,
    },
    world::{WorldLoadingPhaseStatus, WorldLoadingState, WorldLoadingStep},
};

const LOADING_PANEL_WIDTH: f32 = 600.0;
const LOADING_ROW_HEIGHT: f32 = 32.0;
const LOADING_ROW_GAP: f32 = 2.0;
const LOADING_PHASE_INDEX_WIDTH: f32 = 34.0;

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), setup_loading_screen)
            .add_systems(
                Update,
                update_loading_progress.run_if(in_state(GameState::Loading)),
            );
    }
}

#[derive(Component)]
struct LoadingSummaryText;

#[derive(Component)]
struct LoadingStepRow(WorldLoadingStep);

#[derive(Component)]
struct LoadingStepLabel(WorldLoadingStep);

#[derive(Component)]
struct LoadingStepStatusText(WorldLoadingStep);

#[derive(SystemParam)]
struct LoadingProgressUi<'w, 's> {
    summaries: Query<
        'w,
        's,
        &'static mut Text,
        (With<LoadingSummaryText>, Without<LoadingStepStatusText>),
    >,
    rows: Query<'w, 's, (&'static LoadingStepRow, &'static mut BackgroundColor)>,
    labels: Query<
        'w,
        's,
        (&'static LoadingStepLabel, &'static mut TextColor),
        Without<LoadingStepStatusText>,
    >,
    statuses: Query<
        'w,
        's,
        (
            &'static LoadingStepStatusText,
            &'static mut Text,
            &'static mut TextColor,
        ),
    >,
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
            screen.spawn((
                LoadingSummaryText,
                typography::muted(
                    localization
                        .text(language, "loading.summary.pending")
                        .to_owned(),
                ),
            ));

            screen
                .spawn(surface::hud_container(Node {
                    width: px(LOADING_PANEL_WIDTH),
                    padding: UiRect::all(px(12)),
                    border: UiRect::all(px(1)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(LOADING_ROW_GAP),
                    ..default()
                }))
                .with_children(|step_list| {
                    for (index, step) in WorldLoadingStep::ALL.into_iter().enumerate() {
                        spawn_loading_step_row(step_list, &localization, language, index + 1, step);
                    }
                });
        });
}

fn spawn_loading_step_row(
    parent: &mut ChildSpawnerCommands,
    localization: &UiLocalization,
    language: Language,
    index: usize,
    step: WorldLoadingStep,
) {
    parent
        .spawn((
            LoadingStepRow(step),
            Node {
                width: percent(100),
                height: px(LOADING_ROW_HEIGHT),
                padding: UiRect::horizontal(px(10)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            row.spawn((
                typography::caption(format!("{index:02}")),
                Node {
                    width: px(LOADING_PHASE_INDEX_WIDTH),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));

            row.spawn((
                LoadingStepLabel(step),
                typography::hud(loading_step_label(localization, language, step)),
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ));

            row.spawn((
                LoadingStepStatusText(step),
                typography::caption(
                    localization
                        .text(language, "loading.status.pending")
                        .to_owned(),
                ),
                TextLayout::justify(Justify::Right),
                Node {
                    min_width: px(120),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

fn update_loading_progress(
    loading_state: Option<Res<WorldLoadingState>>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut ui: LoadingProgressUi,
) {
    let Some(loading_state) = loading_state else {
        return;
    };
    let language = language.get();

    if let Ok(mut summary) = ui.summaries.single_mut() {
        let next = format_loading_summary(&localization, language, &loading_state);
        if **summary != next {
            **summary = next;
        }
    }

    for (step_row, mut background) in &mut ui.rows {
        let status = loading_state.step_status(step_row.0);
        let target = if status == WorldLoadingPhaseStatus::Active {
            theme::PURPLE_SOFT
        } else {
            Color::NONE
        };
        if background.0 != target {
            background.0 = target;
        }
    }

    for (step_label, mut color) in &mut ui.labels {
        let target = phase_text_color(loading_state.step_status(step_label.0));
        if color.0 != target {
            color.0 = target;
        }
    }

    for (status_label, mut text, mut color) in &mut ui.statuses {
        let step = status_label.0;
        let status = loading_state.step_status(step);
        let next = format_loading_status(
            &localization,
            language,
            status,
            loading_state.step_progress(step),
        );
        if **text != next {
            **text = next;
        }

        let target = phase_status_color(status);
        if color.0 != target {
            color.0 = target;
        }
    }
}

fn format_loading_summary(
    localization: &UiLocalization,
    language: Language,
    loading_state: &WorldLoadingState,
) -> String {
    localization
        .text(language, "loading.summary")
        .replace("{columns}", &loading_state.column_count().to_string())
        .replace("{sections}", &loading_state.total().to_string())
}

fn format_loading_status(
    localization: &UiLocalization,
    language: Language,
    status: WorldLoadingPhaseStatus,
    progress: Option<(usize, usize)>,
) -> String {
    match status {
        WorldLoadingPhaseStatus::Pending => localization
            .text(language, "loading.status.pending")
            .to_owned(),
        WorldLoadingPhaseStatus::Active => progress
            .map(|(completed, total)| format!("{completed}/{total}"))
            .unwrap_or_else(|| {
                localization
                    .text(language, "loading.status.active")
                    .to_owned()
            }),
        WorldLoadingPhaseStatus::Done => localization
            .text(language, "loading.status.done")
            .to_owned(),
    }
}

fn loading_step_label(
    localization: &UiLocalization,
    language: Language,
    step: WorldLoadingStep,
) -> String {
    let translated = match (language, step) {
        (Language::English, WorldLoadingStep::BiomeMap) => "Biome map",
        (Language::English, WorldLoadingStep::TerrainColumns) => "Terrain columns",
        (Language::English, WorldLoadingStep::VolumeBiomes) => "Volume biomes",
        (Language::English, WorldLoadingStep::DensityField) => "Density field",
        (Language::English, WorldLoadingStep::Materials) => "Material rasterization",
        (Language::English, WorldLoadingStep::InitialFluids) => "Initial fluid rasterization",
        (Language::English, WorldLoadingStep::Structures) => "Structures",
        (Language::English, WorldLoadingStep::SurfaceObjects) => "Surface objects",
        (Language::English, WorldLoadingStep::ChunkIntegration) => "Chunk integration",
        (Language::PortugueseBrazil, WorldLoadingStep::BiomeMap) => "Mapa de biomas",
        (Language::PortugueseBrazil, WorldLoadingStep::TerrainColumns) => "Colunas de terreno",
        (Language::PortugueseBrazil, WorldLoadingStep::VolumeBiomes) => "Biomas volumétricos",
        (Language::PortugueseBrazil, WorldLoadingStep::DensityField) => "Campo de densidade",
        (Language::PortugueseBrazil, WorldLoadingStep::Materials) => "Rasterização de materiais",
        (Language::PortugueseBrazil, WorldLoadingStep::InitialFluids) => {
            "Rasterização inicial de fluidos"
        }
        (Language::PortugueseBrazil, WorldLoadingStep::Structures) => "Estruturas",
        (Language::PortugueseBrazil, WorldLoadingStep::SurfaceObjects) => "Objetos de superfície",
        (Language::PortugueseBrazil, WorldLoadingStep::ChunkIntegration) => "Integração de chunks",
        (Language::Spanish, WorldLoadingStep::BiomeMap) => "Mapa de biomas",
        (Language::Spanish, WorldLoadingStep::TerrainColumns) => "Columnas de terreno",
        (Language::Spanish, WorldLoadingStep::VolumeBiomes) => "Biomas volumétricos",
        (Language::Spanish, WorldLoadingStep::DensityField) => "Campo de densidad",
        (Language::Spanish, WorldLoadingStep::Materials) => "Rasterización de materiales",
        (Language::Spanish, WorldLoadingStep::InitialFluids) => "Rasterización inicial de fluidos",
        (Language::Spanish, WorldLoadingStep::Structures) => "Estructuras",
        (Language::Spanish, WorldLoadingStep::SurfaceObjects) => "Objetos de superficie",
        (Language::Spanish, WorldLoadingStep::ChunkIntegration) => "Integración de chunks",
        (_, WorldLoadingStep::SettlingFluids) => {
            return localization
                .text(language, "loading.phase.settlingFluids")
                .to_owned();
        }
        (_, WorldLoadingStep::Lighting) => {
            return localization
                .text(language, "loading.phase.lighting")
                .to_owned();
        }
        (_, WorldLoadingStep::Meshing) => {
            return localization
                .text(language, "loading.phase.meshing")
                .to_owned();
        }
        (_, WorldLoadingStep::Assets) => {
            return localization
                .text(language, "loading.phase.assets")
                .to_owned();
        }
        (_, WorldLoadingStep::Finalizing) => {
            return localization
                .text(language, "loading.phase.finalizing")
                .to_owned();
        }
        (_, WorldLoadingStep::Spawning) => {
            return localization
                .text(language, "loading.phase.presentation")
                .to_owned();
        }
    };
    translated.to_owned()
}

fn phase_text_color(status: WorldLoadingPhaseStatus) -> Color {
    match status {
        WorldLoadingPhaseStatus::Pending => theme::TEXT_SUBTLE,
        WorldLoadingPhaseStatus::Active | WorldLoadingPhaseStatus::Done => theme::TEXT_PRIMARY,
    }
}

fn phase_status_color(status: WorldLoadingPhaseStatus) -> Color {
    match status {
        WorldLoadingPhaseStatus::Pending => theme::TEXT_SUBTLE,
        WorldLoadingPhaseStatus::Active => theme::BORDER_FOCUS,
        WorldLoadingPhaseStatus::Done => theme::TEXT_MUTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_progress_system_initializes_without_query_conflicts() {
        let mut world = World::new();
        let mut system = IntoSystem::into_system(update_loading_progress);

        system.initialize(&mut world);
    }
}
