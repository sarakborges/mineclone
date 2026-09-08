pub mod button;
pub mod theme;
pub mod typography;

use bevy::prelude::*;

pub struct UiDesignSystemPlugin;

impl Plugin for UiDesignSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, button::animate_buttons);
    }
}
