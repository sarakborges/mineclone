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

const INVENTORY_CRAFTING_ENVIRONMENT: &str = "inventory";
const CRAFTING_PANEL_WIDTH: f32 = 420.0;
const CRAFTING_PANEL_GAP: f32 = 14.0;
const CRAFTING_INSET_GAP: f32 = 8.0;
const CRAFTING_PANEL_PADDING: f32 = 18.0;
const CRAFTING_PANEL_BORDER: f32 = 2.0;
const CRAFTING_SCREEN_PADDING: f32 = 24.0;
const CRAFTING_RECIPE_ROW_HEIGHT: f32 = 58.0;
const CRAFTING_INGREDIENT_ROW_HEIGHT: f32 = 58.0;
const CRAFTING_ICON_FRAME_SIZE: f32 = 42.0;
const CRAFTING_RESULT_ICON_FRAME_SIZE: f32 = 72.0;
const CRAFTING_ICON_SIZE: f32 = 32.0;
const CRAFTING_RESULT_ICON_SIZE: f32 = 54.0;
const CURRENT_STATION_PANEL_WIDTH: f32 = 244.0;
const CURRENT_STATION_ICON_FRAME_SIZE: f32 = 84.0;
const CURRENT_STATION_ICON_SIZE: f32 = 58.0;
const CURRENT_STATION_ICON: &str = "textures/creative_categories/crafting_materials.png";
const CRAFTING_READY_COLOR: Color = Color::srgb(0.34, 0.78, 0.42);
const CRAFTING_MISSING_COLOR: Color = theme::DANGER;

#[derive(Resource, Default)]
struct CraftingSession {
    selected_recipe: Option<String>,
    rebuild_requested: bool,
}

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
                OnEnter(GameplayModalState::Inventory),
                spawn_crafting_screen
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(survival_mode),
            )
            .add_systems(
                OnExit(GameplayModalState::Inventory),
                reset_resource::<CraftingSession>,
            )
            .add_systems(
                Update,
                (
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

fn spawn_crafting_screen(
    mut commands: Commands,
    content: CraftingContent,
    hotbar: Res<PlayerHotbar>,
    mut session: ResMut<CraftingSession>,
) {
    spawn_crafting_root(&mut commands, &content, &hotbar, &mut session);
}

fn rebuild_crafting_screen(
    mut commands: Commands,
    content: CraftingContent,
    hotbar: Res<PlayerHotbar>,
    mut session: ResMut<CraftingSession>,
    roots: Query<Entity, With<CraftingRoot>>,
) {
    if !session.rebuild_requested {
        return;
    }
    session.rebuild_requested = false;

    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_crafting_root(&mut commands, &content, &hotbar, &mut session);
}

fn spawn_crafting_root(
    commands: &mut Commands,
    content: &CraftingContent<'_>,
    hotbar: &PlayerHotbar,
    session: &mut CraftingSession,
) {
    let mut recipes = content
        .recipes
        .for_environment(INVENTORY_CRAFTING_ENVIRONMENT)
        .collect::<Vec<_>>();
    recipes.sort_by_key(|recipe| recipe.id.clone());

    let selection_valid = session
        .selected_recipe
        .as_deref()
        .is_some_and(|selected| recipes.iter().any(|recipe| recipe.id == selected));
    if !selection_valid {
        session.selected_recipe = recipes.first().map(|recipe| recipe.id.clone());
    }

    let selected_recipe = session.selected_recipe.as_deref().and_then(|selected| {
        recipes
            .iter()
            .copied()
            .find(|recipe| recipe.id == selected)
    });

    commands
        .spawn((
            CraftingRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(CRAFTING_SCREEN_PADDING)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
            DespawnOnExit(GameplayModalState::Inventory),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            spawn_crafting_panel(root, &recipes, selected_recipe, session, content, hotbar);
            spawn_current_station_dock(root, content);
        });
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
        surface::hud_container(Node {
            width: px(CRAFTING_PANEL_WIDTH),
            padding: UiRect::all(px(CRAFTING_PANEL_PADDING)),
            border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(CRAFTING_PANEL_GAP),
            ..default()
        }),
        Pickable::IGNORE,
    ))
    .with_children(|panel| {
        panel
            .spawn((
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|header| {
                header.spawn((typography::hud_heading("Crafting"), Pickable::IGNORE));
                header.spawn((
                    typography::caption(format!("{} recipe(s)", recipes.len())),
                    Pickable::IGNORE,
                ));
            });

        panel.spawn((
            typography::caption("AVAILABLE RECIPES"),
            Pickable::IGNORE,
        ));
        spawn_recipe_list(panel, recipes, session, content);

        panel.spawn((
            Node {
                width: percent(100),
                height: px(CRAFTING_PANEL_BORDER),
                margin: UiRect::vertical(px(2)),
                ..default()
            },
            BackgroundColor(theme::BORDER),
            Pickable::IGNORE,
        ));

        spawn_recipe_details(panel, selected_recipe, content, hotbar);
    });
}

fn spawn_current_station_dock(
    root: &mut ChildSpawnerCommands,
    content: &CraftingContent<'_>,
) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(CRAFTING_SCREEN_PADDING),
            top: px(0),
            bottom: px(0),
            width: px(CURRENT_STATION_PANEL_WIDTH),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|dock| {
        spawn_current_station_panel(dock, content);
    });
}

