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
};

pub(crate) struct LoadedContent {
    pub biomes: BiomeRegistry,
    pub blocks: BlockRegistry,
    pub dimensions: DimensionRegistry,
    pub day_night_cycles: DayNightCycleRegistry,
    pub fluids: FluidRegistry,
    pub skies: SkyRegistry,
}

impl LoadedContent {
    pub fn insert(self, commands: &mut Commands) {
        commands.insert_resource(self.biomes);
        commands.insert_resource(self.blocks);
        commands.insert_resource(self.dimensions);
        commands.insert_resource(self.day_night_cycles);
        commands.insert_resource(self.fluids);
        commands.insert_resource(self.skies);
    }
}

pub fn load_content(mut commands: Commands) {
    read_content().insert(&mut commands);
}

pub(crate) fn read_content() -> LoadedContent {
    let mut biome_registry = BiomeRegistry::default();
    let mut block_registry = BlockRegistry::default();
    let mut dimension_registry = DimensionRegistry::default();
    let mut day_night_cycle_registry = DayNightCycleRegistry::default();
    let mut fluid_registry = FluidRegistry::default();
    let mut sky_registry = SkyRegistry::default();
    let mut files = Vec::new();

    collect_json_files(&data_root(), &mut files);

    for path in files {
        load_definition(
            &path,
            &mut biome_registry,
            &mut block_registry,
            &mut dimension_registry,
            &mut day_night_cycle_registry,
            &mut fluid_registry,
            &mut sky_registry,
        );
    }

    for biome in biome_registry.iter() {
        biome.validate_material_references(&block_registry);
    }

    LoadedContent {
        biomes: biome_registry,
        blocks: block_registry,
        dimensions: dimension_registry,
        day_night_cycles: day_night_cycle_registry,
        fluids: fluid_registry,
        skies: sky_registry,
    }
}

fn load_definition(
    path: &Path,
    biome_registry: &mut BiomeRegistry,
    block_registry: &mut BlockRegistry,
    dimension_registry: &mut DimensionRegistry,
    day_night_cycle_registry: &mut DayNightCycleRegistry,
    fluid_registry: &mut FluidRegistry,
    sky_registry: &mut SkyRegistry,
) {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if file_name == "dimension.json" {
        dimension_registry.insert(read_json_definition::<DimensionDefinition>(path));
    } else if file_name == "day_night_cycle.json" {
        day_night_cycle_registry.insert(read_json_definition::<DayNightCycleDefinition>(path));
    } else if file_name == "sky.json" {
        sky_registry.insert(read_json_definition::<SkyDefinition>(path));
    } else if path_has_component(path, "biomes") {
        biome_registry.insert(read_json_definition::<BiomeDefinition>(path));
    } else if path_has_component(path, "blocks") {
        block_registry.insert(read_json_definition::<BlockDefinition>(path));
    } else if path_has_component(path, "fluids") {
        fluid_registry.insert(read_json_definition::<FluidDefinition>(path));
    }
}

fn path_has_component(path: &Path, component: &str) -> bool {
    let component = OsStr::new(component);
    path.components()
        .any(|candidate| candidate.as_os_str() == component)
}
