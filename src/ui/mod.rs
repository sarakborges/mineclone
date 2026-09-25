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
struct UiFontFaces {
    family: String,
    handles: Vec<UiFontFaceHandle>,
}

struct UiFontFaceHandle {
    weight: FontWeight,
    width: FontWidth,
    style: FontStyle,
    handle: Handle<Font>,
}

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
                    pin_ui_font_handles,
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
    commands.insert_resource(UiFontFaces {
        family,
        handles: Vec::new(),
    });
}

fn pin_ui_font_handles(
    mut ui_fonts: ResMut<UiFontFaces>,
    mut font_context: ResMut<FontCx>,
    mut font_assets: ResMut<Assets<Font>>,
    mut text_fonts: Query<&mut TextFont, Added<TextFont>>,
) {
    for mut text_font in &mut text_fonts {
        if !matches!(
            &text_font.font,
            FontSource::SystemUi | FontSource::Family(family) if family.as_str() == ui_fonts.family
        ) {
            continue;
        }

        if let Some(cached) = ui_fonts.handles.iter().find(|cached| {
            cached.weight == text_font.weight
                && cached.width == text_font.width
                && cached.style == text_font.style
        }) {
            text_font.font = FontSource::Handle(cached.handle.clone());
            continue;
        }

        let family = font_context
            .collection
            .family_by_name(&ui_fonts.family)
            .unwrap_or_else(|| panic!("resolved system UI family disappeared: {}", ui_fonts.family));
        let face = family
            .match_font(text_font.width, text_font.style, text_font.weight, true)
            .unwrap_or_else(|| {
                panic!(
                    "system UI family {} has no face matching {:?} {:?} {:?}",
                    ui_fonts.family, text_font.weight, text_font.width, text_font.style
                )
            });
        let data = face
            .load(Some(&mut font_context.context.source_cache))
            .unwrap_or_else(|| panic!("failed to load system UI font face from {}", ui_fonts.family));
        let handle = font_assets.add(Font {
            data,
            alias: String::new(),
        });

        ui_fonts.handles.push(UiFontFaceHandle {
            weight: text_font.weight,
            width: text_font.width,
            style: text_font.style,
            handle: handle.clone(),
        });
        text_font.font = FontSource::Handle(handle);
    }
}
