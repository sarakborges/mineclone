use bevy::prelude::*;

use super::theme;

pub(crate) fn sync_selectable_button(
    _entity: Entity,
    active: bool,
    _disabled: bool,
    interaction: Interaction,
    mut background: Mut<'_, BackgroundColor>,
    mut border: Mut<'_, BorderColor>,
) {
    let next_background = BackgroundColor(selectable_button_background(active, interaction));
    if *background != next_background {
        *background = next_background;
    }
    let next_border = BorderColor::all(selectable_button_border(active, interaction));
    if *border != next_border {
        *border = next_border;
    }
}

pub(crate) fn selectable_button_border(active: bool, interaction: Interaction) -> Color {
    if active {
        return match interaction {
            Interaction::Pressed => theme::BORDER_STRONG,
            Interaction::Hovered => theme::BORDER_STRONG,
            Interaction::None => theme::BORDER_FOCUS,
        };
    }

    match interaction {
        Interaction::Pressed | Interaction::Hovered => theme::BORDER_STRONG,
        Interaction::None => theme::BORDER,
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
    if active { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED }
}
