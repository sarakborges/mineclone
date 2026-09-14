use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{inventory_category::InventoryCategoryRegistry, tool::ToolRegistry},
    localization::{ActiveLanguage, UiLocalization},
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::{HOTBAR_INVENTORY_OFFSET, PlayerHotbar},
        inventory::InventoryCursor,
    },
    rendering::block_visual_content::BlockVisualContent,
    ui::surface,
};

use crate::hud::block_icon::BlockIconMaterial;

use super::{
    layout::{
        InventoryItemView, InventoryLayoutState, spawn_creative_catalog_rows, spawn_cursor_icon,
        spawn_inventory_item, spawn_inventory_root,
    },
    state::{
        CreativeCatalogScrollArea, CreativeCategoryButton, CreativeInventorySlot,
        CreativeInventoryUiDirty, CreativeInventoryView, CreativeScrollState, CreativeSearchBar,
        CreativeSearchText, InventoryCursorIcon, InventoryHudRoot, InventorySlot,
        InventoryTrashButton, ITEM_ICON_SIZE,
    },
};

#[derive(SystemParam)]
pub(super) struct InventoryItemContent<'w> {
    visual: BlockVisualContent<'w>,
    tools: Res<'w, ToolRegistry>,
    language: Res<'w, ActiveLanguage>,
}

impl InventoryItemContent<'_> {
    fn view<'a>(
        &'a self,
        player_position: Vec2,
        icon_materials: &'a mut Assets<BlockIconMaterial>,
    ) -> InventoryItemView<'a> {
        InventoryItemView {
            asset_server: &self.visual.asset_server,
            blocks: &self.visual.blocks,
            tools: &self.tools,
            biomes: &self.visual.biomes,
            biome_field: &self.visual.biome_field,
            player_position,
            language: self.language.get(),
            icon_materials,
        }
    }
}

#[derive(SystemParam)]
pub(super) struct InventoryPanelState<'w, 's> {
    hotbar: Res<'w, PlayerHotbar>,
    cursor: Res<'w, InventoryCursor>,
    creative_view: Res<'w, CreativeInventoryView>,
    scroll_state: Res<'w, CreativeScrollState>,
    player: Single<'w, 's, (&'static Transform, &'static GameMode), With<GameplayCamera>>,
}

#[derive(SystemParam)]
pub(super) struct InventoryCursorSyncContext<'w, 's> {
    cursor: Res<'w, InventoryCursor>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    window: Single<'w, 's, &'static Window>,
    roots: Query<'w, 's, Entity, With<InventoryHudRoot>>,
    icons: Query<'w, 's, Entity, With<InventoryCursorIcon>>,
}

#[derive(SystemParam)]
pub(super) struct InventoryRebuildInputs<'w, 's> {
    categories: Res<'w, InventoryCategoryRegistry>,
    localization: Res<'w, UiLocalization>,
    panel: InventoryPanelState<'w, 's>,
    dirty: ResMut<'w, CreativeInventoryUiDirty>,
}

#[derive(SystemParam)]
pub(super) struct InventoryRebuildView<'w, 's> {
    roots: Query<'w, 's, Entity, With<InventoryHudRoot>>,
    catalog_scroll: Query<
        'w,
        's,
        (Entity, &'static mut ScrollPosition, Option<&'static Children>),
        With<CreativeCatalogScrollArea>,
    >,
    search_text: Query<'w, 's, &'static mut Text, With<CreativeSearchText>>,
}

pub(super) fn spawn_inventory(
    mut commands: Commands,
    content: InventoryItemContent,
    categories: Res<InventoryCategoryRegistry>,
    localization: Res<UiLocalization>,
    state: InventoryPanelState,
    window: Single<&Window>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let (player_transform, game_mode) = *state.player;
    let player_position = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    let mut items = content.view(player_position, &mut icon_materials);
    let layout = InventoryLayoutState {
        categories: &categories,
        hotbar: &state.hotbar,
        cursor: &state.cursor,
        creative_view: &state.creative_view,
        scroll_state: &state.scroll_state,
        localization: &localization,
        game_mode: *game_mode,
        cursor_position: window.cursor_position(),
    };

    spawn_inventory_root(&mut commands, &layout, &mut items);
}

pub(super) fn sync_inventory_cursor_icon(
    mut commands: Commands,
    content: InventoryItemContent,
    context: InventoryCursorSyncContext,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !context.cursor.is_changed() {
        return;
    }

    for entity in &context.icons {
        commands.entity(entity).despawn();
    }

    let Some(item_id) = context.cursor.item() else {
        return;
    };
    let Some(root_entity) = context.roots.iter().next() else {
        return;
    };
    let position = context.window.cursor_position().unwrap_or(Vec2::ZERO);
    let player_position = Vec2::new(
        context.player.translation.x,
        context.player.translation.z,
    );
    let mut items = content.view(player_position, &mut icon_materials);

    commands.entity(root_entity).with_children(|root| {
        spawn_cursor_icon(root, item_id, position, &mut items);
    });
}

