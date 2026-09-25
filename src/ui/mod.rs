pub(crate) mod button;
pub(crate) mod cosmic_background;
pub(crate) mod dropdown;
pub(crate) mod numeric_input;
pub(crate) mod scrollbar;
pub(crate) mod screen;
pub(crate) mod selectable;
pub(crate) mod settings;
pub(crate) mod slider;
pub(crate) mod surface;
pub(crate) mod text_input;
pub(crate) mod theme;
pub(crate) mod toggle;
pub(crate) mod transition;
pub(crate) mod typography;
pub(crate) mod visibility;

use bevy::{prelude::*, text::FontCx};

#[derive(Resource)]
struct UiFontFamily(String);

pub(crate) struct UiDesignSystemPlugin;

impl Plugin for UiDesignSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<transition::ScreenTransition>()
            .add_systems(
                Startup,
                (resolve_ui_font_family, transition::spawn_transition_overlay),
            )
            .add_systems(
                Update,
                (
                    pin_ui_font_family,
                    button::animate_buttons,
                    cosmic_background::animate_stars,
                    transition::animate_screen_transition
                        .run_if(transition::screen_transition_active),
                ),
            )
            .add_systems(Last, scrollbar::sync_auto_scrollbars);
    }
}

fn resolve_ui_font_family(mut commands: Commands, mut font_context: ResMut<FontCx>) {
    let family = font_context
        .get_family(&FontSource::SystemUi)
        .expect("system UI font family should resolve when system font discovery is enabled")
        .to_owned();
    commands.insert_resource(UiFontFamily(family));
}

fn pin_ui_font_family(
    family: Res<UiFontFamily>,
    mut fonts: Query<&mut TextFont, Added<TextFont>>,
) {
    for mut font in &mut fonts {
        if font.font == FontSource::SystemUi {
            font.font = FontSource::Family(family.0.clone().into());
        }
    }
}
