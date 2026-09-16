"""Guarded follow-up to the Bevy 0.19 native EditableText migration.

Runs on the diagnostic feature branch only; validation gates promotion to chat PR.
"""
from pathlib import Path


def change(path: str, old: str, new: str):
    p = Path(path)
    source = p.read_text()
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f'{path}: expected one exact replacement, found {count}: {old[:90]!r}')
    p.write_text(source.replace(old, new, 1))


def section(path: str, start: str, end: str, replacement: str):
    p = Path(path)
    source = p.read_text()
    if source.count(start) != 1 or source.count(end) != 1:
        raise RuntimeError(f'{path}: invalid section {start[:70]!r} / {end[:70]!r}')
    first = source.index(start)
    last = source.index(end, first)
    p.write_text(source[:first] + replacement + source[last:])


change('src/hud/chat/visual.rs', '    ui_widgets::TextInput,\n', '')
change('src/hud/chat/visual.rs', '                    TextInput,\n', '')
change('src/ui/numeric_input.rs', '    ui_widgets::TextInput,\n', '')
change('src/ui/numeric_input.rs', '        TextInput,\n', '')
change('src/screens/settings_screen/spawn_biome_section/layout.rs', 'ui_widgets::{ScrollArea, TextInput}', 'ui_widgets::ScrollArea')
change('src/screens/settings_screen/spawn_biome_section/layout.rs', '                                TextInput,\n', '')

# The inventory was still manually pushing KeyboardInput and depended on the
# legacy TextInputState, now removed. Keep only its filter query in the view;
# EditableText owns text selection, caret, clipboard, composition, and editing.
section('src/hud/inventory/state.rs', 'use crate::ui::text_input::TextInputState;\n\n', 'pub(super) const SLOT_SIZE:', '')
section('src/hud/inventory/state.rs', '#[derive(Resource, Default)]\npub(super) struct CreativeInventoryView', '#[derive(Resource, Default)]\npub(super) struct CreativeScrollState', '''#[derive(Resource, Default)]
pub(super) struct CreativeInventoryView {
    search: String,
    search_focused: bool,
    selected_category: Option<String>,
}

impl CreativeInventoryView {
    pub(super) fn search_query(&self) -> &str {
        &self.search
    }

    pub(super) fn search_focused(&self) -> bool {
        self.search_focused
    }

    pub(super) fn selected_category(&self) -> Option<&str> {
        self.selected_category.as_deref()
    }

    pub(super) fn focus_search(&mut self) {
        self.search_focused = true;
    }

    pub(super) fn blur_search(&mut self) {
        self.search_focused = false;
    }

    pub(super) fn set_search_query(&mut self, query: String) {
        if self.search != query {
            self.search = query;
        }
    }

    pub(super) fn select_category(&mut self, category: Option<&str>) {
        let category = category.map(str::to_owned);
        if self.selected_category != category {
            self.selected_category = category;
        }
    }
}

''')
section('src/hud/inventory/interaction.rs', 'use bevy::{', 'use super::state::{', '''use bevy::{
    ecs::system::SystemParam,
    input::mouse::{MouseScrollUnit, MouseWheel},
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
};

use crate::{
    player::{
        hotbar::PlayerHotbar,
        inventory::{InventoryCursor, InventoryState},
    },
    ui::text_input::editable_value,
};

''')
section('src/hud/inventory/interaction.rs', 'pub(super) fn handle_search_focus(', 'pub(super) fn handle_inventory_close_shortcut(', '''pub(super) fn handle_search_focus(
    mut creative_view: ResMut<CreativeInventoryView>,
    search_bars: Query<&Interaction, (With<CreativeSearchBar>, Changed<Interaction>)>,
    editor: Query<Entity, With<CreativeSearchBar>>,
    mut focus: ResMut<InputFocus>,
) {
    if search_bars.iter().any(|interaction| *interaction == Interaction::Pressed)
        && let Ok(entity) = editor.single()
    {
        creative_view.focus_search();
        focus.set(entity, FocusCause::Pressed);
    }
}

''')
section('src/hud/inventory/interaction.rs', 'pub(super) fn handle_search_input(', 'pub(super) fn handle_category_clicks(', '''pub(super) fn handle_search_input(
    editor: Query<(Entity, &EditableText), With<CreativeSearchBar>>,
    focus: Res<InputFocus>,
    mut creative_view: ResMut<CreativeInventoryView>,
    mut scroll_state: ResMut<CreativeScrollState>,
    mut ui_dirty: ResMut<CreativeInventoryUiDirty>,
) {
    let Ok((entity, editable)) = editor.single() else {
        return;
    };
    let focused = focus.get() == Some(entity);
    if focused && !creative_view.search_focused() {
        creative_view.focus_search();
    } else if !focused && creative_view.search_focused() {
        creative_view.blur_search();
    }

    let next = editable_value(editable);
    if next != creative_view.search_query() {
        creative_view.set_search_query(next);
        scroll_state.catalog_y = 0.0;
        ui_dirty.mark();
    }
}

pub(super) fn sync_search_focus(
    creative_view: Res<CreativeInventoryView>,
    mut focus: ResMut<InputFocus>,
    editor: Query<Entity, With<CreativeSearchBar>>,
) {
    if !creative_view.search_focused()
        && let Ok(entity) = editor.single()
        && focus.get() == Some(entity)
    {
        focus.clear();
    }
}

''')