fn spawn_current_station_panel(
    parent: &mut ChildSpawnerCommands,
    content: &CraftingContent<'_>,
) {
    parent
        .spawn((
            surface::hud_container(Node {
                width: percent(100),
                padding: UiRect::all(px(CRAFTING_PANEL_PADDING)),
                border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(CRAFTING_PANEL_GAP),
                ..default()
            }),
            Pickable::IGNORE,
        ))
        .with_children(|panel| {
            panel.spawn((typography::hud_heading("Current Station"), Pickable::IGNORE));

            panel
                .spawn((
                    Node {
                        width: percent(100),
                        padding: UiRect::all(px(14)),
                        border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(CRAFTING_INSET_GAP),
                        ..default()
                    },
                    BackgroundColor(theme::SURFACE_INSET),
                    BorderColor::all(theme::BORDER_STRONG),
                    Pickable::IGNORE,
                ))
                .with_children(|card| {
                    card.spawn((
                        Node {
                            width: px(CURRENT_STATION_ICON_FRAME_SIZE),
                            height: px(CURRENT_STATION_ICON_FRAME_SIZE),
                            border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
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

                    card.spawn((typography::caption("BASE STATION"), Pickable::IGNORE));
                    card.spawn((
                        typography::hud_subheading("Inventory"),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                    card.spawn((
                        typography::muted("Personal crafting"),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                });

            panel
                .spawn((
                    Node {
                        width: percent(100),
                        padding: UiRect::axes(px(10), px(8)),
                        border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    BackgroundColor(theme::SURFACE_INSET),
                    BorderColor::all(theme::BORDER),
                    Pickable::IGNORE,
                ))
                .with_children(|status| {
                    status.spawn((typography::caption("STATUS"), Pickable::IGNORE));
                    status
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: px(6),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|ready| {
                            ready.spawn((
                                Node {
                                    width: px(8),
                                    height: px(8),
                                    ..default()
                                },
                                BackgroundColor(CRAFTING_READY_COLOR),
                                Pickable::IGNORE,
                            ));
                            let mut available = ready.spawn((
                                typography::inventory_category("Available"),
                                Pickable::IGNORE,
                            ));
                            available.insert(TextColor(CRAFTING_READY_COLOR));
                        });
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
                padding: UiRect::all(px(CRAFTING_INSET_GAP)),
                border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(CRAFTING_INSET_GAP),
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
        padding: UiRect::horizontal(px(10)),
        border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::FlexStart,
        column_gap: px(10),
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
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(2),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|copy| {
                copy.spawn((
                    typography::hud(content.item_name(&recipe.result.item)),
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
    let Some(recipe) = recipe else {
        parent.spawn((typography::hud_subheading("Recipe"), Pickable::IGNORE));
        parent.spawn((typography::muted("Select a recipe."), Pickable::IGNORE));
        return;
    };

    parent.spawn((typography::caption("SELECTED RECIPE"), Pickable::IGNORE));
    spawn_result_card(parent, recipe, content);

    parent
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|header| {
            header.spawn((typography::hud_subheading("Ingredients"), Pickable::IGNORE));
            header.spawn((
                typography::caption(format!("{} required", recipe.ingredients.len())),
                Pickable::IGNORE,
            ));
        });

    parent
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(CRAFTING_INSET_GAP),
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
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(14),
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
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(4),
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
                padding: UiRect::axes(px(10), px(8)),
                border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
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
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(2),
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
                border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
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
