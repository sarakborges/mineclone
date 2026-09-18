use std::marker::PhantomData;

use bevy::prelude::*;

use super::{selectable, theme};

pub const CONTROL_HEIGHT: f32 = 44.0;
pub const OPTION_HEIGHT: f32 = 40.0;
pub const PANEL_GAP: f32 = 6.0;

const CONTROL_BORDER_WIDTH: f32 = 2.0;
const CONTROL_PADDING_X: f32 = 12.0;
const OPTION_PADDING_X: f32 = 10.0;
const CHEVRON_WIDTH: f32 = 14.0;
const CHEVRON_HEIGHT: f32 = 8.0;
const CHEVRON_STROKE_WIDTH: f32 = 8.0;
const CHEVRON_STROKE_HEIGHT: f32 = 2.0;
const CHEVRON_ANGLE: f32 = 0.58;

#[derive(Clone, Copy)]
pub enum PanelAnchor {
    Left,
    Right,
}

#[derive(Component)]
pub struct DropdownInside<M: Send + Sync + 'static>(PhantomData<M>);

impl<M: Send + Sync + 'static> Default for DropdownInside<M> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Resource)]
pub struct DropdownState<M: Send + Sync + 'static> {
    open: bool,
    marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> Default for DropdownState<M> {
    fn default() -> Self {
        Self {
            open: false,
            marker: PhantomData,
        }
    }
}

impl<M: Send + Sync + 'static> DropdownState<M> {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn reset(&mut self) {
        self.close();
    }
}

pub fn root(width: Val) -> Node {
    Node {
        position_type: PositionType::Relative,
        width,
        height: px(CONTROL_HEIGHT),
        flex_shrink: 0.0,
        ..default()
    }
}

pub fn control<M: Send + Sync + 'static>() -> impl Bundle {
    let (background, border) = selectable::static_colors(false);
    (
        DropdownInside::<M>::default(),
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::horizontal(px(CONTROL_PADDING_X)),
            border: UiRect::all(px(CONTROL_BORDER_WIDTH)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(border),
    )
}

pub fn panel_node(
    width: Val,
    padding: f32,
    row_gap: f32,
    border_width: f32,
    anchor: PanelAnchor,
) -> Node {
    let (left, right) = match anchor {
        PanelAnchor::Left => (px(0), Val::Auto),
        PanelAnchor::Right => (Val::Auto, px(0)),
    };
    Node {
        display: Display::None,
        position_type: PositionType::Absolute,
        top: px(CONTROL_HEIGHT + PANEL_GAP),
        left,
        right,
        width,
        padding: UiRect::all(px(padding)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(row_gap),
        border: UiRect::all(px(border_width)),
        ..default()
    }
}

pub fn panel_surface<M: Send + Sync + 'static>() -> impl Bundle {
    (
        DropdownInside::<M>::default(),
        Interaction::default(),
        BackgroundColor(theme::HUD_SURFACE),
        BorderColor::all(theme::BORDER),
    )
}

pub fn option<M: Send + Sync + 'static>(selected: bool, border_width: f32) -> impl Bundle {
    let (background, border) = selectable::static_colors(selected);
    (
        DropdownInside::<M>::default(),
        Node {
            width: percent(100),
            height: px(OPTION_HEIGHT),
            min_height: px(OPTION_HEIGHT),
            padding: UiRect::horizontal(px(OPTION_PADDING_X)),
            border: UiRect::all(px(border_width)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(border),
    )
}

pub fn inside<M: Send + Sync + 'static>() -> DropdownInside<M> {
    DropdownInside::default()
}

pub fn clicked_outside<M: Send + Sync + 'static>(
    open: bool,
    mouse: &ButtonInput<MouseButton>,
    inside: &Query<&Interaction, With<DropdownInside<M>>>,
) -> bool {
    open
        && mouse.just_pressed(MouseButton::Left)
        && !inside
            .iter()
            .any(|interaction| *interaction != Interaction::None)
}

pub fn indicator() -> impl Bundle {
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
