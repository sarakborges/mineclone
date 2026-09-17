use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
};

use crate::ui::{text_input, theme};

use super::state::{CreativeInventoryView, CreativeSearchBar, CreativeSearchText, SEARCH_HEIGHT};

#[derive(Component)]
pub(super) struct CreativeSearchFrame;

/// The inventory panel owns its existing editor entity, focus and search state.
/// Insert one frame in the editor's original layout slot, then move the editor
/// and placeholder inside it. A wrapper is necessary because EditableText
/// renders its text/caret on its own node and cannot provide its own padding.
pub(super) fn frame_inventory_search_field(
    mut commands: Commands,
    editors: Query<(Entity, &ChildOf), Added<CreativeSearchBar>>,
    children: Query<&Children>,
    hints: Query<(), With<CreativeSearchText>>,
    mut nodes: Query<&mut Node, With<CreativeSearchBar>>,
) {
    for (editor, parent) in &editors {
        let parent_entity = parent.parent();
        let Some(index) = children
            .get(parent_entity)
            .ok()
            .and_then(|siblings| siblings.iter().position(|sibling| sibling == editor))
        else {
            continue;
        };
        let hint = children
            .get(editor)
            .ok()
            .and_then(|children| children.iter().find(|child| hints.contains(*child)));

        let frame = commands
            .spawn((
                Button,
                CreativeSearchFrame,
                Node {
                    position_type: PositionType::Relative,
                    width: nodes.get(editor).map_or(Val::Auto, |node| node.width),
                    height: px(SEARCH_HEIGHT),
                    padding: UiRect::horizontal(px(text_input::INPUT_PADDING_X)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(text_input::INPUT_RADIUS)),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(text_input::INPUT_FILL),
                BorderColor::all(text_input::input_border(false)),
            ))
            .id();
        commands.entity(parent_entity).insert_children(index, &[frame]);
        commands.entity(frame).add_child(editor);
        if let Some(hint) = hint {
            commands.entity(frame).add_child(hint);
        }
        if let Ok(mut editor_node) = nodes.get_mut(editor) {
            editor_node.width = percent(100);
            editor_node.min_width = px(0);
            // Center the actual line box; centering a 100%-height editor does
            // not center its glyphs or its caret within the frame.
            editor_node.height = px(text_input::INPUT_EDITOR_HEIGHT);
            editor_node.padding = UiRect::default();
            editor_node.border = UiRect::default();
            editor_node.border_radius = BorderRadius::default();
            editor_node.align_items = AlignItems::Center;
            editor_node.overflow = Overflow::clip();
        }
        commands.entity(editor).remove::<(BackgroundColor, BorderColor)>();
    }
}

/// The border and its padding are clickable without changing the actual
/// EditableText entity used by every existing search and focus system.
pub(super) fn focus_inventory_search_frame(
    frames: Query<&Interaction, (With<CreativeSearchFrame>, Changed<Interaction>)>,
    editor: Query<Entity, With<CreativeSearchBar>>,
    mut focus: ResMut<InputFocus>,
    mut view: ResMut<CreativeInventoryView>,
) {
    if frames.iter().any(|interaction| *interaction == Interaction::Pressed)
        && let Ok(entity) = editor.single()
    {
        view.focus_search();
        focus.set(entity, FocusCause::Pressed);
    }
}

/// Only the frame receives the border and padding. The placeholder uses the
/// same horizontal and vertical line-box origin as the editor.
pub(super) fn style_inventory_search_field(
    view: Res<CreativeInventoryView>,
    mut frames: Query<(&mut BackgroundColor, &mut BorderColor), With<CreativeSearchFrame>>,
    mut placeholders: Query<(&mut Node, &mut TextColor), With<CreativeSearchText>>,
) {
    let fill = BackgroundColor(text_input::INPUT_FILL);
    let border = BorderColor::all(text_input::input_border(view.search_focused()));
    for (mut background, mut current_border) in &mut frames {
        if *background != fill {
            *background = fill;
        }
        if *current_border != border {
            *current_border = border;
        }
    }
    for (mut node, mut color) in &mut placeholders {
        let left = px(text_input::INPUT_PADDING_X + 1.0);
        let top = px(text_input::centered_text_top(SEARCH_HEIGHT));
        if node.position_type != PositionType::Absolute {
            node.position_type = PositionType::Absolute;
        }
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }
        if color.0 != theme::TEXT_MUTED {
            color.0 = theme::TEXT_MUTED;
        }
    }
}
