use bevy::{prelude::*, ui::InteractionDisabled};

use super::theme;

pub(crate) fn sync_selectable_button(
    commands: &mut Commands,
    entity: Entity,
    active: bool,
    disabled: bool,
    interaction: Interaction,
    mut background: Mut<'_, BackgroundColor>,
) {
    if active && !disabled {
        commands.entity(entity).insert(InteractionDisabled);
    } else if !active && disabled {
        commands.entity(entity).remove::<InteractionDisabled>();
    }

    let next_background = BackgroundColor(selectable_button_background(active, interaction));
    if *background != next_background {
        *background = next_background;
    }
}

pub(crate) fn selectable_button_background(active: bool, interaction: Interaction) -> Color {
    if active {
        return theme::SURFACE_INSET;
    }

    match interaction {
        Interaction::Pressed => theme::PURPLE_HOVER,
        Interaction::Hovered => theme::PURPLE,
        Interaction::None => theme::PURPLE_SOFT,
    }
}

pub(crate) fn selectable_label_color(active: bool) -> Color {
    if active {
        theme::TEXT_MUTED
    } else {
        theme::TEXT_PRIMARY
    }
}
