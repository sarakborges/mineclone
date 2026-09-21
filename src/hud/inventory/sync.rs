use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::{
        inventory_category::InventoryCategoryRegistry, layer::LayerRegistry,
        secondary_property::SecondaryPropertyRegistry, tool::ToolRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::{HOTBAR_INVENTORY_OFFSET, PlayerHotbar},
        inventory::InventoryCursor,
    },
    rendering::block_visual_content::BlockVisualContent,
    tools::BrushMode,
    ui::selectable,
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
        CreativeSearchText, ITEM_ICON_SIZE, InventoryCursorIcon, InventoryHudRoot, InventorySlot,
        InventoryTrashButton,
    },
};

#[derive(SystemParam)]
pub(super) struct InventoryItemContent<'w> {
    visual: BlockVisualContent<'w>,
    layers: Res<'w, LayerRegistry>,
    tools: Res<'w, ToolRegistry>,
    dyes: Res<'w, SecondaryPropertyRegistry>,
    brush_mode: Res<'w, BrushMode>,
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
            layers: &self.layers,
            tools: &self.tools,
            dyes: &self.dyes,
            brush_mode: &self.brush_mode,
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
        (
            Entity,
            &'static mut ScrollPosition,
            Option<&'static Children>,
        ),
        With<CreativeCatalogScrollArea>,
    >,
    search_text: Query<'w, 's, &'static mut Visibility, With<CreativeSearchText>>,
}

pub(super) type InventoryTrashButtonQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
    ),
    (
        With<Button>,
        With<InventoryTrashButton>,
        Changed<Interaction>,
    ),
>;

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
    let player_position = Vec2::new(context.player.translation.x, context.player.translation.z);
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

    let show_placeholder = inputs.panel.creative_view.search_query().is_empty()
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

    let Some((catalog_entity, mut position, children)) = view.catalog_scroll.iter_mut().next()
    else {
        return;
    };
    let next_scroll_y = inputs.panel.scroll_state.catalog_y;
    if position.0.y != next_scroll_y {
        position.0.y = next_scroll_y;
    }
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
    if !creative_view.is_changed() {
        return;
    }

    let color = if creative_view.search_focused() {
        selectable::SELECTED_BORDER_COLOR
    } else {
        selectable::BORDER_COLOR
    };
    let next_border = BorderColor::all(color);
    for mut border in &mut search_bars {
        if *border != next_border {
            *border = next_border;
        }
    }
}

pub(super) fn style_category_buttons(
    creative_view: Res<CreativeInventoryView>,
    mut buttons: Query<
        (
            Ref<Interaction>,
            &CreativeCategoryButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    let selection_changed = creative_view.is_changed();

    for (interaction, category, background, border) in &mut buttons {
        if !selection_changed && !interaction.is_changed() {
            continue;
        }

        let selected = creative_view.selected_category() == category.id.as_deref();
        selectable::apply_colors(
            selectable::colors(*interaction, selected),
            background,
            border,
        );
    }
}

pub(super) fn style_creative_slots(
    cursor: Res<InventoryCursor>,
    mut slots: Query<
        (
            Ref<Interaction>,
            &CreativeInventorySlot,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    let selection_changed = cursor.is_changed();

    for (interaction, slot, background, border) in &mut slots {
        if !selection_changed && !interaction.is_changed() {
            continue;
        }

        let selected = slot.item.is_some() && slot.item == cursor.item();
        selectable::apply_colors(
            selectable::colors(*interaction, selected),
            background,
            border,
        );
    }
}

pub(super) fn style_inventory_slots(
    hotbar: Res<PlayerHotbar>,
    mut slots: Query<
        (
            Ref<Interaction>,
            &InventorySlot,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
) {
    let selection_changed = hotbar.is_changed();
    let selected_index = HOTBAR_INVENTORY_OFFSET + hotbar.selected_slot();

    for (interaction, slot, background, border) in &mut slots {
        if !selection_changed && !interaction.is_changed() {
            continue;
        }

        selectable::apply_colors(
            selectable::colors(*interaction, slot.index == selected_index),
            background,
            border,
        );
    }
}

pub(super) fn style_inventory_trash_button(mut buttons: InventoryTrashButtonQuery) {
    for (interaction, background, border) in &mut buttons {
        selectable::apply_colors(
            selectable::danger_colors(*interaction),
            background,
            border,
        );
    }
}

pub(super) fn update_cursor_icon_position(
    window: Single<&Window>,
    mut icons: Query<&mut Node, With<InventoryCursorIcon>>,
) {
    let Some(position) = window.cursor_position() else {
        return;
    };

    let left = px(position.x - ITEM_ICON_SIZE * 0.5);
    let top = px(position.y - ITEM_ICON_SIZE * 0.5);
    for mut node in &mut icons {
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }
    }
}
