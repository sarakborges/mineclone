use bevy::prelude::*;

use crate::{
    content::{
        builtin_ids::{BRUSH_TOOL_ID, DYED_PROPERTY_ID},
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolDefinition,
    },
    localization::Language,
    tools::BrushMode,
    ui::typography,
};

#[derive(Component)]
pub(super) struct BrushTintIcon;

fn selected_brush_tint(mode: &BrushMode, properties: &SecondaryPropertyRegistry) -> Option<Color> {
    mode.dye_id()
        .and_then(|dye| properties.get(DYED_PROPERTY_ID, dye))
        .map(|definition| definition.color.to_color())
}

/// Spawn the original image, then an independently tinted overlay only for a dyed brush.
pub(crate) fn spawn_tool_icon(
    parent: &mut ChildSpawnerCommands,
    tool: &ToolDefinition,
    asset_server: &AssetServer,
    mode: &BrushMode,
    properties: &SecondaryPropertyRegistry,
    language: Language,
    size: f32,
) {
    if tool.icon.is_empty() {
        parent.spawn((
            typography::caption(tool.name.text(language)),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ));
        return;
    }

    parent
        .spawn((
            Node {
                position_type: PositionType::Relative,
                width: px(size),
                height: px(size),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|icon| {
            let full_size = Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            };
            icon.spawn((
                ImageNode::new(asset_server.load(tool.icon.clone())),
                full_size.clone(),
                Pickable::IGNORE,
            ));

            if tool.id == BRUSH_TOOL_ID
                && let Some(tint_icon) = &tool.tint_icon
            {
                let tint = selected_brush_tint(mode, properties);
                icon.spawn((
                    BrushTintIcon,
                    ImageNode {
                        color: tint.unwrap_or(Color::WHITE),
                        ..ImageNode::new(asset_server.load(tint_icon.clone()))
                    },
                    full_size,
                    if tint.is_some() {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    Pickable::IGNORE,
                ));
            }
        });
}

/// Changing palette selection refreshes existing inventory and hotbar overlays without a rebuild.
pub(crate) fn sync_brush_tint_icons(
    mode: Res<BrushMode>,
    properties: Res<SecondaryPropertyRegistry>,
    mut overlays: Query<(&mut ImageNode, &mut Visibility), With<BrushTintIcon>>,
) {
    if !mode.is_changed() && !properties.is_changed() {
        return;
    }

    let tint = selected_brush_tint(&mode, &properties);
    let visibility = if tint.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut image, mut current_visibility) in &mut overlays {
        if *current_visibility != visibility {
            *current_visibility = visibility;
        }
        if let Some(color) = tint
            && image.color != color
        {
            image.color = color;
        }
    }
}