change('src/hud/inventory/layout.rs', 'use bevy::prelude::*;', 'use bevy::{prelude::*, text::{EditableText, TextCursorStyle}};')
section('src/hud/inventory/layout.rs', 'fn spawn_search_bar(', 'fn spawn_category_list(', '''fn spawn_search_bar(
    parent: &mut ChildSpawnerCommands,
    creative_view: &CreativeInventoryView,
    localization: &UiLocalization,
    language: Language,
) {
    let search_border = if creative_view.search_focused() {
        surface::HUD_SELECTED_BORDER_COLOR
    } else {
        surface::HUD_BORDER_COLOR
    };
    let placeholder_visible = creative_view.search_query().is_empty()
        && !creative_view.search_focused();
    let placeholder = localization.text(language, "inventory.searchPlaceholder");

    parent
        .spawn((
            Button,
            CreativeSearchBar,
            EditableText {
                max_characters: Some(128),
                ..EditableText::new(creative_view.search_query())
            },
            TextCursorStyle { color: theme::TEXT_PRIMARY, ..default() },
            TextFont { font: FontSource::SystemUi, font_size: FontSize::Px(17.0), ..default() },
            TextColor(theme::TEXT_PRIMARY),
            TextLayout::no_wrap(),
            Node {
                width: px(creative_content_width()),
                height: px(SEARCH_HEIGHT),
                padding: UiRect::horizontal(px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme::HUD_SURFACE),
            BorderColor::all(search_border),
        ))
        .with_children(|search| {
            search.spawn((
                CreativeSearchText,
                typography::hud(placeholder),
                if placeholder_visible { Visibility::Inherited } else { Visibility::Hidden },
                Pickable::IGNORE,
            ));
        });
}

''')
change('src/hud/inventory/sync.rs', "search_text: Query<'w, 's, &'static mut Text, With<CreativeSearchText>>,", "search_text: Query<'w, 's, &'static mut Visibility, With<CreativeSearchText>>,")
section('src/hud/inventory/sync.rs', '    let next_search_text = if inputs.panel.creative_view.search_query().is_empty() {', '    let Some((catalog_entity, mut position, children))', '''    let show_placeholder = inputs.panel.creative_view.search_query().is_empty()
        && !inputs.panel.creative_view.search_focused();
    let next_visibility = if show_placeholder {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut view.search_text {
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
    }

''')
change('src/hud/inventory.rs', '    handle_search_focus, handle_search_input, handle_search_select_all, handle_slot_clicks,', '    handle_search_focus, handle_search_input, handle_slot_clicks, sync_search_focus,')
change('src/hud/inventory.rs', '                    handle_search_select_all,\n', '')
change('src/hud/inventory.rs', '                    handle_empty_inventory_click,\n', '                    handle_empty_inventory_click,\n                    sync_search_focus,\n')

print('Corrected unavailable TextInput markers and migrated inventory search to EditableText.')
