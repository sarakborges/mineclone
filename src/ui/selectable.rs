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
        return Color::srgba(0.08, 0.07, 0.12, 0.62);
    }

    match interaction {
        Interaction::Pressed => Color::srgba(0.34, 0.22, 0.62, 0.92),
        Interaction::Hovered => Color::srgba(0.29, 0.19, 0.54, 0.82),
        Interaction::None => Color::srgba(0.20, 0.14, 0.38, 0.72),
    }
}

pub(crate) fn selectable_label_color(active: bool) -> Color {
    if active {
        theme::TEXT_SUBTLE
    } else {
        theme::TEXT_PRIMARY
    }
}
