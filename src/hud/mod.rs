mod block_icon;
mod crosshair;
mod fps;
mod hotbar;
mod inventory;
mod player;
mod targeting;
mod time;
mod underwater;
mod world;

use bevy::{
    camera::CameraOutputMode,
    prelude::*,
    render::render_resource::BlendState,
    ui::IsDefaultUiCamera,
};
use block_icon::BlockIconMaterial;
use crosshair::CrosshairPlugin;
use fps::FpsHudPlugin;
use hotbar::HotbarHudPlugin;
use inventory::InventoryHudPlugin;
use player::PlayerHudPlugin;
use targeting::TargetHudPlugin;
use time::TimeHudPlugin;
use underwater::UnderwaterTintPlugin;
use world::WorldHudPlugin;

use crate::{app::game_state::GameState, rendering::camera_stack::UI_CAMERA_ORDER};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TargetBlockPosition {
    #[default]
    Center,
    TopRight,
    Hidden,
}

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct HudSettings {
    display_tooltips: bool,
    target_block_position: TargetBlockPosition,
}

impl Default for HudSettings {
    fn default() -> Self {
        Self {
            display_tooltips: true,
            target_block_position: TargetBlockPosition::Center,
        }
    }
}

impl HudSettings {
    pub(crate) const fn display_tooltips(&self) -> bool {
        self.display_tooltips
    }

    pub(crate) fn set_display_tooltips(&mut self, display_tooltips: bool) {
        self.display_tooltips = display_tooltips;
    }

    pub(crate) const fn target_block_position(&self) -> TargetBlockPosition {
        self.target_block_position
    }

    pub(crate) fn set_target_block_position(&mut self, position: TargetBlockPosition) {
        self.target_block_position = position;
    }
}

pub(crate) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSettings>()
            .add_systems(OnEnter(GameState::Gameplay), spawn_gameplay_ui_camera)
            .add_plugins(UiMaterialPlugin::<BlockIconMaterial>::default())
            .add_plugins((
                UnderwaterTintPlugin,
                CrosshairPlugin,
                HotbarHudPlugin,
                InventoryHudPlugin,
                PlayerHudPlugin,
                TimeHudPlugin,
                FpsHudPlugin,
                WorldHudPlugin,
                TargetHudPlugin,
            ));
    }
}

fn spawn_gameplay_ui_camera(mut commands: Commands) {
    commands.spawn((
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
