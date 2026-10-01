use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    content::{
        biome::BiomeRegistry,
        block::BlockRegistry,
        builtin_ids::{BUCKET_FLUID_METADATA_KEY, WATER_FLUID_ID},
        item::{ItemRegistry, display_name},
        layer::LayerRegistry,
        object::ObjectRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
    },
    hud::{
        block_icon::BlockIconMaterial,
        item_icon::{HudBlockIcon, HudItemIconView, spawn_hud_item_icon},
        item_stack_count::spawn_item_stack_count,
    },
    localization::ActiveLanguage,
    player::{
        camera::GameplayCamera,
        hotbar::{HOTBAR_SLOT_COUNT, PlayerHotbar},
        item_stack::ItemStack,
    },
    rendering::{block_model::BlockModel, block_visual_content::BlockVisualContent},
    targeting::{PlacementOrientation, block::BlockTargetingSet},
    tools::BrushMode,
    ui::{selectable, typography},
    world::biome_field::BiomeField,
};

const SLOT_SIZE: f32 = 44.0;
const ITEM_ICON_SIZE: f32 = 34.0;
const BUCKET_TOOL_ID: &str = "asteria:bucket";
const LAVA_FLUID_ID: &str = "asteria:lava";
const BUCKET_EMPTY_ICON: &str = "textures/tools/iron_bucket_empty.png";
const BUCKET_WATER_ICON: &str = "textures/tools/iron_bucket_water.png";
const BUCKET_LAVA_ICON: &str = "textures/tools/iron_bucket_lava.png";

fn bucket_icon_for_stack(stack: &ItemStack) -> Option<&'static str> {
    if stack.id() != BUCKET_TOOL_ID {
        return None;
    }
    Some(match stack.metadata().get(BUCKET_FLUID_METADATA_KEY) {
        Some(WATER_FLUID_ID) => BUCKET_WATER_ICON,
        Some(LAVA_FLUID_ID) => BUCKET_LAVA_ICON,
        _ => BUCKET_EMPTY_ICON,
    })
}

fn stack_display_name(stack: &ItemStack, base_name: &str) -> String {
    if stack.id() != BUCKET_TOOL_ID {
        return base_name.to_owned();
    }
    let variant = stack
        .metadata()
        .get(BUCKET_FLUID_METADATA_KEY)
        .and_then(|fluid| fluid.rsplit(':').next())
        .filter(|fluid| !fluid.is_empty())
        .unwrap_or("empty");
    format!("{base_name} ({variant})")
}

fn player_position(player: &Transform) -> Vec2 {
    Vec2::new(player.translation.x, player.translation.z)
}

#[derive(Component)]
struct HotbarHudRoot;

#[derive(Component)]
struct HotbarSelectedName;

#[derive(Component)]
struct HotbarSlot {
    index: usize,
    item: Option<&'static str>,
    quantity: u32,
    bucket_icon: Option<&'static str>,
}

#[derive(Default)]
struct HotbarVisualCache {
    tint_cell: Option<IVec2>,
}

#[derive(SystemParam)]
struct HotbarHudContent<'w> {
    asset_server: Res<'w, AssetServer>,
    items: Res<'w, ItemRegistry>,
    blocks: Res<'w, BlockRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
    dyes: Res<'w, SecondaryPropertyRegistry>,
    brush_mode: Res<'w, BrushMode>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    hotbar: Res<'w, PlayerHotbar>,
    language: Res<'w, ActiveLanguage>,
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
            &'static mut HudBlockIcon,
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
                Update,
                (sync_hotbar_visibility, sync_hotbar)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                update_hotbar_item_visuals
                    .after(BlockTargetingSet::PlacementState)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_hotbar(
    mut commands: Commands,
    content: HotbarHudContent,
    player: Single<&Transform, With<GameplayCamera>>,
    pause_state: Res<State<PauseState>>,
    settings_state: Res<State<SettingsState>>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let visibility = hotbar_visibility(*pause_state.get(), *settings_state.get());
    let language = content.language.get();
    let selected_name = content
        .hotbar
        .stack_at(content.hotbar.selected_slot())
        .map(|stack| {
            let base_name = display_name(
                stack.id(),
                &content.items,
                &content.blocks,
                &content.layers,
                &content.objects,
                &content.tools,
                language,
            );
            stack_display_name(stack, base_name)
        })
        .unwrap_or_default();
    let mut items = HudItemIconView {
        asset_server: &content.asset_server,
        items: &content.items,
        blocks: &content.blocks,
        layers: &content.layers,
        objects: &content.objects,
        tools: &content.tools,
        dyes: &content.dyes,
        brush_mode: &content.brush_mode,
        biomes: &content.biomes,
        biome_field: &content.biome_field,
        player_position: player_position(&player),
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
                    let (background, border) = selectable::static_colors(selected);
                    let stack = content.hotbar.stack_at(index);
                    let item = stack.map(ItemStack::id);
                    let quantity = stack.map_or(0, ItemStack::quantity);
                    let bucket_icon = stack.and_then(bucket_icon_for_stack);

                    row.spawn((
                        HotbarSlot {
                            index,
                            item,
                            quantity,
                            bucket_icon,
                        },
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
                        if let Some(stack) = stack {
                            spawn_hud_item_icon(
                                slot,
                                stack.id(),
                                &mut items,
                                ITEM_ICON_SIZE,
                                bucket_icon,
                                Some(index),
                            );
                            spawn_item_stack_count(slot, quantity);
                        }
                    });
                }
            });
        });
}

