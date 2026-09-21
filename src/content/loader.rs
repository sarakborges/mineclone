use std::{ffi::OsStr, path::Path};

use bevy::prelude::*;

use crate::app::runtime_paths::data_root;

use super::{
    attack::{AttackDefinition, AttackRegistry},
    biome::{BiomeDefinition, BiomeRegistry},
    block::{BlockDefinition, BlockRegistry},
    creature::{CreatureDefinition, CreatureRegistry},
    player::PlayerDefinition,
    day_night_cycle::{DayNightCycleDefinition, DayNightCycleRegistry},
    dimension::{DimensionDefinition, DimensionRegistry},
    fluid::{FluidDefinition, FluidRegistry},
    inventory_category::{InventoryCategoryDefinition, InventoryCategoryRegistry},
    json_file::{collect_json_files, read_json_definition},
    layer::{LayerDefinition, LayerRegistry},
    secondary_property::{SecondaryPropertyDefinition, SecondaryPropertyRegistry},
    sky::{SkyDefinition, SkyRegistry},
    structure::{StructureDefinition, StructureRegistry},
    tool::{ToolDefinition, ToolRegistry},
    validation::validate_content,
};

#[derive(Default)]
pub(crate) struct LoadedContent {
    pub attacks: AttackRegistry,
    pub biomes: BiomeRegistry,
    pub blocks: BlockRegistry,
    pub creatures: CreatureRegistry,
    pub player: PlayerDefinition,
    pub dimensions: DimensionRegistry,
    pub day_night_cycles: DayNightCycleRegistry,
    pub fluids: FluidRegistry,
    pub inventory_categories: InventoryCategoryRegistry,
    pub layers: LayerRegistry,
    pub secondary_properties: SecondaryPropertyRegistry,
    pub skies: SkyRegistry,
    pub structures: StructureRegistry,
    pub tools: ToolRegistry,
}

impl LoadedContent {
    pub fn insert(self, commands: &mut Commands) {
        commands.insert_resource(self.attacks);
        commands.insert_resource(self.biomes);
        commands.insert_resource(self.blocks);
        commands.insert_resource(self.creatures);
        commands.insert_resource(self.player);
        commands.insert_resource(self.dimensions);
        commands.insert_resource(self.day_night_cycles);
        commands.insert_resource(self.fluids);
        commands.insert_resource(self.inventory_categories);
        commands.insert_resource(self.layers);
        commands.insert_resource(self.secondary_properties);
        commands.insert_resource(self.skies);
        commands.insert_resource(self.structures);
        commands.insert_resource(self.tools);
    }
}

pub fn load_content(mut commands: Commands) {
    read_content().insert(&mut commands);
}

pub(crate) fn read_content() -> LoadedContent {
    let mut content = LoadedContent { player: PlayerDefinition { health: 20.0, attack: "asteria:punch".to_owned(), model: None }, ..Default::default() };
    let mut files = Vec::new();
    let mut player_loaded = false;

    collect_json_files(&data_root(), &mut files);
    files.sort();

    for path in files {
        load_definition(&path, &mut content, &mut player_loaded);
    }

    assert!(player_loaded, "missing player definition under data/entities/player.json");
    validate_content(&content);
    content
}

fn load_definition(path: &Path, content: &mut LoadedContent, player_loaded: &mut bool) {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if file_name == "player.json" && path_has_component(path, "entities") {
        assert!(!*player_loaded, "duplicate player definition: {}", path.display());
        content.player = read_json_definition::<PlayerDefinition>(path);
        content.player.validate();
        *player_loaded = true;
    } else if path_has_component(path, "attacks") {
        content.attacks.insert(read_json_definition::<AttackDefinition>(path));
    } else if file_name == "dimension.json" {
        content
            .dimensions
            .insert(read_json_definition::<DimensionDefinition>(path));
    } else if file_name == "day_night_cycle.json" {
        content
            .day_night_cycles
            .insert(read_json_definition::<DayNightCycleDefinition>(path));
    } else if file_name == "sky.json" {
        content.skies.insert(read_json_definition::<SkyDefinition>(path));
    } else if path_has_component(path, "inventory_categories") {
        content
            .inventory_categories
            .insert(read_json_definition::<InventoryCategoryDefinition>(path));
    } else if path_has_component(path, "layers") {
        content.layers.insert(read_json_definition::<LayerDefinition>(path));
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
    } else if path_has_component(path, "tools") {
        content
            .tools
            .insert(read_json_definition::<ToolDefinition>(path));
    } else if path_has_component(path, "fluids") {
        content
            .fluids
            .insert(read_json_definition::<FluidDefinition>(path));
    } else if path_has_component(path, "structures") {
        assert!(
            path.starts_with(data_root().join("structures")),
            "structure definitions must be under data/structures/, not {}",
            path.display()
        );
        content
            .structures
            .insert(read_json_definition::<StructureDefinition>(path));
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
