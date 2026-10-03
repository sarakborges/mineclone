use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::{crash_log::log_gameplay_event, game_state::GameState, resource_systems::reset_resource},
    content::{
        block::BlockRegistry,
        block_id::intern_block_id,
        crafting_recipe::{CraftingRecipeDefinition, CraftingRecipeRegistry},
        item::{ItemRegistry, display_name},
        item_id::intern_item_id,
        layer::LayerRegistry,
        layer_id::intern_layer_id,
        object::ObjectRegistry,
        object_id::intern_object_id,
        tool::ToolRegistry,
        tool_id::intern_tool_id,
    },
    gameplay::modal::GameplayModalState,
    localization::ActiveLanguage,
    player::{
        camera::GameplayCamera,
        game_mode::GameMode,
        hotbar::{INVENTORY_SLOT_COUNT, PlayerHotbar},
        item_stack::{ItemStack, MAX_STACK_SIZE},
    },
    ui::{
        button::{self, ButtonVariant},
        surface, theme, typography,
    },
};

use super::inventory::{
    INVENTORY_HEADER_GAP, INVENTORY_PANEL_BORDER_WIDTH, INVENTORY_PANEL_PADDING,
    INVENTORY_SECTION_GAP, INVENTORY_SLOT_GAP,
};

const INVENTORY_CRAFTING_ENVIRONMENT: &str = "inventory";
pub(super) const CRAFTING_PANEL_WIDTH: f32 = 482.0;
const CRAFTING_RECIPE_LIST_WIDTH: f32 = 218.0;
const CRAFTING_RECIPE_ROW_HEIGHT: f32 = 58.0;
const CRAFTING_INGREDIENT_ROW_HEIGHT: f32 = 58.0;
const CRAFTING_ICON_FRAME_SIZE: f32 = 42.0;
const CRAFTING_RESULT_ICON_FRAME_SIZE: f32 = 72.0;
const CRAFTING_ICON_SIZE: f32 = 32.0;
const CRAFTING_RESULT_ICON_SIZE: f32 = 54.0;
pub(super) const CURRENT_STATION_PANEL_WIDTH: f32 = 244.0;
const CURRENT_STATION_ICON_FRAME_SIZE: f32 = 160.0;
const CURRENT_STATION_ICON_SIZE: f32 = 112.0;
const CURRENT_STATION_ICON: &str = "textures/creative_categories/crafting_materials.png";
const CRAFTING_READY_COLOR: Color = Color::srgb(0.34, 0.78, 0.42);
const CRAFTING_MISSING_COLOR: Color = theme::DANGER;

#[derive(Resource, Default)]
struct CraftingSession {
    selected_recipe: Option<String>,
    rebuild_requested: bool,
}

#[derive(Component)]
pub(super) struct SurvivalCraftingHost;

#[derive(Component)]
pub(super) struct SurvivalCurrentStationHost;

#[derive(Component)]
struct CraftingWorkspaceMounted;

#[derive(Component)]
struct CurrentStationMounted;

#[derive(Component)]
struct CraftingRoot;

#[derive(Component)]
struct CraftingRecipeSelectionButton {
    recipe_id: String,
}

#[derive(Component)]
struct CraftRecipeButton {
    recipe_id: String,
}

#[derive(Component)]
struct CraftingIngredientLabel {
    item_id: String,
    required: u32,
}

#[derive(Component)]
struct CraftingStatusText;

type RecipeSelectionInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static CraftingRecipeSelectionButton),
    (Changed<Interaction>, With<Button>),
>;
type CraftInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static CraftRecipeButton),
    (Changed<Interaction>, With<Button>),
>;

#[derive(SystemParam)]
struct CraftingContent<'w> {
    asset_server: Res<'w, AssetServer>,
    recipes: Res<'w, CraftingRecipeRegistry>,
    items: Res<'w, ItemRegistry>,
    blocks: Res<'w, BlockRegistry>,
    layers: Res<'w, LayerRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
    language: Res<'w, ActiveLanguage>,
}