pub(super) fn sync_inventory_slot_contents(
    mut commands: Commands,
    content: InventoryItemContent,
    hotbar: Res<PlayerHotbar>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut slots: Query<(Entity, &mut InventorySlot, Option<&Children>)>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !hotbar.is_changed() {
        return;
    }

    let player_position = Vec2::new(player.translation.x, player.translation.z);
    let mut items = content.view(player_position, &mut icon_materials);

    for (entity, mut slot, children) in &mut slots {
        let item = hotbar.inventory_item_at(slot.index);
        if slot.item == item {
            continue;
        }

        if let Some(children) = children {
            for &child in children {
                commands.entity(child).despawn();
            }
        }

        slot.item = item;
        let Some(item_id) = item else {
            continue;
        };

        commands.entity(entity).with_children(|slot_node| {
            spawn_inventory_item(slot_node, item_id, &mut items);
        });
    }
}

pub(super) fn rebuild_inventory_when_changed(
    mut commands: Commands,
    content: InventoryItemContent,
    mut inputs: InventoryRebuildInputs,
    mut view: InventoryRebuildView,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let ui_changed = inputs.dirty.take();
    let language_changed = content.language.is_changed();
    if !ui_changed && !language_changed {
        return;
    }

    let (player_transform, game_mode) = *inputs.panel.player;
    let player_position = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    let mut items = content.view(player_position, &mut icon_materials);
    let layout = InventoryLayoutState {
        categories: &inputs.categories,
        hotbar: &inputs.panel.hotbar,
        cursor: &inputs.panel.cursor,
        creative_view: &inputs.panel.creative_view,
        scroll_state: &inputs.panel.scroll_state,
        localization: &inputs.localization,
        game_mode: *game_mode,
        cursor_position: None,
    };

    if language_changed {
        for entity in &view.roots {
            commands.entity(entity).despawn();
        }
        spawn_inventory_root(&mut commands, &layout, &mut items);
        return;
    }

    let next_search_text = if inputs.panel.creative_view.search_query().is_empty() {
        inputs
            .localization
            .text(content.language.get(), "inventory.searchPlaceholder")
    } else {
        inputs.panel.creative_view.search_query()
    };
    for mut text in &mut view.search_text {
        if text.0 != next_search_text {
            text.0 = next_search_text.to_owned();
        }
    }

    let Some((catalog_entity, mut position, children)) = view.catalog_scroll.iter_mut().next()
    else {
        return;
    };
    position.0.y = inputs.panel.scroll_state.catalog_y;
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }

    commands.entity(catalog_entity).with_children(|scroll| {
        spawn_creative_catalog_rows(
            scroll,
            &inputs.categories,
            &inputs.panel.creative_view,
            inputs.panel.cursor.item(),
            &mut items,
        );
    });
}

pub(super) fn style_search_bar(
    creative_view: Res<CreativeInventoryView>,
    mut search_bars: Query<&mut BorderColor, With<CreativeSearchBar>>,
) {
    let color = if creative_view.search_focused() {
        surface::HUD_SELECTED_BORDER_COLOR
    } else {
        surface::HUD_BORDER_COLOR
    };
    for mut border in &mut search_bars {
        *border = BorderColor::all(color);
    }
}

pub(super) fn style_category_buttons(
    creative_view: Res<CreativeInventoryView>,
    mut buttons: Query<
        (
            &Interaction,
            &CreativeCategoryButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    for (interaction, category, mut background, mut border) in &mut buttons {
        let selected = creative_view.selected_category() == category.id.as_deref();
        apply_button_visual(*interaction, selected, &mut background, &mut border);
    }
}

pub(super) fn style_creative_slots(
    cursor: Res<InventoryCursor>,
    mut slots: Query<
        (
            &Interaction,
            &CreativeInventorySlot,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    for (interaction, slot, mut background, mut border) in &mut slots {
        let selected = slot.item.is_some() && slot.item == cursor.item();
        apply_button_visual(*interaction, selected, &mut background, &mut border);
    }
}

pub(super) fn style_inventory_slots(
    hotbar: Res<PlayerHotbar>,
    mut slots: Query<
        (
            &Interaction,
            &InventorySlot,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    let selected_index = HOTBAR_INVENTORY_OFFSET + hotbar.selected_slot();
    for (interaction, slot, mut background, mut border) in &mut slots {
        apply_button_visual(
            *interaction,
            slot.index == selected_index,
            &mut background,
            &mut border,
        );
    }
}

pub(super) fn style_inventory_trash_button(
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (With<Button>, With<InventoryTrashButton>),
    >,
) {
    for (interaction, mut background, mut border) in &mut buttons {
        let (background_color, border_color) = surface::hud_danger_control_colors(*interaction);
        background.0 = background_color;
        *border = BorderColor::all(border_color);
    }
}

fn apply_button_visual(
    interaction: Interaction,
    selected: bool,
    background: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    let (background_color, border_color) = surface::hud_control_colors(interaction, selected);
    background.0 = background_color;
    *border = BorderColor::all(border_color);
}

pub(super) fn update_cursor_icon_position(
    window: Single<&Window>,
    mut icons: Query<&mut Node, With<InventoryCursorIcon>>,
) {
    let Some(position) = window.cursor_position() else {
        return;
    };

    for mut node in &mut icons {
        node.left = px(position.x - ITEM_ICON_SIZE * 0.5);
        node.top = px(position.y - ITEM_ICON_SIZE * 0.5);
    }
}
