use std::{ffi::OsStr, path::Path, time::Instant};

use bevy::prelude::*;

use crate::app::{crash_log::log_system_event, runtime_paths::data_root};

use super::{
    attack::{AttackDefinition, AttackRegistry},
    biome::{BiomeDefinition, BiomeRegistry},
    block::{BlockDefinition, BlockRegistry},
    crafting_recipe::{CraftingRecipeDefinition, CraftingRecipeRegistry},
    creature::{CreatureDefinition, CreatureRegistry},
    day_night_cycle::{DayNightCycleDefinition, DayNightCycleRegistry},
    dimension::{DimensionDefinition, DimensionRegistry},
    fluid::{FluidDefinition, FluidRegistry},
    inventory_category::{InventoryCategoryDefinition, InventoryCategoryRegistry},
    item::{ItemDefinition, ItemRegistry},
    json_file::{collect_json_files, read_json_definition},
    layer::{LayerDefinition, LayerRegistry},
    object::{ObjectDefinition, ObjectRegistry},
    player::PlayerDefinition,
    secondary_property::{SecondaryPropertyDefinition, SecondaryPropertyRegistry},
    sky::{SkyDefinition, SkyRegistry},
    structure::{StructureDefinition, StructureRegistry},
    structure_set::{StructureSetDefinition, StructureSetRegistry},
    tool::{ToolDefinition, ToolRegistry},
    tool_category::{ToolCategoryDefinition, ToolCategoryRegistry},
    validation::validate_content,
    world_recipe::{WorldRecipeDefinition, WorldRecipeRegistry},
};

#[derive(Default)]
pub(crate) struct LoadedContent {
    pub attacks: AttackRegistry,
    pub biomes: BiomeRegistry,
    pub blocks: BlockRegistry,
    pub crafting_recipes: CraftingRecipeRegistry,
    pub creatures: CreatureRegistry,
    pub player: PlayerDefinition,
    pub dimensions: DimensionRegistry,
    pub day_night_cycles: DayNightCycleRegistry,
    pub fluids: FluidRegistry,
    pub inventory_categories: InventoryCategoryRegistry,
    pub items: ItemRegistry,
    pub layers: LayerRegistry,
    pub objects: ObjectRegistry,
    pub secondary_properties: SecondaryPropertyRegistry,
    pub skies: SkyRegistry,
    pub structures: StructureRegistry,
    pub structure_sets: StructureSetRegistry,
    pub tools: ToolRegistry,
    pub tool_categories: ToolCategoryRegistry,
    pub world_recipes: WorldRecipeRegistry,
}

impl LoadedContent {
    pub fn insert(self, commands: &mut Commands) {
        commands.insert_resource(self.attacks);
        commands.insert_resource(self.biomes);
        commands.insert_resource(self.blocks);
        commands.insert_resource(self.crafting_recipes);
        commands.insert_resource(self.creatures);
        commands.insert_resource(self.player);
        commands.insert_resource(self.dimensions);
        commands.insert_resource(self.day_night_cycles);
        commands.insert_resource(self.fluids);
        commands.insert_resource(self.inventory_categories);
        commands.insert_resource(self.items);
        commands.insert_resource(self.layers);
        commands.insert_resource(self.objects);
        commands.insert_resource(self.secondary_properties);
        commands.insert_resource(self.skies);
        commands.insert_resource(self.structures);
        commands.insert_resource(self.structure_sets);
        commands.insert_resource(self.tools);
        commands.insert_resource(self.tool_categories);
        commands.insert_resource(self.world_recipes);
    }
}

pub fn load_content(mut commands: Commands) {
    read_content().insert(&mut commands);
}

pub(crate) fn read_content() -> LoadedContent {
    let started = Instant::now();
    let root = data_root();
    let mut content = LoadedContent::default();
    let mut files = Vec::new();
    let mut player_loaded = false;

    collect_json_files(&root, &mut files);
    files.sort();
    let file_count = files.len();

    for path in files {
        load_definition(&path, &mut content, &mut player_loaded);
    }

    assert!(
        player_loaded,
        "missing player definition under data/entities/player.json"
    );
    validate_content(&content);
    log_system_event(format!(
        "content.load success root={} files={} duration_ms={:.2}",
        root.display(),
        file_count,
        started.elapsed().as_secs_f64() * 1_000.0,
    ));
    content
}

