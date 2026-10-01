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
    gameplay::{availability::world_interaction_available, modal::GameplayModalState},
    localization::ActiveLanguage,
    player::{
        hotbar::{INVENTORY_SLOT_COUNT, PlayerHotbar},
        inventory::InventoryCursor,
        item_stack::{ItemStack, MAX_STACK_SIZE},
    },
    targeting::block::BlockTargetingSet,
    ui::{
        button::{self, ButtonVariant},
        surface, typography,
    },
    world_objects::TargetedWorldObject,
};

use super::inventory::{CharacterInfoInventoryRoot, CharacterInfoInventorySpawn};

const RUSTIC_WORKBENCH_ID: &str = "asteria:rustic_workbench";
const RECIPE_LIST_WIDTH: f32 = 280.0;
const RECIPE_DETAILS_WIDTH: f32 = 520.0;
const CRAFTING_PANEL_GAP: f32 = 18.0;
const CRAFTING_SECTION_GAP: f32 = 12.0;
const CRAFTING_PANEL_PADDING: f32 = 18.0;
const CRAFTING_PANEL_BORDER: f32 = 2.0;

#[derive(Resource, Default)]
struct CraftingSession {
    station: Option<String>,
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
    item_name: String,
    required: u32,
}

#[derive(Component)]
struct CraftingStatusText;

#[derive(SystemParam)]
struct CraftingContent<'w> {
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
                Update,
                open_rustic_workbench
                    .before(BlockTargetingSet::Interaction)
                    .run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(GameplayModalState::Crafting),
                spawn_crafting_screen.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameplayModalState::Crafting),
                (
                    reset_resource::<CraftingSession>,
                    reset_resource::<InventoryCursor>,
                ),
            )
            .add_systems(
                Update,
                (
                    select_crafting_recipe,
                    rebuild_crafting_screen,
                    handle_craft_clicks,
                    sync_ingredient_labels,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayModalState::Crafting)),
            );
    }
}

fn open_rustic_workbench(
    buttons: Res<ButtonInput<MouseButton>>,
    targeted: Res<TargetedWorldObject>,
    mut session: ResMut<CraftingSession>,
    mut next_modal: ResMut<NextState<GameplayModalState>>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(target) = targeted.0 else {
        return;
    };
    if target.object.object_id != RUSTIC_WORKBENCH_ID {
        return;
    }

    session.station = Some(RUSTIC_WORKBENCH_ID.to_owned());
    session.selected_recipe = None;
    session.rebuild_requested = false;
    next_modal.set(GameplayModalState::Crafting);
}

fn spawn_crafting_screen(
    mut commands: Commands,
    mut inventory: CharacterInfoInventorySpawn,
    content: CraftingContent,
    hotbar: Res<PlayerHotbar>,
    mut session: ResMut<CraftingSession>,
) {
    spawn_crafting_root(
        &mut commands,
        &mut inventory,
        &content,
        &hotbar,
        &mut session,
    );
}

fn rebuild_crafting_screen(
    mut commands: Commands,
    mut inventory: CharacterInfoInventorySpawn,
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
    spawn_crafting_root(
        &mut commands,
        &mut inventory,
        &content,
        &hotbar,
        &mut session,
    );
}

fn spawn_crafting_root(
    commands: &mut Commands,
    inventory: &mut CharacterInfoInventorySpawn<'_, '_>,
    content: &CraftingContent<'_>,
    hotbar: &PlayerHotbar,
    session: &mut CraftingSession,
) {
    let station = session
        .station
        .clone()
        .unwrap_or_else(|| RUSTIC_WORKBENCH_ID.to_owned());
    let mut recipes = content.recipes.for_station(&station).collect::<Vec<_>>();
    recipes.sort_by(|left, right| left.id.cmp(&right.id));

    let selection_valid = session
        .selected_recipe
        .as_deref()
        .is_some_and(|selected| recipes.iter().any(|recipe| recipe.id == selected));
    if !selection_valid {
        session.selected_recipe = recipes.first().map(|recipe| recipe.id.clone());
    }

    let selected_recipe = session
        .selected_recipe
        .as_deref()
        .and_then(|id| content.recipes.get(id));

    commands
        .spawn((
            CraftingRoot,
            CharacterInfoInventoryRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(CRAFTING_PANEL_GAP),
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
            DespawnOnExit(GameplayModalState::Crafting),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            spawn_recipe_list(root, &recipes, session, content);
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: px(CRAFTING_PANEL_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|right| {
                spawn_recipe_details(right, selected_recipe, content, hotbar);
                inventory.spawn(right);
            });
        });
}

