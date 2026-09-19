use bevy::prelude::*;

pub const GROUP_GAP: f32 = 18.0;
pub const SETTING_GAP: f32 = 8.0;

pub fn group_column() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(GROUP_GAP),
        ..default()
    }
}

pub fn setting_column() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(SETTING_GAP),
        ..default()
    }
}
