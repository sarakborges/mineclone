use bevy::prelude::*;

use crate::content::layer::LayerDefinition;

pub(crate) fn spawn_layer_icon(
    parent: &mut ChildSpawnerCommands,
    layer: &LayerDefinition,
    asset_server: &AssetServer,
    tint: Color,
    size: f32,
) {
    parent.spawn((
        ImageNode {
            color: tint,
            ..ImageNode::new(asset_server.load(layer.texture.clone()))
        },
        Node {
            width: px(size),
            height: px(size),
            ..default()
        },
        Pickable::IGNORE,
    ));
}
