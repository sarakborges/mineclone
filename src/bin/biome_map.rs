#![allow(
    dead_code,
    reason = "this diagnostic binary includes the production generation modules directly; the main target owns their full consumer surface"
)]

use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, sync::Arc};

use serde::Deserialize;

mod content {
    pub mod dimension {
        use serde::Deserialize;

        #[derive(Clone, Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct DimensionDefinition {
            pub id: String,
            pub sea_level: i32,
            pub gravity_strength: f32,
        }
    }

    pub mod biome {
        use std::collections::BTreeMap;

        use serde::Deserialize;

        const DEFAULT_WEIGHT: f32 = 1.0;
        const DEFAULT_REGION_MIN: u32 = 384;
        const DEFAULT_REGION_MAX: u32 = 768;

        #[derive(Clone, Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct BiomeDefinition {
            pub id: String,
            #[serde(default)]
            pub surface_layout: Option<SurfaceBiomeLayoutDefinition>,
        }

        #[derive(Clone, Debug, Deserialize, PartialEq)]
        #[serde(rename_all = "camelCase")]
        pub struct SurfaceBiomeLayoutDefinition {
            #[serde(default = "default_weight")]
            pub weight: f32,
            #[serde(default)]
            pub region_size: BiomeRegionSize,
            #[serde(default)]
            pub cannot_border: Vec<String>,
        }

        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
        #[serde(rename_all = "camelCase")]
        pub struct BiomeRegionSize {
            pub min: u32,
            pub max: u32,
        }

        impl Default for BiomeRegionSize {
            fn default() -> Self {
                Self {
                    min: DEFAULT_REGION_MIN,
                    max: DEFAULT_REGION_MAX,
                }
            }
        }

        #[derive(Clone, Default)]
        pub struct BiomeRegistry {
            definitions: BTreeMap<String, BiomeDefinition>,
        }

        impl BiomeRegistry {
            pub fn insert(&mut self, definition: BiomeDefinition) {
                self.definitions.insert(definition.id.clone(), definition);
            }

            pub fn get(&self, id: &str) -> Option<&BiomeDefinition> {
                self.definitions.get(id)
            }

            pub fn surface_for_dimension<'a>(
                &'a self,
                dimension_id: &'a str,
            ) -> impl Iterator<Item = &'a BiomeDefinition> + 'a {
                self.definitions.values().filter(move |definition| {
                    definition.surface_layout.is_some()
                        && belongs_to_dimension(&definition.id, dimension_id)
                })
            }
        }

        impl BiomeDefinition {
            pub fn validate_references(&self, biomes: &BiomeRegistry) {
                let Some(surface) = &self.surface_layout else {
                    return;
                };
                for forbidden in &surface.cannot_border {
                    assert_ne!(
                        forbidden, &self.id,
                        "biome {} cannot forbid bordering itself",
                        self.id
                    );
                    let target = biomes.get(forbidden).unwrap_or_else(|| {
                        panic!(
                            "biome {} surfaceLayout.cannotBorder references missing biome {}",
                            self.id, forbidden
                        )
                    });
                    assert!(
                        target.surface_layout.is_some(),
                        "biome {} surfaceLayout.cannotBorder target {} does not participate in the surface layout",
                        self.id,
                        forbidden
                    );
                    assert_eq!(
                        biome_dimension(&self.id),
                        biome_dimension(&target.id),
                        "biome {} surfaceLayout.cannotBorder target {} belongs to a different dimension",
                        self.id,
                        forbidden
                    );
                }
            }
        }

        fn default_weight() -> f32 {
            DEFAULT_WEIGHT
        }

        fn belongs_to_dimension(biome_id: &str, dimension_id: &str) -> bool {
            let Some((namespace, local_dimension)) = dimension_id.split_once(':') else {
                return false;
            };
            biome_dimension(biome_id)
                .is_some_and(|(biome_namespace, biome_dimension)| {
                    biome_namespace == namespace && biome_dimension == local_dimension
                })
        }

        fn biome_dimension(id: &str) -> Option<(&str, &str)> {
            let (namespace, remainder) = id.split_once(':')?;
            let (dimension, biome) = remainder.split_once('/')?;
            (!namespace.is_empty() && !dimension.is_empty() && !biome.is_empty())
                .then_some((namespace, dimension))
        }
    }
}

mod world {
    pub mod generator {
        use std::sync::Arc;

        use crate::content::{biome::BiomeRegistry, dimension::DimensionDefinition};

