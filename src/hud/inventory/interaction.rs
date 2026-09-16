use bevy::{
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

use super::state::{
    CreativeCatalogScrollArea, CreativeCatalogScrollbar, CreativeCategoryButton,
    CreativeCategoryScrollArea, CreativeCategoryScrollbar, CreativeInventorySlot,
    CreativeInventoryUiDirty, CreativeInventoryView, CreativeScrollState, CreativeSearchBar,
    InventorySlot, InventoryTrashButton, SLOT_GAP, SLOT_SIZE,
};

pub(super) fn handle_search_focus(
    mut creative_view: ResMut<CreativeInventoryView>,
    search_bars: Query<&Interaction, (With<CreativeSearchBar>, Changed<Interaction>)>,
    editor: Query<Entity, With<CreativeSearchBar>>,
    mut focus: ResMut<InputFocus>,
) {
    if search_bars
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
        && let Ok(entity) = editor.single()
    {
        creative_view.focus_search();
        focus.set(entity, FocusCause::Pressed);
    }
}

pub(super) fn handle_inventory_close_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    creative_view: Res<CreativeInventoryView>,
    mut next_inventory: ResMut<NextState<InventoryState>>,
) {
    if keys.just_pressed(KeyCode::KeyE) && !creative_view.search_focused() {
        next_inventory.set(InventoryState::Closed);
    }
}

pub(super) fn handle_search_input(
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

pub(super) fn handle_category_clicks(
    mut creative_view: ResMut<CreativeInventoryView>,
    mut scroll_state: ResMut<CreativeScrollState>,
    mut ui_dirty: ResMut<CreativeInventoryUiDirty>,
    categories: Query<(&Interaction, &CreativeCategoryButton), Changed<Interaction>>,
) {
    for (interaction, category) in &categories {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let previous_category = creative_view.selected_category().map(str::to_owned);
        creative_view.blur_search();
        creative_view.select_category(category.id.as_deref());
        if creative_view.selected_category() != previous_category.as_deref() {
            scroll_state.catalog_y = 0.0;
            ui_dirty.mark();
        }
        break;
    }
}

pub(super) fn handle_creative_scroll(
    mut wheel: MessageReader<MouseWheel>,
    category_buttons: Query<&Interaction, With<CreativeCategoryButton>>,
    category_scrollbars: Query<&Interaction, With<CreativeCategoryScrollbar>>,
    mut category_scroll: Query<
        &mut ScrollPosition,
        (
            With<CreativeCategoryScrollArea>,
            Without<CreativeCatalogScrollArea>,
        ),
    >,
    mut catalog_scroll: Query<
        &mut ScrollPosition,
        (
            With<CreativeCatalogScrollArea>,
            Without<CreativeCategoryScrollArea>,
        ),
    >,
) {
    let mut delta = 0.0;
    for event in wheel.read() {
        delta += match event.unit {
            MouseScrollUnit::Line => -event.y * (SLOT_SIZE + SLOT_GAP),
            MouseScrollUnit::Pixel => -event.y,
        };
    }

    if delta == 0.0 {
        return;
    }

    let pointer_is_over_categories = category_buttons
        .iter()
        .chain(category_scrollbars.iter())
        .any(|interaction| *interaction != Interaction::None);

    if pointer_is_over_categories {
        for mut position in &mut category_scroll {
            position.0.y = (position.0.y + delta).max(0.0);
        }
    } else {
        for mut position in &mut catalog_scroll {
            position.0.y = (position.0.y + delta).max(0.0);
        }
    }
}

pub(super) fn handle_creative_slot_clicks(
    mut cursor: ResMut<InventoryCursor>,
    mut creative_view: ResMut<CreativeInventoryView>,
    slots: Query<(&Interaction, &CreativeInventorySlot), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if creative_view.search_focused() {
            creative_view.blur_search();
        }
        if let Some(item) = slot.item {
            cursor.pick_creative_item(item);
        }
        break;
    }
}

pub(super) fn handle_slot_clicks(
    mut hotbar: ResMut<PlayerHotbar>,
    mut cursor: ResMut<InventoryCursor>,
    mut creative_view: ResMut<CreativeInventoryView>,
    slots: Query<(&Interaction, &InventorySlot), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if creative_view.search_focused() {
            creative_view.blur_search();
        }
        cursor.click_slot(&mut hotbar, slot.index);
        break;
    }
}

pub(super) fn handle_inventory_trash_clicks(
    mut cursor: ResMut<InventoryCursor>,
    buttons: Query<&Interaction, (With<InventoryTrashButton>, Changed<Interaction>)>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            cursor.discard();
            break;
        }
    }
}

#[derive(SystemParam)]
pub(super) struct InventoryControlInteractions<'w, 's> {
    categories: Query<'w, 's, &'static Interaction, With<CreativeCategoryButton>>,
    creative_slots: Query<'w, 's, &'static Interaction, With<CreativeInventorySlot>>,
    inventory_slots: Query<'w, 's, &'static Interaction, With<InventorySlot>>,
    search_bars: Query<'w, 's, &'static Interaction, With<CreativeSearchBar>>,
    trash_buttons: Query<'w, 's, &'static Interaction, With<InventoryTrashButton>>,
    category_scrollbars: Query<'w, 's, &'static Interaction, With<CreativeCategoryScrollbar>>,
    catalog_scrollbars: Query<'w, 's, &'static Interaction, With<CreativeCatalogScrollbar>>,
}

impl InventoryControlInteractions<'_, '_> {
    fn pointer_is_over_control(&self) -> bool {
        self.categories
            .iter()
            .chain(self.creative_slots.iter())
            .chain(self.inventory_slots.iter())
            .chain(self.search_bars.iter())
            .chain(self.trash_buttons.iter())
            .chain(self.category_scrollbars.iter())
            .chain(self.catalog_scrollbars.iter())
            .any(|interaction| *interaction != Interaction::None)
    }
}

pub(super) fn handle_empty_inventory_click(
    mouse: Res<ButtonInput<MouseButton>>,
    controls: InventoryControlInteractions,
    mut cursor: ResMut<InventoryCursor>,
) {
    if !mouse.just_pressed(MouseButton::Left) || cursor.item().is_none() {
        return;
    }

    if !controls.pointer_is_over_control() {
        cursor.discard();
    }
}

pub(super) fn remember_creative_scroll_positions(
    category_scroll: Query<&ScrollPosition, With<CreativeCategoryScrollArea>>,
    catalog_scroll: Query<&ScrollPosition, With<CreativeCatalogScrollArea>>,
    mut state: ResMut<CreativeScrollState>,
) {
    if let Some(position) = category_scroll.iter().next() {
        state.category_y = position.0.y;
    }
    if let Some(position) = catalog_scroll.iter().next() {
        state.catalog_y = position.0.y;
    }
}
