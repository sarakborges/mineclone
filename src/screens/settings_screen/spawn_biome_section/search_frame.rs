use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
};

use crate::ui::text_input;

use super::layout::{SpawnBiomeSearchBar, SpawnBiomeSearchFrame};

/// Clicking the padded border should still focus the actual editable child.
pub(in crate::screens::settings_screen) fn focus_spawn_biome_search_frame(
    frames: Query<&Interaction, (Changed<Interaction>, With<SpawnBiomeSearchFrame>)>,
    editor: Single<Entity, With<SpawnBiomeSearchBar>>,
    mut focus: ResMut<InputFocus>,
) {
    if frames.iter().any(|interaction| *interaction == Interaction::Pressed) {
        focus.set(*editor, FocusCause::Pressed);
    }
}

pub(in crate::screens::settings_screen) fn sync_spawn_biome_search_frame(
    focus: Res<InputFocus>,
    editor: Single<Entity, With<SpawnBiomeSearchBar>>,
    mut frames: Query<&mut BorderColor, With<SpawnBiomeSearchFrame>>,
) {
    let border = BorderColor::all(text_input::input_border(focus.get() == Some(*editor)));
    for mut current in &mut frames {
        if *current != border {
            *current = border;
        }
    }
}