impl CraftingContent<'_> {
    fn item_name(&self, item_id: &str) -> String {
        display_name(
            item_id,
            &self.items,
            &self.blocks,
            &self.layers,
            &self.objects,
            &self.tools,
            self.language.get(),
        )
        .to_owned()
    }

    fn item_icon_path<'a>(&'a self, item_id: &str) -> Option<&'a str> {
        if let Some(item) = self.items.get(item_id) {
            return Some(item.icon.as_str());
        }
        if let Some(object) = self.objects.get(item_id) {
            return Some(object.icon.as_str());
        }
        if let Some(layer) = self.layers.get(item_id) {
            return Some(layer.texture.as_str());
        }
        if let Some(tool) = self.tools.get(item_id) {
            return (!tool.icon.is_empty()).then_some(tool.icon.as_str());
        }
        let block = self.blocks.get(item_id)?;
        block
            .textures
            .top
            .first()
            .or_else(|| block.textures.front.first())
            .or_else(|| block.textures.right.first())
            .map(|layer| layer.texture.as_str())
    }

    fn resolve_inventory_item_id(&self, item_id: &str) -> Option<&'static str> {
        if self.items.get(item_id).is_some() {
            Some(intern_item_id(item_id))
        } else if self.blocks.get(item_id).is_some() {
            Some(intern_block_id(item_id))
        } else if self.layers.get(item_id).is_some() {
            Some(intern_layer_id(item_id))
        } else if self.objects.get(item_id).is_some() {
            Some(intern_object_id(item_id))
        } else if self.tools.get(item_id).is_some() {
            Some(intern_tool_id(item_id))
        } else {
            None
        }
    }
}

pub(super) struct CraftingHudPlugin;

impl Plugin for CraftingHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftingSession>()
            .add_systems(
                OnExit(GameplayModalState::Inventory),
                reset_resource::<CraftingSession>,
            )
            .add_systems(
                Update,
                (
                    mount_crafting_workspace,
                    select_crafting_recipe,
                    rebuild_crafting_screen,
                    handle_craft_clicks,
                    sync_crafting_availability,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayModalState::Inventory))
                    .run_if(survival_mode),
            );
    }
}

fn survival_mode(game_mode: Single<&GameMode, With<GameplayCamera>>) -> bool {
    **game_mode == GameMode::Survival
}

fn mount_crafting_workspace(
    mut commands: Commands,
    content: CraftingContent,
    hotbar: Res<PlayerHotbar>,
    mut session: ResMut<CraftingSession>,
    crafting_hosts: Query<
        Entity,
        (With<SurvivalCraftingHost>, Without<CraftingWorkspaceMounted>),
    >,
    station_hosts: Query<
        Entity,
        (With<SurvivalCurrentStationHost>, Without<CurrentStationMounted>),
    >,
) {
    let mut recipes = content
        .recipes
        .for_environment(INVENTORY_CRAFTING_ENVIRONMENT)
        .collect::<Vec<_>>();
    recipes.sort_by_key(|recipe| recipe.id.clone());

    let selection_invalid = session
        .selected_recipe
        .as_deref()
        .is_some_and(|selected| !recipes.iter().any(|recipe| recipe.id == selected));
    if selection_invalid {
        session.selected_recipe = None;
    }

    let selected_recipe = session.selected_recipe.as_deref().and_then(|selected| {
        recipes
            .iter()
            .copied()
            .find(|recipe| recipe.id == selected)
    });

    for host in &crafting_hosts {
        commands
            .entity(host)
            .insert(CraftingWorkspaceMounted)
            .with_children(|root| {
                spawn_crafting_panel(
                    root,
                    &recipes,
                    selected_recipe,
                    &session,
                    &content,
                    &hotbar,
                );
            });
    }

    for host in &station_hosts {
        commands
            .entity(host)
            .insert(CurrentStationMounted)
            .with_children(|root| {
                spawn_current_station_panel(root, &content);
            });
    }
}

fn rebuild_crafting_screen(
    mut commands: Commands,
    mut session: ResMut<CraftingSession>,
    roots: Query<Entity, With<CraftingRoot>>,
    hosts: Query<Entity, With<SurvivalCraftingHost>>,
) {
    if !session.rebuild_requested {
        return;
    }
    session.rebuild_requested = false;

    for root in &roots {
        commands.entity(root).despawn();
    }
    for host in &hosts {
        commands.entity(host).remove::<CraftingWorkspaceMounted>();
    }
}

