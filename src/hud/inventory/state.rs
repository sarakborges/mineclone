use bevy::prelude::*;

pub(super) const SLOT_SIZE: f32 = 44.0;
pub(super) const ITEM_ICON_SIZE: f32 = 34.0;
pub(super) const SLOT_GAP: f32 = 4.0;
pub(super) const SECTION_GAP: f32 = 18.0;
pub(super) const PANEL_GAP: f32 = 24.0;
pub(super) const PANEL_PADDING: f32 = 18.0;
pub(super) const SEARCH_HEIGHT: f32 = 40.0;
pub(super) const SEARCH_GAP: f32 = 14.0;
pub(super) const CATEGORY_WIDTH: f32 = 172.0;
pub(super) const CATEGORY_ROW_HEIGHT: f32 = SLOT_SIZE;
pub(super) const CATEGORY_ICON_SIZE: f32 = 28.0;
pub(super) const CATEGORY_GAP: f32 = 12.0;
pub(super) const CREATIVE_VISIBLE_ROWS: usize = 5;
pub(super) const CREATIVE_COLUMNS: usize = 9;
pub(super) const SCROLLBAR_TOTAL_WIDTH: f32 = 14.0;
pub(super) const TRASH_GAP: f32 = 10.0;
pub(super) const CREATIVE_GRID_HEIGHT: f32 =
    CREATIVE_VISIBLE_ROWS as f32 * SLOT_SIZE + (CREATIVE_VISIBLE_ROWS - 1) as f32 * SLOT_GAP;

#[derive(Component)]
pub(super) struct InventoryHudRoot;

#[derive(Component)]
pub(super) struct InventorySlot {
    pub(super) index: usize,
    pub(super) item: Option<&'static str>,
}

#[derive(Component)]
pub(super) struct InventoryTrashButton;

#[derive(Component)]
pub(super) struct CreativeInventorySlot {
    pub(super) item: Option<&'static str>,
}

#[derive(Component)]
pub(super) struct CreativeSearchBar;

#[derive(Component)]
pub(super) struct CreativeSearchText;

#[derive(Component)]
pub(super) struct CreativeCategoryButton {
    pub(super) id: Option<String>,
}

#[derive(Component)]
pub(super) struct CreativeCategoryScrollArea;

#[derive(Component)]
pub(super) struct CreativeCatalogScrollArea;

#[derive(Component)]
pub(super) struct CreativeCategoryScrollbar;

#[derive(Component)]
pub(super) struct CreativeCatalogScrollbar;

#[derive(Component)]
pub(super) struct InventoryCursorIcon;

#[derive(Component)]
pub(super) struct InventoryItemTooltip;

#[derive(Component)]
pub(super) struct InventoryItemTooltipText;

#[derive(Resource, Default)]
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

#[derive(Resource, Default)]
pub(super) struct CreativeScrollState {
    pub(super) category_y: f32,
    pub(super) catalog_y: f32,
}

#[derive(Resource, Default)]
pub(super) struct CreativeInventoryUiDirty(bool);

impl CreativeInventoryUiDirty {
    pub(super) fn mark(&mut self) {
        self.0 = true;
    }

    pub(super) fn take(&mut self) -> bool {
        std::mem::take(&mut self.0)
    }
}
