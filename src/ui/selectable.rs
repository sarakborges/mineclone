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
        return match interaction {
            Interaction::Pressed => theme::PURPLE_HOVER,
            Interaction::Hovered => theme::PURPLE_HOVER,
            Interaction::None => theme::PURPLE,
        };
    }

    match interaction {
        Interaction::Pressed => theme::SURFACE_ELEVATED,
        Interaction::Hovered => theme::PURPLE_SOFT,
        Interaction::None => theme::SURFACE_INSET,
    }
}

pub(crate) fn selectable_label_color(active: bool) -> Color {
    theme::TEXT_PRIMARY
}
