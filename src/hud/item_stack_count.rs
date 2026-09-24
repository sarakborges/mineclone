use bevy::prelude::*;

use crate::ui::typography;

pub(crate) fn spawn_item_stack_count(parent: &mut ChildSpawnerCommands, quantity: u32) {
    if quantity <= 1 {
        return;
    }

    parent.spawn((
        typography::crosshair_hint(quantity.to_string()),
        TextLayout::no_wrap(),
        Node {
            position_type: PositionType::Absolute,
            right: px(2),
            bottom: px(0),
            ..default()
        },
        Pickable::IGNORE,
    ));
}
