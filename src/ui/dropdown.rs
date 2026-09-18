use bevy::prelude::*;

use super::theme;

const CHEVRON_WIDTH: f32 = 14.0;
const CHEVRON_HEIGHT: f32 = 8.0;
const CHEVRON_STROKE_WIDTH: f32 = 8.0;
const CHEVRON_STROKE_HEIGHT: f32 = 2.0;
const CHEVRON_ANGLE: f32 = 0.58;

pub(crate) fn indicator() -> impl Bundle {
    (
        Node {
            width: px(CHEVRON_WIDTH),
            height: px(CHEVRON_HEIGHT),
            position_type: PositionType::Relative,
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(2),
                    width: px(CHEVRON_STROKE_WIDTH),
                    height: px(CHEVRON_STROKE_HEIGHT),
                    ..default()
                },
                BackgroundColor(theme::TEXT_MUTED),
                Transform::from_rotation(Quat::from_rotation_z(CHEVRON_ANGLE)),
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    top: px(2),
                    width: px(CHEVRON_STROKE_WIDTH),
                    height: px(CHEVRON_STROKE_HEIGHT),
                    ..default()
                },
                BackgroundColor(theme::TEXT_MUTED),
                Transform::from_rotation(Quat::from_rotation_z(-CHEVRON_ANGLE)),
            ),
        ],
    )
}
