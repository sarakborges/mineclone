use bevy::prelude::*;

use crate::{
    content::{
        biome::BiomeRegistry,
        block::{BlockRegistry, BlockTint},
        block_orientation::BlockOrientation,
        item::ItemRegistry,
        layer::LayerRegistry,
        object::ObjectRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
    },
    localization::Language,
    player::camera::GameplayCamera,
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    tools::BrushMode,
    world::biome_field::BiomeField,
};

use super::{block_icon::BlockIconMaterial, tool_icon::spawn_tool_icon};

#[derive(Component)]
pub(crate) struct HudBlockIcon {
    pub(crate) placement_slot: Option<usize>,
    pub(crate) orientation: BlockOrientation,
}

#[derive(Component)]
pub(crate) struct HudBiomeTintIcon(pub(crate) BlockTint);

pub(crate) struct HudItemIconView<'a> {
    pub(crate) asset_server: &'a AssetServer,
    pub(crate) items: &'a ItemRegistry,
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) layers: &'a LayerRegistry,
    pub(crate) objects: &'a ObjectRegistry,
    pub(crate) tools: &'a ToolRegistry,
    pub(crate) dyes: &'a SecondaryPropertyRegistry,
    pub(crate) brush_mode: &'a BrushMode,
    pub(crate) biomes: &'a BiomeRegistry,
    pub(crate) biome_field: &'a BiomeField,
    pub(crate) player_position: Vec2,
    pub(crate) language: Language,
    pub(crate) icon_materials: &'a mut Assets<BlockIconMaterial>,
}

pub(crate) fn spawn_hud_item_icon(
    parent: &mut ChildSpawnerCommands,
    item_id: &'static str,
    items: &mut HudItemIconView<'_>,
    size: f32,
    image_override: Option<&str>,
    placement_slot: Option<usize>,
) {
    if let Some(icon) = image_override {
        parent.spawn((
            ImageNode::new(items.asset_server.load(icon.to_owned())),
            icon_node(size),
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(item) = items.items.get(item_id) {
        parent.spawn((
            ImageNode::new(items.asset_server.load(item.icon.clone())),
            icon_node(size),
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(object) = items.objects.get(item_id) {
        let tint = block_tint_at(
            object.tint,
            items.player_position,
            items.biome_field,
            items.biomes,
        );
        parent.spawn((
            HudBiomeTintIcon(object.tint),
            ImageNode {
                color: tint,
                ..ImageNode::new(items.asset_server.load(object.icon.clone()))
            },
            icon_node(size),
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(block) = items.blocks.get(item_id) {
        let orientation = block.default_orientation();
        let tint = block_tint_at(
            block.tint,
            items.player_position,
            items.biome_field,
            items.biomes,
        );
        let material = items.icon_materials.add(BlockIconMaterial::from_block(
            block,
            items.asset_server,
            tint,
        ));

        parent.spawn((
            HudBlockIcon {
                placement_slot,
                orientation,
            },
            BlockModel::display(item_id),
            MaterialNode(material),
            icon_node(size),
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(layer) = items.layers.get(item_id) {
        let tint = block_tint_at(
            layer.tint,
            items.player_position,
            items.biome_field,
            items.biomes,
        );
        parent.spawn((
            HudBiomeTintIcon(layer.tint),
            ImageNode {
                color: tint,
                ..ImageNode::new(items.asset_server.load(layer.texture.clone()))
            },
            icon_node(size),
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(tool) = items.tools.get(item_id) {
        spawn_tool_icon(
            parent,
            tool,
            items.asset_server,
            items.brush_mode,
            items.dyes,
            items.language,
            size,
        );
        return;
    }

    panic!("HUD references missing item: {item_id}");
}

pub(crate) fn sync_hud_biome_tint_icons(
    player: Single<&Transform, With<GameplayCamera>>,
    biomes: Res<BiomeRegistry>,
    biome_field: Res<BiomeField>,
    mut tint_cell: Local<Option<IVec2>>,
    mut icons: Query<(&HudBiomeTintIcon, &mut ImageNode)>,
) {
    let next_cell = IVec2::new(
        player.translation.x.floor() as i32,
        player.translation.z.floor() as i32,
    );
    if *tint_cell == Some(next_cell) && !biomes.is_changed() && !biome_field.is_changed() {
        return;
    }
    *tint_cell = Some(next_cell);

    let position = next_cell.as_vec2() + Vec2::splat(0.5);
    for (tint, mut image) in &mut icons {
        let color = block_tint_at(tint.0, position, &biome_field, &biomes);
        if image.color != color {
            image.color = color;
        }
    }
}

fn icon_node(size: f32) -> Node {
    Node {
        width: px(size),
        height: px(size),
        ..default()
    }
}
