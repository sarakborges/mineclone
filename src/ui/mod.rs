pub mod button;
pub mod surface;
pub mod theme;
pub mod transition;
pub mod typography;

use bevy::prelude::*;

pub struct UiDesignSystemPlugin;

impl Plugin for UiDesignSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<transition::ScreenTransition>()
            .add_systems(Startup, transition::spawn_transition_overlay)
            .add_systems(
                Update,
                (
                    button::animate_buttons,
                    transition::animate_screen_transition,
                ),
            );
    }
}
