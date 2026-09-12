pub(crate) mod button;
pub(crate) mod cosmic_background;
pub(crate) mod scrollbar;
pub(crate) mod surface;
pub(crate) mod theme;
pub(crate) mod transition;
pub(crate) mod typography;

use bevy::prelude::*;

pub(crate) struct UiDesignSystemPlugin;

impl Plugin for UiDesignSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<transition::ScreenTransition>()
            .add_systems(Startup, transition::spawn_transition_overlay)
            .add_systems(
                Update,
                (
                    button::animate_buttons,
                    cosmic_background::animate_stars,
                    transition::animate_screen_transition,
                ),
            )
            .add_systems(Last, scrollbar::sync_auto_scrollbars);
    }
}
