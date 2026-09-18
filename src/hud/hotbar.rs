use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::{
        block::BlockRegistry, block_orientation::BlockOrientation,
        secondary_property::SecondaryPropertyRegistry, tool::ToolRegistry,
    },
    hud::{block_icon::BlockIconMaterial, tool_icon::spawn_tool_icon},
    localization::{ActiveLanguage, Language},
    player::{
        camera::GameplayCamera,
        hotbar::{HOTBAR_SLOT_COUNT, PlayerHotbar},
        inventory::InventoryState,
    },
    rendering::{block_model::BlockModel, block_visual_content::BlockVisualContent},
    targeting::{PlacementOrientation, block::BlockTargetingSet},
    tools::BrushMode,
    ui::{surface, typography, visibility::set_visibility},
};

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 34.0;

#[derive(Component)]
struct HotbarHudRoot;


#[derive(Component)]
struct HotbarSelectedName;

#[derive(Component)]
struct HotbarSlot {
    index: usize,
    item: Option<&'static str>,
}

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
    dyes: Res<'w, SecondaryPropertyRegistry>,
    brush_mode: Res<'w, BrushMode>,
    hotbar: Res<'w, PlayerHotbar>,
    language: Res<'w, ActiveLanguage>,
}

struct HotbarItemView<'a> {
    asset_server: &'a AssetServer,
    blocks: &'a BlockRegistry,
    tools: &'a ToolRegistry,
    dyes: &'a SecondaryPropertyRegistry,
    brush_mode: &'a BrushMode,
    language: Language,
    icon_materials: &'a mut Assets<BlockIconMaterial>,
}

#[derive(SystemParam)]
struct HotbarVisualState<'w, 's> {
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
    placement_orientation: Res<'w, PlacementOrientation>,
    hotbar: Res<'w, PlayerHotbar>,
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
        app.add_systems(OnEnter(GameState::Gameplay), spawn_hotbar)
            .add_systems(
                OnEnter(PauseState::Paused),
                set_visibility::<HotbarHudRoot, false>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                set_visibility::<HotbarHudRoot, true>.run_if(in_state(GameState::Gameplay)),
            )
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

fn spawn_hotbar(
    mut commands: Commands,
    content: HotbarHudContent,
    inventory_state: Res<State<InventoryState>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let visibility = if *inventory_state.get() == InventoryState::Open {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    let language = content.language.get();
    let selected_name = content
        .hotbar
        .item_at(content.hotbar.selected_slot())
        .map(|item_id| item_name(item_id, &content.blocks, &content.tools, language))
        .unwrap_or("");
    let mut items = HotbarItemView {
        asset_server: &content.asset_server,
        blocks: &content.blocks,
        tools: &content.tools,
        dyes: &content.dyes,
        brush_mode: &content.brush_mode,
        language,
        icon_materials: &mut icon_materials,
    };

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
                HotbarSelectedName,
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
                    let selected = index == content.hotbar.selected_slot();
                    let (background, border) = surface::hud_control_static(selected);
                    let item = content.hotbar.item_at(index);

                    row.spawn((
                        HotbarSlot { index, item },
                        Node {
                            width: px(SLOT_SIZE),
                            height: px(SLOT_SIZE),
                            border: UiRect::all(px(2)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(background),
                        BorderColor::all(border),
                        Pickable::IGNORE,
                    ))
                    .with_children(|slot| {
                        if let Some(item_id) = item {
                            spawn_hotbar_item(slot, index, item_id, &mut items);
                        }
                    });
                }
            });
        });
}

fn sync_hotbar(
    mut commands: Commands,
    content: HotbarHudContent,
    mut selected_name: Single<&mut Text, With<HotbarSelectedName>>,
    mut slots: Query<(
        Entity,
        &mut HotbarSlot,
        &mut BackgroundColor,
        &mut BorderColor,
        Option<&Children>,
    )>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    if !content.hotbar.is_changed() && !content.language.is_changed() {
        return;
    }

    let language = content.language.get();
    let next_name = content
        .hotbar
        .item_at(content.hotbar.selected_slot())
        .map(|item_id| item_name(item_id, &content.blocks, &content.tools, language))
        .unwrap_or("");
    if selected_name.0 != next_name {
        selected_name.0 = next_name.to_owned();
    }

    let language_changed = content.language.is_changed();
    let mut items = HotbarItemView {
        asset_server: &content.asset_server,
        blocks: &content.blocks,
        tools: &content.tools,
        dyes: &content.dyes,
        brush_mode: &content.brush_mode,
        language,
        icon_materials: &mut icon_materials,
    };
    for (entity, mut slot, background, border, children) in &mut slots {
        let selected = slot.index == content.hotbar.selected_slot();
        surface::apply_control_colors(surface::hud_control_static(selected), background, border);

        let next_item = content.hotbar.item_at(slot.index);
        if slot.item == next_item && !language_changed {
            continue;
        }

        if let Some(children) = children {
            for &child in children {
                commands.entity(child).despawn();
            }
        }
        slot.item = next_item;

        let Some(item_id) = next_item else {
            continue;
        };
        commands.entity(entity).with_children(|slot_node| {
            spawn_hotbar_item(slot_node, slot.index, item_id, &mut items);
        });
    }
}

fn spawn_hotbar_item(
    slot: &mut ChildSpawnerCommands,
    index: usize,
    item_id: &'static str,
    items: &mut HotbarItemView<'_>,
) {
    if let Some(block) = items.blocks.get(item_id) {
        let orientation = block.default_orientation();
        let material = items.icon_materials.add(BlockIconMaterial::from_block(
            block,
            items.asset_server,
            Color::WHITE,
        ));

        slot.spawn((
            HotbarBlockModel { index, orientation },
            BlockModel::display(item_id),
            MaterialNode(material),
            Node {
                width: px(ITEM_ICON_SIZE),
                height: px(ITEM_ICON_SIZE),
                ..default()
            },
            Pickable::IGNORE,
        ));
        return;
    }

    if let Some(tool) = items.tools.get(item_id) {
        spawn_tool_icon(
            slot,
            tool,
            items.asset_server,
            items.brush_mode,
            items.dyes,
            items.language,
            ITEM_ICON_SIZE,
        );
        return;
    }

    panic!("hotbar references missing item: {item_id}");
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
    content: BlockVisualContent,
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
    let block_definitions_changed = content.block_definitions_changed();
    let global_refresh = cache.tint_cell != Some(tint_cell)
        || state.hotbar.is_changed()
        || state.placement_orientation.is_changed()
        || content.inputs_changed();
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
        let tint = content.tint_at(block_id, position).unwrap_or(Color::WHITE);
        let orientation_changed = icon.orientation != orientation;
        let textures_changed = orientation_changed || block_definitions_changed;
        let tint_changed = materials
            .get(&material_handle.0)
            .is_some_and(|material| !material.has_tint(tint));

        // Acquiring mutable asset access emits a modification event, even if the
        // value written is identical. Compare the rendered inputs first.
        if !textures_changed && !tint_changed {
            continue;
        }
        let Some(mut material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        if textures_changed {
            material.set_block_orientation(block, orientation, &content.asset_server);
        }
        if orientation_changed {
            icon.orientation = orientation;
        }
        if tint_changed {
            material.set_tint(tint);
        }
    }
}
