mod interaction;
mod layout;
mod search_style;
mod state;
mod sync;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{
        block::BlockRegistry,
        builtin_ids::BUCKET_FLUID_METADATA_KEY,
        fluid::FluidRegistry,
        inventory_category::InventoryCategoryRegistry,
        item::{ItemRegistry, display_name_with_metadata},
        layer::LayerRegistry,
        object::ObjectRegistry,
        tool::ToolRegistry,
    },
    gameplay::modal::GameplayModalState,
    localization::{ActiveLanguage, UiLocalization},
    player::{hotbar::PlayerHotbar, inventory::InventoryCursor},
};

use crate::hud::block_icon::BlockIconMaterial;

use interaction::{
    handle_category_clicks, handle_creative_scroll, handle_creative_slot_clicks,
    handle_empty_inventory_click, handle_inventory_close_shortcut, handle_inventory_sort_clicks,
    handle_inventory_trash_clicks, handle_inventory_view_toggle_clicks, handle_player_search_focus,
    handle_player_search_input, handle_search_focus, handle_search_input, handle_slot_clicks,
    remember_creative_scroll_positions, sync_player_search_focus, sync_search_focus,
};
use layout::spawn_character_info_inventory;
use search_style::{
    focus_inventory_search_frame, frame_inventory_search_field, style_inventory_search_field,
    style_player_inventory_search_field,
};
use state::{
    CreativeInventorySlot, CreativeInventoryUiDirty, CreativeInventoryView, CreativeScrollState,
    InventoryItemTooltipText, InventorySlot, PlayerInventoryView,
};
use sync::{
    InventoryItemContent, InventoryPanelState, rebuild_inventory_when_changed, spawn_inventory,
    style_category_buttons, style_creative_slots, style_inventory_slots, style_inventory_trash_button,
    style_search_bar, sync_inventory_cursor_icon, sync_inventory_item_tooltip,
    sync_inventory_slot_contents, sync_inventory_sort_tooltip, update_cursor_icon_position,
};

const BUCKET_TOOL_ID: &str = "asteria:bucket";

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InventoryHudSet {
    Input,
    Sync,
    Style,
}

#[derive(Component)]
pub(super) struct CharacterInfoInventoryRoot;

pub(super) const INVENTORY_SLOT_SIZE: f32 = state::SLOT_SIZE;
pub(super) const INVENTORY_SLOT_GAP: f32 = state::SLOT_GAP;
pub(super) const INVENTORY_PANEL_BORDER_WIDTH: f32 = state::PANEL_BORDER_WIDTH;

#[derive(SystemParam)]
pub(super) struct CharacterInfoInventorySpawn<'w, 's> {
    content: InventoryItemContent<'w>,
    categories: Res<'w, InventoryCategoryRegistry>,
    localization: Res<'w, UiLocalization>,
    panel: InventoryPanelState<'w, 's>,
    window: Single<'w, 's, &'static Window>,
    icon_materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

impl CharacterInfoInventorySpawn<'_, '_> {
    pub(super) fn spawn(&mut self, root: &mut ChildSpawnerCommands) {
        let mut items = self
            .content
            .view(self.panel.player_position(), &mut self.icon_materials);
        let layout = self.panel.layout(
            &self.categories,
            &self.localization,
            self.window.cursor_position(),
        );

        spawn_character_info_inventory(root, &layout, &mut items);
    }
}

#[derive(SystemParam)]
struct MetadataDisplayContent<'w> {
    items: Res<'w, ItemRegistry>,
    blocks: Res<'w, BlockRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
    fluids: Res<'w, FluidRegistry>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
}

impl MetadataDisplayContent<'_> {
    fn name(&self, item_id: &str, metadata: Option<(&str, &str)>) -> String {
        display_name_with_metadata(
            item_id,
            metadata,
            &self.items,
            &self.blocks,
            &self.layers,
            &self.objects,
            &self.tools,
            &self.fluids,
            &self.localization,
            self.language.get(),
        )
    }
}

pub(super) struct InventoryHudPlugin;

impl Plugin for InventoryHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreativeInventoryView>()
            .init_resource::<PlayerInventoryView>()
            .init_resource::<CreativeScrollState>()
            .init_resource::<CreativeInventoryUiDirty>()
            .configure_sets(
                Update,
                (
                    InventoryHudSet::Input,
                    InventoryHudSet::Sync,
                    InventoryHudSet::Style,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(GameplayModalState::Inventory),
                spawn_inventory.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameplayModalState::Inventory),
                (
                    reset_resource::<CreativeInventoryView>,
                    reset_resource::<PlayerInventoryView>,
                    reset_resource::<CreativeScrollState>,
                    reset_resource::<CreativeInventoryUiDirty>,
                ),
            )
            .add_systems(
                OnExit(GameplayModalState::CharacterInfo),
                (
                    reset_resource::<InventoryCursor>,
                    reset_resource::<PlayerInventoryView>,
                ),
            )
            .add_systems(
                Update,
                (
                    remember_creative_scroll_positions,
                    handle_inventory_view_toggle_clicks,
                    handle_search_focus,
                    focus_inventory_search_frame,
                    handle_player_search_focus,
                    handle_inventory_close_shortcut.run_if(in_state(GameplayModalState::Inventory)),
                    handle_search_input,
                    handle_player_search_input,
                    handle_category_clicks,
                    handle_creative_scroll,
                    handle_creative_slot_clicks,
                    handle_slot_clicks,
                    handle_inventory_sort_clicks,
                    handle_inventory_trash_clicks,
                    handle_empty_inventory_click,
                    sync_search_focus,
                    sync_player_search_focus,
                )
                    .chain()
                    .in_set(InventoryHudSet::Input)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(inventory_ui_open),
            )
            .add_systems(
                Update,
                (
                    sync_inventory_cursor_icon,
                    sync_inventory_slot_contents,
                    rebuild_inventory_when_changed,
                )
                    .chain()
                    .in_set(InventoryHudSet::Sync)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(inventory_ui_open),
            )
            .add_systems(
                Update,
                (
                    style_search_bar,
                    frame_inventory_search_field,
                    style_inventory_search_field,
                    style_player_inventory_search_field,
                    style_category_buttons,
                    style_creative_slots,
                    style_inventory_slots,
                    style_inventory_trash_button,
                    sync_inventory_sort_tooltip,
                    update_cursor_icon_position,
                    sync_inventory_item_tooltip,
                    sync_bucket_variant_tooltip,
                    localize_inventory_crafting_text,
                )
                    .chain()
                    .in_set(InventoryHudSet::Style)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(inventory_ui_open),
            );
    }
}

