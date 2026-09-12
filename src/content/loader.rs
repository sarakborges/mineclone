use std::{ffi::OsStr, path::Path};

use bevy::prelude::*;

use crate::app::runtime_paths::data_root;

use super::{
    biome::{BiomeDefinition, BiomeRegistry},
    block::{BlockDefinition, BlockRegistry},
    day_night_cycle::{DayNightCycleDefinition, DayNightCycleRegistry},
    dimension::{DimensionDefinition, DimensionRegistry},
    fluid::{FluidDefinition, FluidRegistry},
    inventory_category::{InventoryCategoryDefinition, InventoryCategoryRegistry},
    json_file::{collect_json_files, read_json_definition},
    secondary_property::{SecondaryPropertyDefinition, SecondaryPropertyRegistry},
    sky::{SkyDefinition, SkyRegistry},
    structure::{StructureDefinition, StructureRegistry},
    tool::{ToolDefinition, ToolRegistry},
};

#[derive(Default)]
pub(crate) struct LoadedContent {
    pub biomes: BiomeRegistry,
    pub blocks: BlockRegistry,
    pub dimensions: DimensionRegistry,
    pub day_night_cycles: DayNightCycleRegistry,
    pub fluids: FluidRegistry,
    pub inventory_categories: InventoryCategoryRegistry,
    pub secondary_properties: SecondaryPropertyRegistry,
    pub skies: SkyRegistry,
    pub structures: StructureRegistry,
    pub tools: ToolRegistry,
}

impl LoadedContent {
    pub fn insert(self, commands: &mut Commands) {
        commands.insert_resource(self.biomes);
        commands.insert_resource(self.blocks);
        commands.insert_resource(self.dimensions);
        commands.insert_resource(self.day_night_cycles);
        commands.insert_resource(self.fluids);
        commands.insert_resource(self.inventory_categories);
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
    let mut content = LoadedContent::default();
    let mut files = Vec::new();

    collect_json_files(&data_root(), &mut files);

    for path in files {
        load_definition(&path, &mut content);
    }

    for category in content.inventory_categories.iter() {
        category.validate_references(&content.blocks);
    }

    for block in content.blocks.iter() {
        assert!(
            content.inventory_categories.get(&block.category).is_some(),
            "block {} references missing inventory category {}",
            block.id,
            block.category
        );
        for property in &block.secondary_properties {
            assert!(
                content.secondary_properties.contains_property(property),
                "block {} references missing secondary property {}",
                block.id,
                property
            );
        }
    }

    for tool in content.tools.iter() {
        assert!(
            content.blocks.get(&tool.id).is_none(),
            "content id {} cannot be both a block and a tool",
            tool.id
        );
        assert!(
            content.inventory_categories.get(&tool.category).is_some(),
            "tool {} references missing inventory category {}",
            tool.id,
            tool.category
        );
    }

    for structure in content.structures.iter() {
        structure.validate_references(&content.blocks);
    }

    for biome in content.biomes.iter() {
        biome.validate_material_references(&content.blocks);
        biome.validate_structure_references(&content.structures);
    }

    content
}

fn load_definition(path: &Path, content: &mut LoadedContent) {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if file_name == "dimension.json" {
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
    } else if path_has_component(path, "tools") {
        content
            .tools
            .insert(read_json_definition::<ToolDefinition>(path));
    } else if path_has_component(path, "fluids") {
        content
            .fluids
            .insert(read_json_definition::<FluidDefinition>(path));
    } else if path_has_component(path, "structures") {
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
