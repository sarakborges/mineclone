use bevy::{
    ecs::system::SystemParam,
    input::{ButtonState, keyboard::KeyboardInput},
    prelude::*,
};

use crate::{
    content::{
        biome::{BiomeKind, BiomeRegistry},
        dimension::DimensionRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    ui::{scrollbar::vertical_scrollbar, surface, text_input::select_all_pressed, typography},
    world::{NewWorldConfig, dimension::DEFAULT_DIMENSION_ID},
};

use super::{
    layout::{
        OPTION_HEIGHT, SpawnBiomeDropdownButton, SpawnBiomeDropdownLabel,
        SpawnBiomeDropdownPanel, SpawnBiomeOption, SpawnBiomeOptionLabel, SpawnBiomeOptionsFrame,
        SpawnBiomeOptionsList, SpawnBiomeSearchBar, SpawnBiomeSearchText,
    },
    state::SpawnBiomeDropdownState,
};
use crate::screens::settings_screen::{
    game_rules_section::TicksPerSecondInputState,
    navigation::{SettingsSection, SettingsSectionSelection},
    new_world_section::SeedInputState,
};

#[derive(SystemParam)]
pub(in crate::screens::settings_screen) struct SpawnBiomeUiContent<'w> {
    biomes: Res<'w, BiomeRegistry>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
}

impl SpawnBiomeUiContent<'_> {
    fn inputs_changed(&self) -> bool {
        self.biomes.is_changed() || self.localization.is_changed() || self.language.is_changed()
    }

    fn option_label(&self, biome_id: Option<&str>) -> String {
        let language = self.language.get();
        biome_id.map_or_else(
            || {
                self.localization
                    .text(language, "newWorld.spawnBiome.random")
                    .to_owned()
            },
            |biome_id| {
                self.biomes.get(biome_id).map_or_else(
                    || biome_id.to_owned(),
                    |biome| biome.name.text(language).to_owned(),
                )
            },
        )
    }
}

pub(in crate::screens::settings_screen) fn populate_spawn_biome_options(
    mut commands: Commands,
    frames: Query<Entity, Added<SpawnBiomeOptionsFrame>>,
    lists: Query<Entity, Added<SpawnBiomeOptionsList>>,
    dimensions: Res<DimensionRegistry>,
    content: SpawnBiomeUiContent,
    config: Res<NewWorldConfig>,
) {
    let Ok(frame_entity) = frames.single() else {
        return;
    };
    let Ok(list_entity) = lists.single() else {
        return;
    };

    let dimension = dimensions
        .get(DEFAULT_DIMENSION_ID)
        .unwrap_or_else(|| panic!("missing default dimension definition: {DEFAULT_DIMENSION_ID}"));
    let language = content.language.get();
    let mut options = dimension
        .biomes
        .iter()
        .filter_map(|entry| {
            let biome = content
                .biomes
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

    commands.entity(list_entity).with_children(|list| {
        spawn_option(
            list,
            None,
            content.option_label(None),
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

    commands.entity(frame_entity).with_children(|frame| {
        frame.spawn(vertical_scrollbar(list_entity));
    });
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

pub(in crate::screens::settings_screen) fn handle_spawn_biome_dropdown_button(
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

pub(in crate::screens::settings_screen) fn close_spawn_biome_dropdown_outside_general(
    selection: Res<SettingsSectionSelection>,
    mut state: ResMut<SpawnBiomeDropdownState>,
) {
    if selection.is_changed() && selection.selected != SettingsSection::General && state.open {
        state.close();
    }
}

pub(in crate::screens::settings_screen) fn handle_spawn_biome_search_focus(
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

pub(in crate::screens::settings_screen) fn handle_spawn_biome_option_buttons(
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

pub(in crate::screens::settings_screen) fn handle_spawn_biome_search_keyboard(
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

pub(in crate::screens::settings_screen) fn sync_spawn_biome_dropdown_state(
    state: Res<SpawnBiomeDropdownState>,
    content: SpawnBiomeUiContent,
    mut search_texts: Query<&mut Text, With<SpawnBiomeSearchText>>,
    mut panels: Query<&mut Node, With<SpawnBiomeDropdownPanel>>,
    mut search_borders: Query<&mut BorderColor, With<SpawnBiomeSearchBar>>,
) {
    if !state.is_changed() && !content.localization.is_changed() && !content.language.is_changed() {
        return;
    }

    let language = content.language.get();
    let next_search_text = if state.search.text().is_empty() {
        content
            .localization
            .text(language, "newWorld.spawnBiome.search")
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
            *border = next_border;
        }
    }
}

pub(in crate::screens::settings_screen) fn sync_spawn_biome_selected_label(
    config: Res<NewWorldConfig>,
    content: SpawnBiomeUiContent,
    mut labels: Query<&mut Text, With<SpawnBiomeDropdownLabel>>,
) {
    if !config.is_changed() && !content.inputs_changed() {
        return;
    }

    let next = content.option_label(config.spawn_biome());
    for mut text in &mut labels {
        if text.0 != next {
            text.0 = next.clone();
        }
    }
}

pub(in crate::screens::settings_screen) fn sync_spawn_biome_option_labels(
    content: SpawnBiomeUiContent,
    mut labels: Query<(&SpawnBiomeOptionLabel, &mut Text)>,
) {
    if !content.inputs_changed() {
        return;
    }

    for (option, mut text) in &mut labels {
        let next = content.option_label(option.biome_id.as_deref());
        if text.0 != next {
            text.0 = next;
        }
    }
}

pub(in crate::screens::settings_screen) fn sync_spawn_biome_options(
    state: Res<SpawnBiomeDropdownState>,
    config: Res<NewWorldConfig>,
    content: SpawnBiomeUiContent,
    changed_interactions: Query<(), (With<SpawnBiomeOption>, Changed<Interaction>)>,
    mut options: Query<(
        &SpawnBiomeOption,
        &Interaction,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut scroll_positions: Query<&mut ScrollPosition, With<SpawnBiomeOptionsList>>,
    mut previous_query: Local<String>,
) {
    let query_changed = previous_query.as_str() != state.search.text();
    let filter_changed = query_changed || content.inputs_changed();
    let style_changed = config.is_changed() || !changed_interactions.is_empty();
    if !filter_changed && !style_changed {
        return;
    }

    let normalized_query = filter_changed.then(|| state.search.text().to_lowercase());
    if query_changed {
        *previous_query = state.search.text().to_owned();
        for mut scroll in &mut scroll_positions {
            if scroll.0 != Vec2::ZERO {
                scroll.0 = Vec2::ZERO;
            }
        }
    }

    for (option, interaction, mut node, background, border) in &mut options {
        if let Some(normalized_query) = normalized_query.as_deref() {
            let option_label = content.option_label(option.biome_id.as_deref());
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
        surface::apply_control_colors(
            surface::hud_control_colors(*interaction, selected),
            background,
            border,
        );
    }
}