fn load_definition(path: &Path, content: &mut LoadedContent, player_loaded: &mut bool) {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if file_name == "player.json" && path_has_component(path, "entities") {
        assert!(
            !*player_loaded,
            "duplicate player definition: {}",
            path.display()
        );
        content.player = read_json_definition::<PlayerDefinition>(path);
        content.player.validate();
        *player_loaded = true;
    } else if path_has_component(path, "attacks") {
        content
            .attacks
            .insert(read_json_definition::<AttackDefinition>(path));
    } else if file_name == "dimension.json" {
        content
            .dimensions
            .insert(read_json_definition::<DimensionDefinition>(path));
    } else if file_name == "day_night_cycle.json" {
        content
            .day_night_cycles
            .insert(read_json_definition::<DayNightCycleDefinition>(path));
    } else if file_name == "sky.json" {
        content
            .skies
            .insert(read_json_definition::<SkyDefinition>(path));
    } else if path_has_component(path, "inventory_categories") {
        content
            .inventory_categories
            .insert(read_json_definition::<InventoryCategoryDefinition>(path));
    } else if path_has_component(path, "crafting_recipes") {
        content
            .crafting_recipes
            .insert(read_json_definition::<CraftingRecipeDefinition>(path));
    } else if path_has_component(path, "items") {
        content
            .items
            .insert(read_json_definition::<ItemDefinition>(path));
    } else if path_has_component(path, "layers") {
        content
            .layers
            .insert(read_json_definition::<LayerDefinition>(path));
    } else if path_has_component(path, "objects") {
        content
            .objects
            .insert(read_json_definition::<ObjectDefinition>(path));
    } else if path_has_component(path, "world_recipes") {
        content
            .world_recipes
            .insert(read_json_definition::<WorldRecipeDefinition>(path));
    } else if path_has_component(path, "secondary_properties") {
        let property = secondary_property_group(path).unwrap_or_else(|| {
            panic!(
                "secondary property definition must be inside data/secondary_properties/<property>: {}",
                path.display()
            )
        });
        content.secondary_properties.insert(
            property,
            read_json_definition::<SecondaryPropertyDefinition>(path),
        );
    } else if path_has_component(path, "biomes") {
        content
            .biomes
            .insert(read_json_definition::<BiomeDefinition>(path));
    } else if path_has_component(path, "blocks") {
        content
            .blocks
            .insert(read_json_definition::<BlockDefinition>(path));
    } else if path_has_component(path, "creatures") {
        content
            .creatures
            .insert(read_json_definition::<CreatureDefinition>(path));
    } else if path_has_component(path, "tool_categories") {
        content
            .tool_categories
            .insert(read_json_definition::<ToolCategoryDefinition>(path));
    } else if path_has_component(path, "tools") {
        content
            .tools
            .insert(read_json_definition::<ToolDefinition>(path));
    } else if path_has_component(path, "fluids") {
        content
            .fluids
            .insert(read_json_definition::<FluidDefinition>(path));
    } else if path_has_component(path, "structure_sets") {
        assert!(
            path.starts_with(data_root().join("structure_sets")),
            "structure set definitions must be under data/structure_sets/, not {}",
            path.display()
        );
        content
            .structure_sets
            .insert(read_json_definition::<StructureSetDefinition>(path));
    } else if path_has_component(path, "structures") {
        assert!(
            path.starts_with(data_root().join("structures")),
            "structure definitions must be under data/structures/, not {}",
            path.display()
        );
        content
            .structures
            .insert(read_json_definition::<StructureDefinition>(path));
    } else if path.starts_with(data_root().join("localization")) {
        // Localization owns its own JSON loader. Keep this explicit so a
        // misspelled or misplaced content definition never disappears silently.
    } else {
        panic!("unrecognized data JSON definition: {}", path.display());
    }
}

fn secondary_property_group(path: &Path) -> Option<String> {
    let mut components = path.components();

    while let Some(component) = components.next() {
        if component.as_os_str() != OsStr::new("secondary_properties") {
            continue;
        }

        return components
            .next()
            .and_then(|component| component.as_os_str().to_str())
            .map(str::to_owned);
    }

    None
}

fn path_has_component(path: &Path, component: &str) -> bool {
    let component = OsStr::new(component);
    path.components()
        .any(|candidate| candidate.as_os_str() == component)
}
