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
    ui::{surface, theme, typography, text_input::select_all_pressed},
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
    search_query: String,
    search_focused: bool,
    replace_search_on_next_input: bool,
}

impl SpawnBiomeDropdownState {
    pub(super) fn input_editing(&self) -> bool {
        self.open || self.search_focused
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    fn open(&mut self) {
        self.open = true;
        self.search_query.clear();
        self.search_focused = true;
        self.replace_search_on_next_input = false;
    }

    pub(super) fn close(&mut self) {
        self.open = false;
        self.search_query.clear();
        self.search_focused = false;
        self.replace_search_on_next_input = false;
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
    search_label: String,
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
    let search_label = label.to_lowercase();

    list.spawn((
        Button,
        SpawnBiomeOption {
            biome_id,
            search_label,
        },
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
        children![(typography::hud(label), Pickable::IGNORE)],
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
        state.search_focused = true;
        state.replace_search_on_next_input = false;
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
    if !state.open || !state.search_focused {
        keyboard_input.clear();
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        state.close();
        keyboard_input.clear();
        return;
    }

    if select_all_pressed(&keys) {
        state.replace_search_on_next_input = true;
    }

    for event in keyboard_input.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        if event.key_code == KeyCode::Backspace {
            if state.replace_search_on_next_input {
                state.search_query.clear();
                state.replace_search_on_next_input = false;
            } else {
                state.search_query.pop();
            }
            continue;
        }

        let Some(text) = &event.text else {
            continue;
        };
        let filtered = text
            .chars()
            .filter(|character| !character.is_control())
            .collect::<String>();
        if filtered.is_empty() {
            continue;
        }

        if state.replace_search_on_next_input {
            state.search_query.clear();
            state.replace_search_on_next_input = false;
        }
        state.search_query.push_str(&filtered);
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "spawn biome dropdown synchronizes one compact settings control from shared UI state"
)]
pub(super) fn sync_spawn_biome_dropdown_view(
    state: Res<SpawnBiomeDropdownState>,
    config: Res<NewWorldConfig>,
    biomes: Res<BiomeRegistry>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut labels: Query<&mut Text, With<SpawnBiomeDropdownLabel>>,
    mut search_texts: Query<&mut Text, (With<SpawnBiomeSearchText>, Without<SpawnBiomeDropdownLabel>)>,
    mut panels: Query<&mut Node, With<SpawnBiomeDropdownPanel>>,
    mut search_borders: Query<&mut BorderColor, With<SpawnBiomeSearchBar>>,
    mut options: Query<(
        Ref<SpawnBiomeOption>,
        Ref<Interaction>,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let state_changed = state.is_changed();
    let inputs_changed = state_changed
        || config.is_changed()
        || biomes.is_changed()
        || localization.is_changed()
        || language.is_changed();
    let language = language.get();

    if inputs_changed {
        let selected_label = config.spawn_biome().map_or_else(
            || {
                localization
                    .text(language, "newWorld.spawnBiome.random")
                    .to_owned()
            },
            |biome_id| {
                biomes
                    .get(biome_id)
                    .map_or_else(|| biome_id.to_owned(), |biome| biome.name.text(language).to_owned())
            },
        );
        for mut text in &mut labels {
            let next = format!("{selected_label}   ▾");
            if text.0 != next {
                text.0 = next;
            }
        }

        let next_search_text = if state.search_query.is_empty() {
            localization
                .text(language, "newWorld.spawnBiome.search")
                .to_owned()
        } else {
            state.search_query.clone()
        };
        for mut text in &mut search_texts {
            if text.0 != next_search_text {
                text.0 = next_search_text.clone();
            }
        }

        for mut panel in &mut panels {
            panel.display = if state.open {
                Display::Flex
            } else {
                Display::None
            };
        }
        for mut border in &mut search_borders {
            *border = BorderColor::all(if state.search_focused {
                surface::HUD_SELECTED_BORDER_COLOR
            } else {
                surface::HUD_BORDER_COLOR
            });
        }
    }

    let normalized_query = state_changed.then(|| state.search_query.to_lowercase());
    for (option, interaction, mut node, mut background, mut border) in &mut options {
        if let Some(normalized_query) = normalized_query.as_deref() {
            node.display = if normalized_query.is_empty()
                || option.search_label.contains(normalized_query)
            {
                Display::Flex
            } else {
                Display::None
            };
        } else if option.is_added() {
            node.display = Display::Flex;
        }

        if !config.is_changed() && !interaction.is_changed() && !option.is_added() {
            continue;
        }
        let selected = config.spawn_biome() == option.biome_id.as_deref();
        let (next_background, next_border) =
            surface::hud_control_colors(*interaction, selected);
        *background = BackgroundColor(next_background);
        *border = BorderColor::all(next_border);
    }
}
