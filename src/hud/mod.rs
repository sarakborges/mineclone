mod block_icon;
pub(crate) mod chat;
mod character_info;
mod crosshair;
mod entity_card;
mod entity_targeting;
mod fps;
mod hotbar;
mod inventory;
mod layer_icon;
mod player;
mod targeting;
mod time;
mod tool_icon;
mod underwater;
mod world;

use bevy::{
    camera::CameraOutputMode,
    prelude::*,
    render::render_resource::BlendState,
    ui::IsDefaultUiCamera,
};
use serde::{Deserialize, Serialize};
use block_icon::BlockIconMaterial;
use chat::ChatHudPlugin;
use character_info::CharacterInfoHudPlugin;
use crosshair::CrosshairPlugin;
use entity_targeting::EntityHudPlugin;
use fps::FpsHudPlugin;
use hotbar::HotbarHudPlugin;
use inventory::InventoryHudPlugin;
use player::PlayerHudPlugin;
use targeting::TargetHudPlugin;
use time::TimeHudPlugin;
use underwater::UnderwaterTintPlugin;
use world::WorldHudPlugin;

use crate::{
    app::game_state::GameState,
    rendering::camera_stack::UI_CAMERA_ORDER,
    targeting::block::BlockTargetingSet,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetBlockPosition {
    #[default]
    Center,
    TopRight,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HintKind {
    OpenInventory,
    CloseInventory,
    RotateBlock,
    BreakBlock,
    BreakOrPlaceBlock,
    BrushPaint,
    BrushClear,
    Chisel,
    Shears,
    StructureTool,
}

impl HintKind {
    pub(crate) const fn localization_key(self) -> &'static str {
        match self {
            Self::OpenInventory => "settings.hint.openInventory",
            Self::CloseInventory => "settings.hint.closeInventory",
            Self::RotateBlock => "settings.hint.rotateBlock",
            Self::BreakBlock => "settings.hint.breakBlock",
            Self::BreakOrPlaceBlock => "settings.hint.breakOrPlaceBlock",
            Self::BrushPaint => "settings.hint.brushPaint",
            Self::BrushClear => "settings.hint.brushClear",
            Self::Chisel => "settings.hint.chisel",
            Self::Shears => "settings.hint.shears",
            Self::StructureTool => "settings.hint.structureTool",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
struct HintSettings {
    open_inventory: bool,
    close_inventory: bool,
    rotate_block: bool,
    break_block: bool,
    break_or_place_block: bool,
    brush_paint: bool,
    brush_clear: bool,
    chisel: bool,
    shears: bool,
    structure_tool: bool,
}

impl Default for HintSettings {
    fn default() -> Self {
        Self {
            open_inventory: true,
            close_inventory: true,
            rotate_block: true,
            break_block: true,
            break_or_place_block: true,
            brush_paint: true,
            brush_clear: true,
            chisel: true,
            shears: true,
            structure_tool: true,
        }
    }
}

impl HintSettings {
    const fn enabled(self, kind: HintKind) -> bool {
        match kind {
            HintKind::OpenInventory => self.open_inventory,
            HintKind::CloseInventory => self.close_inventory,
            HintKind::RotateBlock => self.rotate_block,
            HintKind::BreakBlock => self.break_block,
            HintKind::BreakOrPlaceBlock => self.break_or_place_block,
            HintKind::BrushPaint => self.brush_paint,
            HintKind::BrushClear => self.brush_clear,
            HintKind::Chisel => self.chisel,
            HintKind::Shears => self.shears,
            HintKind::StructureTool => self.structure_tool,
        }
    }

    fn set(&mut self, kind: HintKind, enabled: bool) {
        match kind {
            HintKind::OpenInventory => self.open_inventory = enabled,
            HintKind::CloseInventory => self.close_inventory = enabled,
            HintKind::RotateBlock => self.rotate_block = enabled,
            HintKind::BreakBlock => self.break_block = enabled,
            HintKind::BreakOrPlaceBlock => self.break_or_place_block = enabled,
            HintKind::BrushPaint => self.brush_paint = enabled,
            HintKind::BrushClear => self.brush_clear = enabled,
            HintKind::Chisel => self.chisel = enabled,
            HintKind::Shears => self.shears = enabled,
            HintKind::StructureTool => self.structure_tool = enabled,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct HudSettings {
    hide_hints: bool,
    hints: HintSettings,
    target_block_position: TargetBlockPosition,
}

impl Default for HudSettings {
    fn default() -> Self {
        Self {
            hide_hints: false,
            hints: HintSettings::default(),
            target_block_position: TargetBlockPosition::Center,
        }
    }
}

impl HudSettings {
    pub(crate) const fn hide_hints(&self) -> bool {
        self.hide_hints
    }

    pub(crate) fn set_hide_hints(&mut self, hide_hints: bool) {
        self.hide_hints = hide_hints;
    }

    pub(crate) const fn hint_preference(&self, kind: HintKind) -> bool {
        self.hints.enabled(kind)
    }

    pub(crate) const fn hint_enabled(&self, kind: HintKind) -> bool {
        !self.hide_hints && self.hints.enabled(kind)
    }

    pub(crate) fn set_hint_preference(&mut self, kind: HintKind, enabled: bool) {
        self.hints.set(kind, enabled);
    }

    pub(crate) const fn target_block_position(&self) -> TargetBlockPosition {
        self.target_block_position
    }

    pub(crate) fn set_target_block_position(&mut self, position: TargetBlockPosition) {
        self.target_block_position = position;
    }
}

#[derive(Component)]
pub(crate) struct GameplayUiCamera;

pub(crate) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSettings>()
            .add_systems(OnEnter(GameState::Gameplay), spawn_gameplay_ui_camera)
            .add_systems(
                Update,
                (
                    tool_icon::sync_brush_tint_icons,
                    entity_card::sync_entity_cards.after(BlockTargetingSet::Raycast),
                )
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_plugins(UiMaterialPlugin::<BlockIconMaterial>::default())
            .add_plugins((
                UnderwaterTintPlugin,
                CharacterInfoHudPlugin,
                CrosshairPlugin,
                HotbarHudPlugin,
                InventoryHudPlugin,
                PlayerHudPlugin,
                ChatHudPlugin,
                TimeHudPlugin,
                FpsHudPlugin,
                WorldHudPlugin,
                TargetHudPlugin,
                EntityHudPlugin,
            ));
    }
}

fn spawn_gameplay_ui_camera(mut commands: Commands) {
    commands.spawn((
        GameplayUiCamera,
        Camera2d,
        Camera {
            order: UI_CAMERA_ORDER,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        IsDefaultUiCamera,
        DespawnOnExit(GameState::Gameplay),
    ));
}
