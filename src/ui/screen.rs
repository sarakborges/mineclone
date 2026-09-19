use bevy::prelude::*;

pub const CONTENT_WIDTH: f32 = 1120.0;
pub const HEADER_HEIGHT: f32 = 116.0;
pub const FOOTER_HEIGHT: f32 = 104.0;
pub const ACTION_GAP: f32 = 18.0;

const BODY_PADDING_X: f32 = 32.0;
const BODY_PADDING_Y: f32 = 18.0;

pub fn header() -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: px(0),
        left: px(0),
        right: px(0),
        height: px(HEADER_HEIGHT),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

pub fn body() -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: px(HEADER_HEIGHT),
        bottom: px(FOOTER_HEIGHT),
        left: px(0),
        right: px(0),
        padding: UiRect::axes(px(BODY_PADDING_X), px(BODY_PADDING_Y)),
        align_items: AlignItems::Stretch,
        justify_content: JustifyContent::Center,
        min_height: px(0),
        ..default()
    }
}

pub fn content() -> Node {
    Node {
        width: px(CONTENT_WIDTH),
        max_width: percent(100),
        height: percent(100),
        min_height: px(0),
        ..default()
    }
}

pub fn content_row(column_gap: f32) -> Node {
    let mut node = content();
    node.flex_direction = FlexDirection::Row;
    node.align_items = AlignItems::Stretch;
    node.column_gap = px(column_gap);
    node
}

pub fn content_column(row_gap: f32) -> Node {
    let mut node = content();
    node.flex_direction = FlexDirection::Column;
    node.align_items = AlignItems::Stretch;
    node.row_gap = px(row_gap);
    node
}

pub fn footer() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        right: px(0),
        bottom: px(0),
        height: px(FOOTER_HEIGHT),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: px(ACTION_GAP),
        ..default()
    }
}
