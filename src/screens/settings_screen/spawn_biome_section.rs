use bevy::{
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
};

use crate::{
    content::{
        biome::{BiomeKind, BiomeRegistry},
        dimension::DimensionRegistry,
    },
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        surface, theme,
        text_input::{TextInputState, select_all_pressed},
        typography,
    },
    world::{NewWorldConfig, dimension::DEFAULT_DIMENSION_ID},
};

use super::{
    game_rules_section::TicksPerSecondInputState,
    navigation::{SettingsSection, SettingsSectionSelection},
    new_world_section::SeedInputState,
};

const CONTROL_HEIGHT: f32 = 44.0;
const SEARCH_HEIGHT: f32 = 40.0;
const OPTION_HEIGHT: f32 = 40.0;

#[derive(Resource, Default)]
pub(super) struct SpawnBiomeDropdownState {
    open: bool,
    search: TextInputState,
}

impl SpawnBiomeDropdownState {
    pub(super) fn input_editing(&self) -> bool {
        self.open || self.search.focused()
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    fn open(&mut self) {
        self.open = true;
        self.search.reset();
        self.search.focus();
    }

    pub(super) fn close(&mut self) {
        self.open = false;
        self.search.reset();
    }
}

#[derive(Component)]
pub(super) struct SpawnBiomeDropdownButton;

#[derive(Component)]
pub(super) struct SpawnBiomeDropdownLabel;

#[derive(Component)]
pub(super) struct SpawnBiomeDropdownPanel;

#[derive(Component)]
pub(super) struct SpawnBiomeSearchBar;

#[derive(Component)]
pub(super) struct SpawnBiomeSearchText;

#[derive(Component)]
pub(super) struct SpawnBiomeOptionsList;

#[derive(Component)]
pub(super) struct SpawnBiomeOption {
    biome_id: Option<String>,
}

#[derive(Component)]
pub(super) struct SpawnBiomeOptionLabel {
    biome_id: Option<String>,
}

pub(super) fn spawn_biome_setting(
    localization: &UiLocalization,
    language: Language,
) -> impl Bundle {
    let (control_background, control_border) = surface::hud_control_static(false);

    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(10),
            ..default()
        },
        children![
            typography::setting_title(
                localization
                    .text(language, "newWorld.spawnBiome")
                    .to_owned(),
            ),
            typography::caption(
                localization
                    .text(language, "newWorld.spawnBiome.description")
                    .to_owned(),
            ),
            (
                Button,
                SpawnBiomeDropdownButton,
                Node {
                    width: percent(100),
                    height: px(CONTROL_HEIGHT),
                    padding: UiRect::horizontal(px(12)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                BackgroundColor(control_background),
                BorderColor::all(control_border),
                children![
                    (
                        SpawnBiomeDropdownLabel,
                        typography::hud(format!(
                            "{}   ▾",
                            localization.text(language, "newWorld.spawnBiome.random")
                        )),
                        Pickable::IGNORE,
                    ),
                ],
            ),
            (
                SpawnBiomeDropdownPanel,
                Node {
                    display: Display::None,
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(6),
                    padding: UiRect::all(px(8)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(theme::HUD_SURFACE),
                BorderColor::all(surface::HUD_BORDER_COLOR),
                children![
                    (
                        Button,
                        SpawnBiomeSearchBar,
                        Node {
                            width: percent(100),
                            height: px(SEARCH_HEIGHT),
                            padding: UiRect::horizontal(px(10)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(5)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(theme::HUD_SURFACE),
                        BorderColor::all(surface::HUD_SELECTED_BORDER_COLOR),
                        children![(
                            SpawnBiomeSearchText,
                            typography::hud(
                                localization
                                    .text(language, "newWorld.spawnBiome.search")
                                    .to_owned(),
                            ),
                            Pickable::IGNORE,
                        )],
                    ),
                    (
                        SpawnBiomeOptionsList,
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            row_gap: px(4),
                            ..default()
                        },
                    ),
                ],
            ),
        ],
    )
}

pub(super) fn populate_spawn_biome_options(
    mut commands: Commands,
    lists: Query<Entity, Added<SpawnBiomeOptionsList>>,
    dimensions: Res<DimensionRegistry>,
    biomes: Res<BiomeRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    config: Res<NewWorldConfig>,
) {
    if lists.is_empty() {
        return;
    }

    let dimension = dimensions
        .get(DEFAULT_DIMENSION_ID)
        .unwrap_or_else(|| panic!("missing default dimension definition: {DEFAULT_DIMENSION_ID}"));
    let language = language.get();
    let mut options = dimension
        .biomes
        .iter()
        .filter_map(|entry| {
            let biome = biomes
                .get(&entry.id)
                .unwrap_or_else(|| panic!("missing biome definition: {}", entry.id));
            (biome.kind == BiomeKind::Surface).then_some((
                biome.id.clone(),
                biome.name.text(language).to_owned(),
            ))
        })
        .collect::<Vec<_>>();
    options.sort_by(|left, right| {
        left.1
            .to_lowercase()
            .cmp(&right.1.to_lowercase())
            .then_with(|| left.0.cmp(&right.0))
    });

    for list_entity in &lists {
        commands.entity(list_entity).with_children(|list| {
            let random_label = localization
                .text(language, "newWorld.spawnBiome.random")
                .to_owned();
            spawn_option(
                list,
                None,
                random_label,
                config.spawn_biome().is_none(),
            );

            for (biome_id, label) in &options {
                spawn_option(
                    list,
                    Some(biome_id.clone()),
                    label.clone(),
                    config.spawn_biome() == Some(biome_id.as_str()),
                );
            }
        });
    }
}

fn spawn_option(
    list: &mut ChildSpawnerCommands,
    biome_id: Option<String>,
    label: String,
    selected: bool,
) {
    let (background, border) = surface::hud_control_static(selected);
    let label_biome_id = biome_id.clone();

    list.spawn((
        Button,
        SpawnBiomeOption { biome_id },
        Node {
            width: percent(100),
            height: px(OPTION_HEIGHT),
            min_height: px(OPTION_HEIGHT),
            padding: UiRect::horizontal(px(10)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(border),
        children![(
            SpawnBiomeOptionLabel {
                biome_id: label_biome_id,
            },
            typography::hud(label),
            Pickable::IGNORE,
        )],
    ));
}

pub(super) fn handle_spawn_biome_dropdown_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<SpawnBiomeDropdownButton>)>,
    mut state: ResMut<SpawnBiomeDropdownState>,
    mut seed_input: ResMut<SeedInputState>,
    mut ticks_input: ResMut<TicksPerSecondInputState>,
) {
    if !buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    if state.open {
        state.close();
    } else {
        seed_input.reset();
        ticks_input.reset();
        state.open();
    }
}

pub(super) fn close_spawn_biome_dropdown_outside_general(
    selection: Res<SettingsSectionSelection>,
    mut state: ResMut<SpawnBiomeDropdownState>,
) {
    if selection.is_changed() && selection.selected != SettingsSection::General && state.open {
        state.close();
    }
}

pub(super) fn handle_spawn_biome_search_focus(
    search_bars: Query<&Interaction, (Changed<Interaction>, With<SpawnBiomeSearchBar>)>,
    mut state: ResMut<SpawnBiomeDropdownState>,
) {
    if search_bars
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.search.focus();
    }
}

pub(super) fn handle_spawn_biome_option_buttons(
    options: Query<(&Interaction, &SpawnBiomeOption), Changed<Interaction>>,
    mut config: ResMut<NewWorldConfig>,
    mut state: ResMut<SpawnBiomeDropdownState>,
) {
    for (interaction, option) in &options {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if config.spawn_biome() != option.biome_id.as_deref() {
            config.set_spawn_biome(option.biome_id.clone());
        }
        state.close();
        break;
    }
}

pub(super) fn handle_spawn_biome_search_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut state: ResMut<SpawnBiomeDropdownState>,
) {
    if !state.open || !state.search.focused() {
        keyboard_input.clear();
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        state.close();
        keyboard_input.clear();
        return;
    }

    if select_all_pressed(&keys) {
        state.search.select_all();
    }

    for event in keyboard_input.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        if event.key_code == KeyCode::Backspace {
            state.search.backspace();
            continue;
        }

        if let Some(text) = &event.text {
            state.search.push_text(text);
        }
    }
}

pub(super) fn sync_spawn_biome_dropdown_state(
    state: Res<SpawnBiomeDropdownState>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut search_texts: Query<&mut Text, With<SpawnBiomeSearchText>>,
    mut panels: Query<&mut Node, With<SpawnBiomeDropdownPanel>>,
    mut search_borders: Query<&mut BorderColor, With<SpawnBiomeSearchBar>>,
) {
    if !state.is_changed() && !localization.is_changed() && !language.is_changed() {
        return;
    }

    let language = language.get();
    let next_search_text = if state.search.text().is_empty() {
        localization.text(language, "newWorld.spawnBiome.search")
    } else {
        state.search.text()
    };
    for mut text in &mut search_texts {
        if text.0 != next_search_text {
            text.0 = next_search_text.to_owned();
        }
    }

    let next_display = if state.open {
        Display::Flex
    } else {
        Display::None
    };
    for mut panel in &mut panels {
        if panel.display != next_display {
            panel.display = next_display;
        }
    }

    let next_border = BorderColor::all(if state.search.focused() {
        surface::HUD_SELECTED_BORDER_COLOR
    } else {
        surface::HUD_BORDER_COLOR
    });
    for mut border in &mut search_borders {
        if *border != next_border {
            *border = next_border.clone();
        }
    }
}

pub(super) fn sync_spawn_biome_selected_label(
    config: Res<NewWorldConfig>,
    biomes: Res<BiomeRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut labels: Query<&mut Text, With<SpawnBiomeDropdownLabel>>,
) {
    if !config.is_changed()
        && !biomes.is_changed()
        && !localization.is_changed()
        && !language.is_changed()
    {
        return;
    }

    let language = language.get();
    let selected_label = spawn_biome_option_label(
        config.spawn_biome(),
        &biomes,
        &localization,
        language,
    );
    let next = format!("{selected_label}   ▾");
    for mut text in &mut labels {
        if text.0 != next {
            text.0 = next.clone();
        }
    }
}

pub(super) fn sync_spawn_biome_option_labels(
    biomes: Res<BiomeRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut labels: Query<(&SpawnBiomeOptionLabel, &mut Text)>,
) {
    if !biomes.is_changed() && !localization.is_changed() && !language.is_changed() {
        return;
    }

    let language = language.get();
    for (option, mut text) in &mut labels {
        let next = spawn_biome_option_label(
            option.biome_id.as_deref(),
            &biomes,
            &localization,
            language,
        );
        if text.0 != next {
            text.0 = next;
        }
    }
}

pub(super) fn sync_spawn_biome_options(
    state: Res<SpawnBiomeDropdownState>,
    config: Res<NewWorldConfig>,
    biomes: Res<BiomeRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    changed_interactions: Query<(), (With<SpawnBiomeOption>, Changed<Interaction>)>,
    mut options: Query<(
        &SpawnBiomeOption,
        &Interaction,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut previous_query: Local<String>,
) {
    let query_changed = previous_query.as_str() != state.search.text();
    let filter_changed = query_changed
        || biomes.is_changed()
        || localization.is_changed()
        || language.is_changed();
    let style_changed = config.is_changed() || !changed_interactions.is_empty();
    if !filter_changed && !style_changed {
        return;
    }

    let language = language.get();
    let normalized_query = filter_changed.then(|| state.search.text().to_lowercase());
    if query_changed {
        *previous_query = state.search.text().to_owned();
    }

    for (option, interaction, mut node, mut background, mut border) in &mut options {
        if let Some(normalized_query) = normalized_query.as_deref() {
            let option_label = spawn_biome_option_label(
                option.biome_id.as_deref(),
                &biomes,
                &localization,
                language,
            );
            let visible = normalized_query.is_empty()
                || option_label.to_lowercase().contains(normalized_query);
            let next_display = if visible {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != next_display {
                node.display = next_display;
            }
        }

        if !style_changed {
            continue;
        }
        let selected = config.spawn_biome() == option.biome_id.as_deref();
        let (next_background, next_border) = surface::hud_control_colors(*interaction, selected);
        if background.0 != next_background {
            background.0 = next_background;
        }
        let next_border = BorderColor::all(next_border);
        if *border != next_border {
            *border = next_border;
        }
    }
}

fn spawn_biome_option_label(
    biome_id: Option<&str>,
    biomes: &BiomeRegistry,
    localization: &UiLocalization,
    language: Language,
) -> String {
    biome_id.map_or_else(
        || localization.text(language, "newWorld.spawnBiome.random").to_owned(),
        |biome_id| {
            biomes
                .get(biome_id)
                .map_or_else(|| biome_id.to_owned(), |biome| biome.name.text(language).to_owned())
        },
    )
}