fn sync_bucket_variant_tooltip(
    content: MetadataDisplayContent,
    hotbar: Res<PlayerHotbar>,
    inventory_slots: Query<(&Interaction, &InventorySlot)>,
    creative_slots: Query<(&Interaction, &CreativeInventorySlot)>,
    mut tooltip_text: Single<&mut Text, With<InventoryItemTooltipText>>,
) {
    let inventory_bucket = inventory_slots.iter().find_map(|(interaction, slot)| {
        if *interaction == Interaction::None || slot.item != Some(BUCKET_TOOL_ID) {
            return None;
        }
        let stack = hotbar.inventory_stack_at(slot.index)?;
        let metadata = stack
            .metadata()
            .get(BUCKET_FLUID_METADATA_KEY)
            .map(|fluid| (BUCKET_FLUID_METADATA_KEY, fluid));
        Some(metadata)
    });

    let creative_bucket = creative_slots.iter().find_map(|(interaction, slot)| {
        (*interaction != Interaction::None && slot.item == Some(BUCKET_TOOL_ID))
            .then_some(slot.metadata)
    });

    let metadata = inventory_bucket.or(creative_bucket);
    let Some(metadata) = metadata else {
        return;
    };

    let next = content.name(BUCKET_TOOL_ID, metadata);
    if tooltip_text.0 != next {
        tooltip_text.0 = next;
    }
}

fn localize_inventory_crafting_text(
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
    mut texts: Query<&mut Text>,
) {
    let language = language.get();
    for mut text in &mut texts {
        let Some(next) = localized_inventory_crafting_text(&text.0, &localization, language) else {
            continue;
        };
        if text.0 != next {
            text.0 = next;
        }
    }
}

fn localized_inventory_crafting_text(
    source: &str,
    localization: &UiLocalization,
    language: crate::localization::Language,
) -> Option<String> {
    let exact_key = match source {
        "Inventory" => "inventory.title",
        "Creative" => "inventory.view.creative",
        "Crafting" => "crafting.title",
        "AVAILABLE RECIPES" => "crafting.availableRecipes",
        "Current Station" => "crafting.currentStation",
        "BASE STATION" => "crafting.baseStation",
        "Personal crafting" => "crafting.personalCrafting",
        "STATUS" => "crafting.status",
        "Available" => "crafting.available",
        "No recipes available." => "crafting.noRecipes",
        "Recipe" => "crafting.recipe",
        "Select a recipe." => "crafting.selectRecipe",
        "SELECTED RECIPE" => "crafting.selectedRecipe",
        "Ingredients" => "crafting.ingredients",
        "Craft Item" => "crafting.craftItem",
        "All materials available." => "crafting.allMaterialsAvailable",
        "Missing required materials." => "crafting.missingRequiredMaterials",
        "RESULT" => "crafting.result",
        "Material" => "crafting.material",
        "Recipe unavailable." => "crafting.recipeUnavailable",
        "Recipe unavailable in this environment." => "crafting.recipeUnavailableEnvironment",
        "Missing ingredients." => "crafting.missingIngredients",
        "Recipe result unavailable." => "crafting.resultUnavailable",
        "Not enough inventory space." => "crafting.inventoryFull",
        _ => "",
    };
    if !exact_key.is_empty() {
        return Some(localization.text(language, exact_key).to_owned());
    }

    if let Some(count) = source.strip_suffix(" recipe(s)") {
        return Some(localization.format(language, "crafting.recipeCount", &[("count", count)]));
    }
    if let Some(count) = source.strip_prefix("Creates ×") {
        return Some(localization.format(language, "crafting.creates", &[("count", count)]));
    }
    if let Some(count) = source.strip_suffix(" required") {
        return Some(localization.format(
            language,
            "crafting.requiredCount",
            &[("count", count)],
        ));
    }
    if let Some(count) = source.strip_prefix("Output ×") {
        return Some(localization.format(
            language,
            "crafting.outputCount",
            &[("count", count)],
        ));
    }
    if let Some(item) = source
        .strip_prefix("Crafted ")
        .and_then(|value| value.strip_suffix('.'))
    {
        return Some(localization.format(language, "crafting.crafted", &[("item", item)]));
    }

    None
}

fn inventory_ui_open(modal: Res<State<GameplayModalState>>) -> bool {
    modal.get().shows_inventory()
}
