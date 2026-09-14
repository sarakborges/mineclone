use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry,
        block::{BlockDefinition, BlockRegistry},
        block_orientation::BlockOrientation,
        tool::ToolRegistry,
    },
    hud::block_icon::BlockIconMaterial,
    localization::{ActiveLanguage, Language},
    player::{
        camera::GameplayCamera,
        hotbar::{HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::InventoryState,
    },
    rendering::{block_model::BlockModel, block_tint::block_tint_at},
    targeting::{PlacementOrientation, block::BlockTargetingSet},
    ui::{theme, typography, visibility::set_visibility},
    world::biome_field::BiomeField,
};

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 34.0;

#[derive(Component)]
struct HotbarHudRoot;

#[derive(Component)]
struct HotbarBlockModel {
    index: usize,
    orientation: BlockOrientation,
}

#[derive(Default)]
struct HotbarVisualCache {
    tint_cell: Option<IVec2>,
}

#[derive(SystemParam)]
struct HotbarHudContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    tools: Res<'w, ToolRegistry>,
    hotbar: Res<'w, PlayerHotbar>,
    inventory_state: Res<'w, State<InventoryState>>,
    language: Res<'w, ActiveLanguage>,
}

#[derive(SystemParam)]
struct HotbarVisualState<'w, 's> {
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    placement_orientation: Res<'w, PlacementOrientation>,
    hotbar: Res<'w, PlayerHotbar>,
}

#[derive(SystemParam)]
struct HotbarVisualContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
}

#[derive(SystemParam)]
struct HotbarVisualView<'w, 's> {
    icons: Query<
        'w,
        's,
        (
            &'static BlockModel,
            &'static mut HotbarBlockModel,
            &'static MaterialNode<BlockIconMaterial>,
        ),
    >,
    materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

pub struct HotbarHudPlugin;

impl Plugin for HotbarHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), sync_hotbar)
            .add_systems(Update, sync_hotbar.run_if(in_state(GameState::Gameplay)))
            .add_systems(
                Update,
                update_hotbar_item_visuals
                    .after(BlockTargetingSet::PlacementState)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(InventoryState::Open),
                set_visibility::<HotbarHudRoot, false>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(InventoryState::Closed),
                set_visibility::<HotbarHudRoot, true>.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn sync_hotbar(
    mut commands: Commands,
    content: HotbarHudContent,
    roots: Query<Entity, With<HotbarHudRoot>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let has_root = !roots.is_empty();
    if has_root && !content.hotbar.is_changed() && !content.language.is_changed() {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let visibility = if *content.inventory_state.get() == InventoryState::Open {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };

    spawn_hotbar_root(
        &mut commands,
        &content.asset_server,
        &content.blocks,
        &content.tools,
        &content.hotbar,
        content.language.get(),
        visibility,
        &mut icon_materials,
    );
}

fn spawn_hotbar_root(
    commands: &mut Commands,
    asset_server: &AssetServer,
    blocks: &BlockRegistry,
    tools: &ToolRegistry,
    hotbar: &PlayerHotbar,
    language: Language,
    visibility: Visibility,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let selected_name = hotbar
        .item_at(hotbar.selected_slot())
        .map(|item_id| item_name(item_id, blocks, tools, language))
        .unwrap_or("");

    commands
        .spawn((
            HotbarHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                bottom: px(18),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(8),
                ..default()
            },
            visibility,
            GlobalZIndex(10),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                typography::hud(selected_name),
                TextLayout::justify(Justify::Center),
                Node {
                    min_height: px(18),
                    ..default()
                },
            ));

            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                padding: UiRect::all(px(4)),
                ..default()
            })
            .with_children(|row| {
                for index in 0..HOTBAR_SLOT_COUNT {
                    let selected = index == hotbar.selected_slot();
                    let border_color = if selected {
                        theme::TEXT_PRIMARY
                    } else {
                        Color::srgba(0.70, 0.72, 0.82, 0.28)
                    };
                    let background = if selected {
                        Color::srgba(0.08, 0.07, 0.16, 0.94)
                    } else {
                        theme::HUD_SURFACE
                    };

                    row.spawn((
                        Node {
                            width: px(SLOT_SIZE),
                            height: px(SLOT_SIZE),
                            border: UiRect::all(px(2)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(background),
                        BorderColor::all(border_color),
                        Pickable::IGNORE,
                    ))
                    .with_children(|slot| {
                        let Some(item_id) = hotbar.item_at(index) else {
                            return;
                        };

                        if let Some(block) = blocks.get(item_id) {
                            spawn_block_icon(
                                slot,
                                index,
                                item_id,
                                block,
                                asset_server,
                                icon_materials,
                            );
                            return;
                        }

                        if let Some(tool) = tools.get(item_id) {
                            slot.spawn((
                                typography::caption(tool.name.text(language)),
                                TextLayout::justify(Justify::Center),
                                Pickable::IGNORE,
                            ));
                            return;
                        }

                        panic!("hotbar references missing item: {item_id}");
                    });
                }
            });
        });
}

fn spawn_block_icon(
    slot: &mut ChildSpawnerCommands,
    index: usize,
    block_id: &'static str,
    block: &BlockDefinition,
    asset_server: &AssetServer,
    icon_materials: &mut Assets<BlockIconMaterial>,
) {
    let orientation = block.default_orientation();
    let material = icon_materials.add(BlockIconMaterial::from_block(
        block,
        asset_server,
        Color::WHITE,
    ));

    slot.spawn((
        HotbarBlockModel { index, orientation },
        BlockModel::display(block_id),
        MaterialNode(material),
        Node {
            width: px(ITEM_ICON_SIZE),
            height: px(ITEM_ICON_SIZE),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

fn item_name<'a>(
    item_id: &'a str,
    blocks: &'a BlockRegistry,
    tools: &'a ToolRegistry,
    language: Language,
) -> &'a str {
    if let Some(block) = blocks.get(item_id) {
        return block.name.text(language);
    }
    if let Some(tool) = tools.get(item_id) {
        return tool.name.text(language);
    }
    item_id
}

fn update_hotbar_item_visuals(
    state: HotbarVisualState,
    content: HotbarVisualContent,
    mut cache: Local<HotbarVisualCache>,
    view: HotbarVisualView,
) {
    let HotbarVisualView {
        mut icons,
        mut materials,
    } = view;
    let tint_cell = IVec2::new(
        state.player.translation.x.floor() as i32,
        state.player.translation.z.floor() as i32,
    );
    let global_refresh = cache.tint_cell != Some(tint_cell)
        || state.hotbar.is_changed()
        || state.placement_orientation.is_changed()
        || content.blocks.is_changed()
        || content.biomes.is_changed()
        || content.biome_field.is_changed();
    cache.tint_cell = Some(tint_cell);
    let position = tint_cell.as_vec2() + Vec2::splat(0.5);

    for (model, mut icon, material_handle) in &mut icons {
        if !global_refresh && !icon.is_added() {
            continue;
        }

        let Some(block_id) = model.block_id() else {
            continue;
        };
        let block = content
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));
        let orientation = state.placement_orientation.for_block(icon.index, block);
        let tint = block_tint_at(
            block.tint,
            position,
            &content.biome_field,
            &content.biomes,
        );
        let Some(mut material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        if icon.orientation != orientation {
            material.set_block_orientation(block, orientation, &content.asset_server);
            icon.orientation = orientation;
        }
        material.set_tint(tint);
    }
}
