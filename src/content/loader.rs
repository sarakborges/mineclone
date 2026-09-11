use std::{ffi::OsStr, path::Path};

use bevy::prelude::*;

use crate::app::runtime_paths::data_root;

use super::{
    biome::{BiomeDefinition, BiomeRegistry},
    block::{BlockDefinition, BlockRegistry},
    day_night_cycle::{DayNightCycleDefinition, DayNightCycleRegistry},
    dimension::{DimensionDefinition, DimensionRegistry},
    fluid::{FluidDefinition, FluidRegistry},
    json_file::{collect_json_files, read_json_definition},
    sky::{SkyDefinition, SkyRegistry},
    structure::{StructureDefinition, StructureRegistry},
};

#[derive(Default)]
pub(crate) struct LoadedContent {
    pub biomes: BiomeRegistry,
    pub blocks: BlockRegistry,
    pub dimensions: DimensionRegistry,
    pub day_night_cycles: DayNightCycleRegistry,
    pub fluids: FluidRegistry,
    pub skies: SkyRegistry,
    pub structures: StructureRegistry,
}

impl LoadedContent {
    pub fn insert(self, commands: &mut Commands) {
        commands.insert_resource(self.biomes);
        commands.insert_resource(self.blocks);
        commands.insert_resource(self.dimensions);
        commands.insert_resource(self.day_night_cycles);
        commands.insert_resource(self.fluids);
        commands.insert_resource(self.skies);
        commands.insert_resource(self.structures);
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
    } else if path_has_component(path, "biomes") {
        content
            .biomes
            .insert(read_json_definition::<BiomeDefinition>(path));
    } else if path_has_component(path, "blocks") {
        content
            .blocks
            .insert(read_json_definition::<BlockDefinition>(path));
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

fn path_has_component(path: &Path, component: &str) -> bool {
    let component = OsStr::new(component);
    path.components()
        .any(|candidate| candidate.as_os_str() == component)
}
