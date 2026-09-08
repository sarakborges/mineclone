use std::{
    fs,
    path::{Path, PathBuf},
};

use bevy::prelude::*;
use serde::de::DeserializeOwned;

use super::{
    biome::{BiomeDefinition, BiomeRegistry},
    block::{BlockDefinition, BlockRegistry},
    day_night_cycle::{DayNightCycleDefinition, DayNightCycleRegistry},
    dimension::{DimensionDefinition, DimensionRegistry},
    sky::{SkyDefinition, SkyRegistry},
};

const DATA_ROOT: &str = "assets/data";

pub fn load_content(mut commands: Commands) {
    let mut biome_registry = BiomeRegistry::default();
    let mut block_registry = BlockRegistry::default();
    let mut dimension_registry = DimensionRegistry::default();
    let mut day_night_cycle_registry = DayNightCycleRegistry::default();
    let mut sky_registry = SkyRegistry::default();
    let mut files = Vec::new();

    collect_ron_files(Path::new(DATA_ROOT), &mut files);

    for path in files {
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();

        if file_name == "dimension.ron" {
            dimension_registry.insert(read_definition::<DimensionDefinition>(&path));
        } else if file_name == "day_night_cycle.ron" {
            day_night_cycle_registry.insert(read_definition::<DayNightCycleDefinition>(&path));
        } else if file_name == "sky.ron" {
            sky_registry.insert(read_definition::<SkyDefinition>(&path));
        } else if path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "biomes")
        {
            biome_registry.insert(read_definition::<BiomeDefinition>(&path));
        } else if path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "blocks")
        {
            block_registry.insert(read_definition::<BlockDefinition>(&path));
        }
    }

    commands.insert_resource(biome_registry);
    commands.insert_resource(block_registry);
    commands.insert_resource(dimension_registry);
    commands.insert_resource(day_night_cycle_registry);
    commands.insert_resource(sky_registry);
}

fn collect_ron_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read content directory {}: {error}", directory.display()));

    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("failed to read content entry: {error}"))
            .path();

        if path.is_dir() {
            collect_ron_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("ron") {
            files.push(path);
        }
    }
}

fn read_definition<T: DeserializeOwned>(path: &Path) -> T {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    ron::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}
