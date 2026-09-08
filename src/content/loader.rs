use std::path::Path;

use bevy::prelude::*;

use crate::app::runtime_paths::data_root;

use super::{
    biome::{BiomeDefinition, BiomeRegistry},
    block::{BlockDefinition, BlockRegistry},
    day_night_cycle::{DayNightCycleDefinition, DayNightCycleRegistry},
    dimension::{DimensionDefinition, DimensionRegistry},
    json_file::{collect_json_files, read_json_definition},
    sky::{SkyDefinition, SkyRegistry},
};

pub fn load_content(mut commands: Commands) {
    let mut biome_registry = BiomeRegistry::default();
    let mut block_registry = BlockRegistry::default();
    let mut dimension_registry = DimensionRegistry::default();
    let mut day_night_cycle_registry = DayNightCycleRegistry::default();
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
            &mut sky_registry,
        );
    }

    commands.insert_resource(biome_registry);
    commands.insert_resource(block_registry);
    commands.insert_resource(dimension_registry);
    commands.insert_resource(day_night_cycle_registry);
    commands.insert_resource(sky_registry);
}

fn load_definition(
    path: &Path,
    biome_registry: &mut BiomeRegistry,
    block_registry: &mut BlockRegistry,
    dimension_registry: &mut DimensionRegistry,
    day_night_cycle_registry: &mut DayNightCycleRegistry,
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
    }
}

fn path_has_component(path: &Path, component: &str) -> bool {
    path.components()
        .any(|candidate| candidate.as_os_str() == component)
}