fn hotbar_visibility(pause: PauseState, settings: SettingsState) -> Visibility {
    if pause == PauseState::Paused || settings == SettingsState::Open {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

fn sync_hotbar_visibility(
    pause: Res<State<PauseState>>,
    settings: Res<State<SettingsState>>,
    mut root: Single<&mut Visibility, With<HotbarHudRoot>>,
) {
    if !pause.is_changed() && !settings.is_changed() {
        return;
    }
    let next = hotbar_visibility(*pause.get(), *settings.get());
    if **root != next {
        **root = next;
    }
}

fn sync_hotbar(
    mut commands: Commands,
    content: HotbarHudContent,
    player: Single<&Transform, With<GameplayCamera>>,
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
        .stack_at(content.hotbar.selected_slot())
        .map(|stack| {
            let base_name = display_name(
                stack.id(),
                &content.items,
                &content.blocks,
                &content.layers,
                &content.objects,
                &content.tools,
                language,
            );
            stack_display_name(stack, base_name)
        })
        .unwrap_or_default();
    if selected_name.0 != next_name {
        selected_name.0 = next_name;
    }

    let language_changed = content.language.is_changed();
    let mut items = HudItemIconView {
        asset_server: &content.asset_server,
        items: &content.items,
        blocks: &content.blocks,
        layers: &content.layers,
        objects: &content.objects,
        tools: &content.tools,
        dyes: &content.dyes,
        brush_mode: &content.brush_mode,
        biomes: &content.biomes,
        biome_field: &content.biome_field,
        player_position: player_position(&player),
        language,
        icon_materials: &mut icon_materials,
    };
    for (entity, mut slot, background, border, children) in &mut slots {
        let selected = slot.index == content.hotbar.selected_slot();
        selectable::apply_colors(selectable::static_colors(selected), background, border);

        let next_stack = content.hotbar.stack_at(slot.index);
        let next_item = next_stack.map(ItemStack::id);
        let next_quantity = next_stack.map_or(0, ItemStack::quantity);
        let next_bucket_icon = next_stack.and_then(bucket_icon_for_stack);
        if slot.item == next_item
            && slot.quantity == next_quantity
            && slot.bucket_icon == next_bucket_icon
            && !language_changed
        {
            continue;
        }

        if let Some(children) = children {
            for &child in children {
                commands.entity(child).despawn();
            }
        }
        slot.item = next_item;
        slot.quantity = next_quantity;
        slot.bucket_icon = next_bucket_icon;

        let Some(stack) = next_stack else {
            continue;
        };
        commands.entity(entity).with_children(|slot_node| {
            spawn_hud_item_icon(
                slot_node,
                stack.id(),
                &mut items,
                ITEM_ICON_SIZE,
                next_bucket_icon,
                Some(slot.index),
            );
            spawn_item_stack_count(slot_node, next_quantity);
        });
    }
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

        let Some(index) = icon.placement_slot else {
            continue;
        };
        let Some(block_id) = model.block_id() else {
            continue;
        };
        let block = content
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));
        let orientation = state.placement_orientation.for_block(index, block);
        let tint = content.tint_at(block_id, position).unwrap_or(Color::WHITE);
        let orientation_changed = icon.orientation != orientation;
        let textures_changed = orientation_changed || block_definitions_changed;
        let tint_changed = materials
            .get(&material_handle.0)
            .is_some_and(|material| !material.has_tint(tint));

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