        mod foundation {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world/generator/foundation.rs"));
        }
        mod biome {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world/generator/biome.rs"));
        }
        mod biome_map {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world/generator/biome_map.rs"));
        }

        pub(crate) use biome::BiomeQueries;
        use biome::BiomeLayout;
        pub(crate) use biome_map::{BiomeMapConfig, render_biome_map};
        use foundation::{GenerationDimension, GenerationSeed, GenerationSnapshot};

        #[derive(Clone, Debug)]
        pub(crate) struct WorldGenerator {
            snapshot: Arc<GenerationSnapshot>,
            biomes: Arc<BiomeLayout>,
        }

        impl WorldGenerator {
            pub(crate) fn new(
                seed: u64,
                dimension: &DimensionDefinition,
                biome_registry: &BiomeRegistry,
            ) -> Self {
                let snapshot = GenerationSnapshot::new(
                    GenerationSeed::new(seed),
                    GenerationDimension::from_definition(dimension),
                );
                Self::from_snapshot(snapshot, biome_registry)
            }

            fn from_snapshot(snapshot: GenerationSnapshot, biome_registry: &BiomeRegistry) -> Self {
                let biomes = BiomeLayout::new(&snapshot, biome_registry);
                Self {
                    snapshot: Arc::new(snapshot),
                    biomes: Arc::new(biomes),
                }
            }

            pub(crate) fn biomes(&self) -> BiomeQueries<'_> {
                self.biomes.queries()
            }
        }
    }
}

use content::{biome::{BiomeDefinition, BiomeRegistry}, dimension::DimensionDefinition};
use world::generator::{BiomeMapConfig, WorldGenerator, render_biome_map};

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut seed = 0_u64;
    let mut dimension_id = "asteria:overworld".to_owned();
    let mut output = PathBuf::from("biome-map.png");
    let mut config = BiomeMapConfig::default();
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--seed" => {
                index += 1;
                seed = parse_value(&arguments, index, "--seed");
            }
            "--dimension" => {
                index += 1;
                dimension_id = value(&arguments, index, "--dimension").to_owned();
            }
            "--output" => {
                index += 1;
                output = PathBuf::from(value(&arguments, index, "--output"));
            }
            "--center-x" => {
                index += 1;
                config.center_x = parse_value(&arguments, index, "--center-x");
            }
            "--center-z" => {
                index += 1;
                config.center_z = parse_value(&arguments, index, "--center-z");
            }
            "--scale" => {
                index += 1;
                config.blocks_per_pixel = parse_value(&arguments, index, "--scale");
            }
            "--width" => {
                index += 1;
                config.width = parse_value(&arguments, index, "--width");
            }
            "--height" => {
                index += 1;
                config.height = parse_value(&arguments, index, "--height");
            }
            "--no-boundaries" => config.show_boundaries = false,
            "--no-influences" => config.show_influences = false,
            unknown => panic!("unknown biome-map argument {unknown}"),
        }
        index += 1;
    }

    let (dimension, biomes) = load_generation_content(&dimension_id);
    let generator = WorldGenerator::new(seed, &dimension, &biomes);
    render_biome_map(generator.biomes(), &config)
        .save(&output)
        .unwrap_or_else(|error| panic!("{error}"));
    println!("biome map saved: {}", output.display());
}

fn load_generation_content(dimension_id: &str) -> (DimensionDefinition, BiomeRegistry) {
    let (_, local_dimension) = dimension_id
        .split_once(':')
        .unwrap_or_else(|| panic!("dimension id must be namespaced: {dimension_id}"));
    let dimension_root = Path::new("data").join("dimensions").join(local_dimension);
    let dimension = read_json::<DimensionDefinition>(&dimension_root.join("dimension.json"));
    assert_eq!(dimension.id, dimension_id, "dimension path/id mismatch");

    let mut biomes = BiomeRegistry::default();
    let biome_root = dimension_root.join("biomes");
    let mut files = fs::read_dir(&biome_root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", biome_root.display()))
        .map(|entry| entry.expect("failed to read biome directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .collect::<Vec<_>>();
    files.sort();
    for path in files {
        biomes.insert(read_json::<BiomeDefinition>(&path));
    }
    (dimension, biomes)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn value<'a>(arguments: &'a [String], index: usize, flag: &str) -> &'a str {
    arguments
        .get(index)
        .unwrap_or_else(|| panic!("{flag} requires a value"))
}

fn parse_value<T>(arguments: &[String], index: usize, flag: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = value(arguments, index, flag);
    raw.parse::<T>()
        .unwrap_or_else(|error| panic!("invalid {flag} value {raw}: {error}"))
}
