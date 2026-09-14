use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry,
        inventory_category::InventoryCategoryRegistry, tool::ToolRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::{HOTBAR_INVENTORY_OFFSET, PlayerHotbar},
        inventory::InventoryCursor,
    },
    ui::surface,
    world::biome_field::BiomeField,
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
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    tools: Res<'w, ToolRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    language: Res<'w, ActiveLanguage>,
}

impl InventoryItemContent<'_> {
    fn view<'a>(
        &'a self,
        player_position: Vec2,
        icon_materials: &'a mut Assets<BlockIconMaterial>,
    ) -> InventoryItemView<'a> {
        InventoryItemView {
            asset_server: &self.asset_server,
            blocks: &self.blocks,
            tools: &self.tools,
            biomes: &self.biomes,
            biome_field: &self.biome_field,
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
    cursor: Res<InventoryCursor>,
    player: Single<&Transform, With<GameplayCamera>>,
    window: Single<&Window>,
    roots: Query<Entity, With<InventoryHudRoot>>,
    icons: Query<Entity, With<InventoryCursorIcon>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !cursor.is_changed() {
        return;
    }

    for entity in &icons {
        commands.entity(entity).despawn();
    }

    let Some(item_id) = cursor.item() else {
        return;
    };
    let Some(root_entity) = roots.iter().next() else {
        return;
    };
    let position = window.cursor_position().unwrap_or(Vec2::ZERO);
    let player_position = Vec2::new(player.translation.x, player.translation.z);
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
    categories: Res<InventoryCategoryRegistry>,
    localization: Res<UiLocalization>,
    state: InventoryPanelState,
    roots: Query<Entity, With<InventoryHudRoot>>,
    mut catalog_scroll: Query<
        (Entity, &mut ScrollPosition, Option<&Children>),
        With<CreativeCatalogScrollArea>,
    >,
    mut search_text: Query<&mut Text, With<CreativeSearchText>>,
    mut ui_dirty: ResMut<CreativeInventoryUiDirty>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let ui_changed = ui_dirty.take();
    let language_changed = content.language.is_changed();
    if !ui_changed && !language_changed {
        return;
    }

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
        cursor_position: None,
    };

    if language_changed {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        spawn_inventory_root(&mut commands, &layout, &mut items);
        return;
    }

    let next_search_text = if state.creative_view.search_query().is_empty() {
        localization.text(content.language.get(), "inventory.searchPlaceholder")
    } else {
        state.creative_view.search_query()
    };
    for mut text in &mut search_text {
        if text.0 != next_search_text {
            text.0 = next_search_text.to_owned();
        }
    }

    let Some((catalog_entity, mut position, children)) = catalog_scroll.iter_mut().next() else {
        return;
    };
    position.0.y = state.scroll_state.catalog_y;
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }

    commands.entity(catalog_entity).with_children(|scroll| {
        spawn_creative_catalog_rows(
            scroll,
            &categories,
            &state.creative_view,
            state.cursor.item(),
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