fn spawn_recipe_list(
    root: &mut ChildSpawnerCommands,
    recipes: &[&CraftingRecipeDefinition],
    session: &CraftingSession,
    content: &CraftingContent<'_>,
) {
    root.spawn((
        surface::hud_container(Node {
            width: px(RECIPE_LIST_WIDTH),
            min_height: px(420),
            padding: UiRect::all(px(CRAFTING_PANEL_PADDING)),
            border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(CRAFTING_SECTION_GAP),
            ..default()
        }),
        Pickable::IGNORE,
    ))
    .with_children(|panel| {
        panel.spawn((typography::hud_heading("Crafting"), Pickable::IGNORE));

        if recipes.is_empty() {
            panel.spawn((typography::muted("No recipes available."), Pickable::IGNORE));
            return;
        }

        for recipe in recipes {
            let selected = session.selected_recipe.as_deref() == Some(recipe.id.as_str());
            panel.spawn(button::button(
                content.item_name(&recipe.result.item),
                CraftingRecipeSelectionButton {
                    recipe_id: recipe.id.clone(),
                },
                percent(100),
                46.0,
                ButtonVariant::from_active(selected),
            ));
        }
    });
}

fn spawn_recipe_details(
    root: &mut ChildSpawnerCommands,
    recipe: Option<&CraftingRecipeDefinition>,
    content: &CraftingContent<'_>,
    hotbar: &PlayerHotbar,
) {
    root.spawn((
        surface::hud_container(Node {
            width: px(RECIPE_DETAILS_WIDTH),
            min_height: px(220),
            padding: UiRect::all(px(CRAFTING_PANEL_PADDING)),
            border: UiRect::all(px(CRAFTING_PANEL_BORDER)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(CRAFTING_SECTION_GAP),
            ..default()
        }),
        Pickable::IGNORE,
    ))
    .with_children(|panel| {
        let Some(recipe) = recipe else {
            panel.spawn((typography::hud_heading("Recipe"), Pickable::IGNORE));
            panel.spawn((typography::muted("Select a recipe."), Pickable::IGNORE));
            return;
        };

        let result_name = content.item_name(&recipe.result.item);
        panel.spawn((
            typography::hud_heading(result_name.clone()),
            Pickable::IGNORE,
        ));
        panel.spawn((
            typography::muted(format!(
                "Produces {} × {}",
                recipe.result.quantity, result_name
            )),
            Pickable::IGNORE,
        ));
        panel.spawn((
            typography::hud_subheading("Ingredients"),
            Pickable::IGNORE,
        ));

        for ingredient in &recipe.ingredients {
            let item_name = content.item_name(&ingredient.item);
            let available = inventory_quantity(hotbar, &ingredient.item);
            panel.spawn((
                CraftingIngredientLabel {
                    item_id: ingredient.item.clone(),
                    item_name: item_name.clone(),
                    required: ingredient.quantity,
                },
                typography::hud(format!(
                    "{item_name}   {available}/{}",
                    ingredient.quantity
                )),
                Pickable::IGNORE,
            ));
        }

        panel.spawn(button::button(
            "Craft",
            CraftRecipeButton {
                recipe_id: recipe.id.clone(),
            },
            percent(100),
            46.0,
            ButtonVariant::Primary,
        ));
        panel.spawn((
            CraftingStatusText,
            typography::muted(""),
            Pickable::IGNORE,
        ));
    });
}

fn select_crafting_recipe(
    interactions: Query<
        (&Interaction, &CraftingRecipeSelectionButton),
        (Changed<Interaction>, With<Button>),
    >,
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
    interactions: Query<(&Interaction, &CraftRecipeButton), (Changed<Interaction>, With<Button>)>,
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
        if !recipe
            .ingredients
            .iter()
            .all(|ingredient| inventory_quantity(&hotbar, &ingredient.item) >= ingredient.quantity)
        {
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
            "craft station={} recipe={} result={} quantity={}",
            recipe.station, recipe.id, recipe.result.item, recipe.result.quantity
        ));
    }
}

fn sync_ingredient_labels(
    hotbar: Res<PlayerHotbar>,
    mut labels: Query<(&CraftingIngredientLabel, &mut Text)>,
) {
    if !hotbar.is_changed() {
        return;
    }

    for (label, mut text) in &mut labels {
        let available = inventory_quantity(&hotbar, &label.item_id);
        text.0 = format!("{}   {available}/{}", label.item_name, label.required);
    }
}

fn set_status(status: &mut Query<&mut Text, With<CraftingStatusText>>, message: &str) {
    for mut text in status.iter_mut() {
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