fn spawn_crafting_panel(
    root: &mut ChildSpawnerCommands,
    recipes: &[&CraftingRecipeDefinition],
    selected_recipe: Option<&CraftingRecipeDefinition>,
    session: &CraftingSession,
    content: &CraftingContent<'_>,
    hotbar: &PlayerHotbar,
) {
    root.spawn((
        CraftingRoot,
        surface::hud_container(Node {
            width: px(CRAFTING_PANEL_WIDTH),
            padding: UiRect::all(px(INVENTORY_PANEL_PADDING)),
            border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexStart,
            column_gap: px(INVENTORY_SECTION_GAP),
            ..default()
        }),
        Pickable::IGNORE,
    ))
    .with_children(|workspace| {
        workspace
            .spawn((
                Node {
                    width: px(CRAFTING_RECIPE_LIST_WIDTH),
                    min_width: px(CRAFTING_RECIPE_LIST_WIDTH),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(INVENTORY_SECTION_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|available| {
                available.spawn((
                    typography::hud_heading("AVAILABLE RECIPES"),
                    TextLayout::no_wrap(),
                    Pickable::IGNORE,
                ));
                spawn_recipe_list(available, recipes, session, content);
            });

        workspace
            .spawn((
                Node {
                    min_width: px(0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(INVENTORY_SECTION_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|selected| {
                spawn_recipe_details(selected, selected_recipe, content, hotbar);
            });
    });
}

fn spawn_current_station_panel(
    parent: &mut ChildSpawnerCommands,
    content: &CraftingContent<'_>,
) {
    parent
        .spawn((
            surface::hud_container(Node {
                width: px(CURRENT_STATION_PANEL_WIDTH),
                padding: UiRect::all(px(INVENTORY_PANEL_PADDING)),
                border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(INVENTORY_SECTION_GAP),
                ..default()
            }),
            Pickable::IGNORE,
        ))
        .with_children(|station| {
            station.spawn((typography::hud_heading("Current Station"), Pickable::IGNORE));

            station
                .spawn((
                    Node {
                        width: percent(100),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|preview| {
                    preview
                        .spawn((
                            Node {
                                width: px(CURRENT_STATION_ICON_FRAME_SIZE),
                                height: px(CURRENT_STATION_ICON_FRAME_SIZE),
                                min_width: px(CURRENT_STATION_ICON_FRAME_SIZE),
                                min_height: px(CURRENT_STATION_ICON_FRAME_SIZE),
                                border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(theme::SURFACE_ELEVATED),
                            BorderColor::all(theme::BORDER),
                            Pickable::IGNORE,
                        ))
                        .with_children(|frame| {
                            frame.spawn((
                                ImageNode::new(content.asset_server.load(CURRENT_STATION_ICON)),
                                Node {
                                    width: px(CURRENT_STATION_ICON_SIZE),
                                    height: px(CURRENT_STATION_ICON_SIZE),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                        });
                });

            station
                .spawn((
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexStart,
                        row_gap: px(INVENTORY_SLOT_GAP),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|copy| {
                    copy.spawn((typography::caption("BASE STATION"), Pickable::IGNORE));
                    copy.spawn((
                        typography::hud_subheading("Inventory"),
                        Pickable::IGNORE,
                    ));
                    copy.spawn((
                        typography::muted("Personal crafting"),
                        Pickable::IGNORE,
                    ));
                });
        });
}

fn spawn_recipe_list(
    parent: &mut ChildSpawnerCommands,
    recipes: &[&CraftingRecipeDefinition],
    session: &CraftingSession,
    content: &CraftingContent<'_>,
) {
    if recipes.is_empty() {
        parent.spawn((typography::muted("No recipes available."), Pickable::IGNORE));
        return;
    }

    parent
        .spawn((
            Node {
                width: percent(100),
                padding: UiRect::all(px(INVENTORY_HEADER_GAP)),
                border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(INVENTORY_HEADER_GAP),
                ..default()
            },
            BackgroundColor(theme::SURFACE_INSET),
            BorderColor::all(theme::BORDER),
            Pickable::IGNORE,
        ))
        .with_children(|list| {
            for recipe in recipes {
                let selected = session.selected_recipe.as_deref() == Some(recipe.id.as_str());
                spawn_recipe_button(list, recipe, selected, content);
            }
        });
}

fn spawn_recipe_button(
    parent: &mut ChildSpawnerCommands,
    recipe: &CraftingRecipeDefinition,
    selected: bool,
    content: &CraftingContent<'_>,
) {
    let mut recipe_button = parent.spawn(button::button(
        "",
        CraftingRecipeSelectionButton {
            recipe_id: recipe.id.clone(),
        },
        percent(100),
        CRAFTING_RECIPE_ROW_HEIGHT,
        ButtonVariant::from_active(selected),
    ));
    recipe_button.insert(Node {
        width: percent(100),
        height: px(CRAFTING_RECIPE_ROW_HEIGHT),
        padding: UiRect::horizontal(px(INVENTORY_HEADER_GAP)),
        border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::FlexStart,
        column_gap: px(INVENTORY_HEADER_GAP),
        ..default()
    });
    recipe_button.with_children(|button_node| {
        if selected {
            button_node.spawn((
                Node {
                    width: px(3),
                    height: percent(72),
                    ..default()
                },
                BackgroundColor(theme::BORDER_FOCUS),
                Pickable::IGNORE,
            ));
        }
        spawn_item_icon_frame(
            button_node,
            &recipe.result.item,
            CRAFTING_ICON_FRAME_SIZE,
            CRAFTING_ICON_SIZE,
            content,
        );
        button_node
            .spawn((
                Node {
                    min_width: px(0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(INVENTORY_SLOT_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|copy| {
                copy.spawn((
                    typography::inventory_category(content.item_name(&recipe.result.item)),
                    Pickable::IGNORE,
                ));
                copy.spawn((
                    typography::caption(format!("Creates ×{}", recipe.result.quantity)),
                    Pickable::IGNORE,
                ));
            });
    });
}

fn spawn_recipe_details(
    parent: &mut ChildSpawnerCommands,
    recipe: Option<&CraftingRecipeDefinition>,
    content: &CraftingContent<'_>,
    hotbar: &PlayerHotbar,
) {
    parent.spawn((
        typography::hud_heading("SELECTED RECIPE"),
        TextLayout::no_wrap(),
        Pickable::IGNORE,
    ));

    let Some(recipe) = recipe else {
        parent.spawn((typography::muted("Select a recipe."), Pickable::IGNORE));
        return;
    };

    spawn_result_card(parent, recipe, content);

    parent.spawn((
        typography::hud_subheading("Ingredients"),
        Pickable::IGNORE,
    ));

    parent
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(INVENTORY_HEADER_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|ingredients| {
            for ingredient in &recipe.ingredients {
                spawn_ingredient_row(ingredients, ingredient, content, hotbar);
            }
        });

    let craftable = recipe_craftable(hotbar, recipe);
    parent.spawn(button::button(
        "Craft Item",
        CraftRecipeButton {
            recipe_id: recipe.id.clone(),
        },
        percent(100),
        50.0,
        ButtonVariant::from_active(craftable),
    ));
    parent.spawn((
        CraftingStatusText,
        typography::caption(if craftable {
            "All materials available."
        } else {
            "Missing required materials."
        }),
        Pickable::IGNORE,
    ));
}

fn spawn_result_card(
    parent: &mut ChildSpawnerCommands,
    recipe: &CraftingRecipeDefinition,
    content: &CraftingContent<'_>,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(96),
                padding: UiRect::all(px(INVENTORY_HEADER_GAP)),
                border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(INVENTORY_HEADER_GAP),
                ..default()
            },
            BackgroundColor(theme::SURFACE_INSET),
            BorderColor::all(theme::BORDER_STRONG),
            Pickable::IGNORE,
        ))
        .with_children(|card| {
            spawn_item_icon_frame(
                card,
                &recipe.result.item,
                CRAFTING_RESULT_ICON_FRAME_SIZE,
                CRAFTING_RESULT_ICON_SIZE,
                content,
            );
            card.spawn((
                Node {
                    min_width: px(0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(INVENTORY_SLOT_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|copy| {
                copy.spawn((typography::caption("RESULT"), Pickable::IGNORE));
                copy.spawn((
                    typography::hud_subheading(content.item_name(&recipe.result.item)),
                    Pickable::IGNORE,
                ));
                copy.spawn((
                    typography::muted(format!("Output ×{}", recipe.result.quantity)),
                    Pickable::IGNORE,
                ));
            });
        });
}

fn spawn_ingredient_row(
    parent: &mut ChildSpawnerCommands,
    ingredient: &crate::content::crafting_recipe::CraftingRecipeIngredientDefinition,
    content: &CraftingContent<'_>,
    hotbar: &PlayerHotbar,
) {
    let available = inventory_quantity(hotbar, &ingredient.item);
    let enough = available >= ingredient.quantity;
    let semantic_color = if enough {
        CRAFTING_READY_COLOR
    } else {
        CRAFTING_MISSING_COLOR
    };

    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(CRAFTING_INGREDIENT_ROW_HEIGHT),
                padding: UiRect::all(px(INVENTORY_HEADER_GAP)),
                border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(INVENTORY_HEADER_GAP),
                ..default()
            },
            BackgroundColor(theme::SURFACE_INSET),
            BorderColor::all(if enough {
                CRAFTING_READY_COLOR
            } else {
                theme::BORDER
            }),
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            spawn_item_icon_frame(
                row,
                &ingredient.item,
                CRAFTING_ICON_FRAME_SIZE,
                CRAFTING_ICON_SIZE,
                content,
            );
            row.spawn((
                Node {
                    min_width: px(0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(INVENTORY_SLOT_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|copy| {
                copy.spawn((
                    typography::hud(content.item_name(&ingredient.item)),
                    Pickable::IGNORE,
                ));
                copy.spawn((typography::caption("Material"), Pickable::IGNORE));
            });

            let mut availability = row.spawn((
                CraftingIngredientLabel {
                    item_id: ingredient.item.clone(),
                    required: ingredient.quantity,
                },
                typography::inventory_category(format!(
                    "{} {available}/{}",
                    if enough { "✓" } else { "•" },
                    ingredient.quantity
                )),
                Pickable::IGNORE,
            ));
            availability.insert(TextColor(semantic_color));
        });
}

fn spawn_item_icon_frame(
    parent: &mut ChildSpawnerCommands,
    item_id: &str,
    frame_size: f32,
    icon_size: f32,
    content: &CraftingContent<'_>,
) {
    parent
        .spawn((
            Node {
                width: px(frame_size),
                height: px(frame_size),
                min_width: px(frame_size),
                min_height: px(frame_size),
                border: UiRect::all(px(INVENTORY_PANEL_BORDER_WIDTH)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(theme::SURFACE_ELEVATED),
            BorderColor::all(theme::BORDER),
            Pickable::IGNORE,
        ))
        .with_children(|frame| {
            if let Some(icon) = content.item_icon_path(item_id) {
                frame.spawn((
                    ImageNode::new(content.asset_server.load(icon.to_owned())),
                    Node {
                        width: px(icon_size),
                        height: px(icon_size),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            } else {
                frame.spawn((
                    typography::hud("?"),
                    TextLayout::justify(Justify::Center),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn select_crafting_recipe(
    interactions: RecipeSelectionInteractionQuery,
    mut session: ResMut<CraftingSession>,
) {
    for (interaction, selection) in &interactions {
        if *interaction != Interaction::Pressed
            || session.selected_recipe.as_deref() == Some(selection.recipe_id.as_str())
        {
            continue;
        }
        session.selected_recipe = Some(selection.recipe_id.clone());
        session.rebuild_requested = true;
    }
}

fn handle_craft_clicks(
    interactions: CraftInteractionQuery,
    content: CraftingContent,
    mut hotbar: ResMut<PlayerHotbar>,
    mut status: Query<&mut Text, With<CraftingStatusText>>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(recipe) = content.recipes.get(&action.recipe_id) else {
            set_status(&mut status, "Recipe unavailable.");
            continue;
        };
        if recipe.environment != INVENTORY_CRAFTING_ENVIRONMENT {
            set_status(&mut status, "Recipe unavailable in this environment.");
            continue;
        }
        if !recipe_craftable(&hotbar, recipe) {
            set_status(&mut status, "Missing ingredients.");
            continue;
        }

        let Some(result_id) = content.resolve_inventory_item_id(&recipe.result.item) else {
            set_status(&mut status, "Recipe result unavailable.");
            continue;
        };

        let snapshot = (0..INVENTORY_SLOT_COUNT)
            .map(|index| hotbar.inventory_stack_at(index).cloned())
            .collect::<Vec<_>>();
        for ingredient in &recipe.ingredients {
            let consumed =
                consume_inventory_quantity(&mut hotbar, &ingredient.item, ingredient.quantity);
            debug_assert!(consumed, "ingredient preflight must make consumption succeed");
        }

        let mut remaining = recipe.result.quantity;
        let mut result_fits = true;
        while remaining > 0 {
            let quantity = remaining.min(MAX_STACK_SIZE);
            let stack = ItemStack::new(result_id).with_quantity(quantity);
            if hotbar.try_insert_stack(stack).is_err() {
                result_fits = false;
                break;
            }
            remaining -= quantity;
        }

        if !result_fits {
            for (index, stack) in snapshot.into_iter().enumerate() {
                hotbar.replace_inventory_item(index, stack);
            }
            set_status(&mut status, "Not enough inventory space.");
            continue;
        }

        let result_name = content.item_name(&recipe.result.item);
        set_status(&mut status, &format!("Crafted {result_name}."));
        log_gameplay_event(format!(
            "craft environment={} recipe={} result={} quantity={}",
            recipe.environment, recipe.id, recipe.result.item, recipe.result.quantity
        ));
    }
}

fn sync_crafting_availability(
    hotbar: Res<PlayerHotbar>,
    content: CraftingContent,
    mut labels: Query<(&CraftingIngredientLabel, &mut Text, &mut TextColor)>,
    mut buttons: Query<(&CraftRecipeButton, &mut ButtonVariant)>,
) {
    if !hotbar.is_changed() {
        return;
    }

    for (label, mut text, mut color) in &mut labels {
        let available = inventory_quantity(&hotbar, &label.item_id);
        let enough = available >= label.required;
        text.0 = format!(
            "{} {available}/{}",
            if enough { "✓" } else { "•" },
            label.required
        );
        color.0 = if enough {
            CRAFTING_READY_COLOR
        } else {
            CRAFTING_MISSING_COLOR
        };
    }

    for (action, mut variant) in &mut buttons {
        let craftable = content
            .recipes
            .get(&action.recipe_id)
            .is_some_and(|recipe| recipe_craftable(&hotbar, recipe));
        *variant = ButtonVariant::from_active(craftable);
    }
}

fn recipe_craftable(hotbar: &PlayerHotbar, recipe: &CraftingRecipeDefinition) -> bool {
    recipe
        .ingredients
        .iter()
        .all(|ingredient| inventory_quantity(hotbar, &ingredient.item) >= ingredient.quantity)
}

fn set_status(status: &mut Query<&mut Text, With<CraftingStatusText>>, message: &str) {
    for mut text in status {
        text.0 = message.to_owned();
    }
}

fn inventory_quantity(hotbar: &PlayerHotbar, item_id: &str) -> u32 {
    (0..INVENTORY_SLOT_COUNT)
        .filter_map(|index| hotbar.inventory_stack_at(index))
        .filter(|stack| stack.id() == item_id)
        .map(ItemStack::quantity)
        .sum()
}

fn consume_inventory_quantity(hotbar: &mut PlayerHotbar, item_id: &str, quantity: u32) -> bool {
    if inventory_quantity(hotbar, item_id) < quantity {
        return false;
    }

    let mut remaining = quantity;
    for index in 0..INVENTORY_SLOT_COUNT {
        if remaining == 0 {
            break;
        }
        let Some(stack) = hotbar.inventory_stack_at(index).cloned() else {
            continue;
        };
        if stack.id() != item_id {
            continue;
        }

        let consumed = remaining.min(stack.quantity());
        let left = stack.quantity() - consumed;
        let replacement = if left == 0 {
            None
        } else {
            Some(stack.with_quantity(left))
        };
        hotbar.replace_inventory_item(index, replacement);
        remaining -= consumed;
    }

    remaining == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingredient_consumption_spans_inventory_slots() {
        let mut hotbar = PlayerHotbar::default();
        hotbar.replace_inventory_item(
            0,
            Some(ItemStack::new("asteria:stick").with_quantity(2)),
        );
        hotbar.replace_inventory_item(
            1,
            Some(ItemStack::new("asteria:stick").with_quantity(2)),
        );

        assert!(consume_inventory_quantity(
            &mut hotbar,
            "asteria:stick",
            3
        ));
        assert_eq!(inventory_quantity(&hotbar, "asteria:stick"), 1);
    }
}
