#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod content;
mod creatures;
mod entity;
mod gameplay;
mod hud;
mod localization;
mod player;
mod rendering;
mod screens;
mod targeting;
mod tools;
mod ui;
mod voxel;
mod world;
mod world_items;
mod world_objects;

#[cfg(debug_assertions)]
#[expect(
    unused_imports,
    clippy::single_component_path_imports,
    reason = "the debug import intentionally enables Bevy dynamic linking"
)]
use bevy_dylib;

use std::path::PathBuf;

use app::{
    controls_state::ControlsState,
    crash_log::{install_crash_logger, log_system_event, mark_clean_shutdown, write_caught_panic},
    game_config::GameConfigPlugin,
    game_state::GameState,
    pause_state::PauseState,
    runtime_paths::prepare_runtime_directory,
    window_icon::WindowIconPlugin,
};
#[cfg(target_os = "windows")]
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};
use bevy::{
    app::{TaskPoolOptions, TaskPoolPlugin},
    prelude::*,
    window::PresentMode,
};
use content::{ContentPlugin, builtin_ids::OVERWORLD_DIMENSION_ID};
use creatures::CreaturesPlugin;
use gameplay::GameplayPlugin;
use hud::HudPlugin;
use localization::LocalizationPlugin;
use rendering::RenderingPlugin;
use screens::ScreensPlugin;
use targeting::{biome_tint::BiomeTintInteractionPlugin, block::BlockTargetingPlugin};
use tools::ToolsPlugin;
use ui::UiDesignSystemPlugin;
use voxel::block_gravity::BlockGravityPlugin;
use world::{
    WorldPlugin,
    destination::find_generated_surface_destination,
    generator::{
        BiomeMapConfig, StructureDebugConfig, TerrainDebugConfig, WorldGenerator,
        render_biome_map, render_terrain_debug, validate_structure_debug,
    },
};
use world_items::WorldItemsPlugin;
use world_objects::WorldObjectsPlugin;

fn main() {
    install_crash_logger();

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_game)) {
        Ok(()) => mark_clean_shutdown(),
        Err(payload) => {
            write_caught_panic(payload.as_ref());
            std::panic::resume_unwind(payload);
        }
    }
}

fn run_game() {
    prepare_runtime_directory();
    if run_worldgen_benchmark_cli()
        || run_biome_map_cli()
        || run_terrain_debug_cli()
        || run_structure_debug_cli()
    {
        return;
    }
    log_system_event(format!(
        "app.start debug={} backend_override={} present_mode={:?} io_pool_percent=0.10 io_pool_max_threads=2 async_compute_percent=0.50 async_compute_max_threads=8",
        cfg!(debug_assertions),
        std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "<default>".to_owned()),
        primary_present_mode(),
    ));

    let default_plugins = DefaultPlugins
        .set(TaskPoolPlugin {
            task_pool_options: voxel_task_pool_options(),
        })
        .set(ImagePlugin::default_nearest())
        .set(WindowPlugin {
            // World exit owns durability. The OS close button must not
            // destroy the window before the active world is saved.
            close_when_requested: false,
            primary_window: Some(primary_window()),
            ..default()
        });
    #[cfg(target_os = "windows")]
    let default_plugins = default_plugins.set(RenderPlugin {
        render_creation: windows_wgpu_settings().into(),
        ..default()
    });

    App::new()
        .add_plugins(default_plugins)
        .init_state::<GameState>()
        .init_state::<PauseState>()
        .init_state::<ControlsState>()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.04)))
        .add_systems(OnEnter(GameState::StartingScreen), log_game_state)
        .add_systems(OnEnter(GameState::NewWorld), log_game_state)
        .add_systems(OnEnter(GameState::WorldSelection), log_game_state)
        .add_systems(OnEnter(GameState::Loading), log_game_state)
        .add_systems(OnEnter(GameState::Gameplay), log_game_state)
        .add_systems(OnEnter(PauseState::Running), log_pause_state)
        .add_systems(OnEnter(PauseState::Paused), log_pause_state)
        .add_plugins((
            WindowIconPlugin,
            GameConfigPlugin,
            UiDesignSystemPlugin,
            LocalizationPlugin,
            ContentPlugin,
            ScreensPlugin,
            WorldPlugin,
            GameplayPlugin,
        ))
        .add_plugins((
            CreaturesPlugin,
            RenderingPlugin,
            WorldObjectsPlugin,
            WorldItemsPlugin,
            BlockGravityPlugin,
            BlockTargetingPlugin,
            ToolsPlugin,
            HudPlugin,
        ))
        .add_plugins(BiomeTintInteractionPlugin)
        .add_plugins(targeting::portal_activation_plugin())
        .run();
}

fn run_biome_map_cli() -> bool {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if !arguments.iter().any(|argument| argument == "--biome-map") {
        return false;
    }

    let mut seed = 0_u64;
    let mut dimension_id = OVERWORLD_DIMENSION_ID.to_owned();
    let mut output = PathBuf::from("biome-map.png");
    let mut config = BiomeMapConfig::default();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--biome-map" => {}
            "--seed" => {
                index += 1;
                seed = parse_cli_value(&arguments, index, "--seed");
            }
            "--dimension" => {
                index += 1;
                dimension_id = cli_value(&arguments, index, "--dimension").to_owned();
            }
            "--output" => {
                index += 1;
                output = PathBuf::from(cli_value(&arguments, index, "--output"));
            }
            "--center-x" => {
                index += 1;
                config.center_x = parse_cli_value(&arguments, index, "--center-x");
            }
            "--center-z" => {
                index += 1;
                config.center_z = parse_cli_value(&arguments, index, "--center-z");
            }
            "--scale" => {
                index += 1;
                config.blocks_per_pixel = parse_cli_value(&arguments, index, "--scale");
            }
            "--width" => {
                index += 1;
                config.width = parse_cli_value(&arguments, index, "--width");
            }
            "--height" => {
                index += 1;
                config.height = parse_cli_value(&arguments, index, "--height");
            }
            "--no-boundaries" => config.show_boundaries = false,
            "--no-influences" => config.show_influences = false,
            unknown => panic!("unknown biome-map argument {unknown}"),
        }
        index += 1;
    }

    let loaded = content::read_content();
    let dimension = loaded.dimensions.get(&dimension_id).unwrap_or_else(|| {
        panic!("biome map references missing dimension {dimension_id}")
    });
    let generator = WorldGenerator::new(seed, dimension, &loaded.biomes);
    let render = render_biome_map(generator.biomes(), &config);
    render
        .save(&output)
        .unwrap_or_else(|error| panic!("{error}"));
    println!(
        "biome map saved: {} (legend: {})",
        output.display(),
        output.with_extension("legend.json").display()
    );
    true
}

fn run_terrain_debug_cli() -> bool {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if !arguments.iter().any(|argument| argument == "--terrain-debug") {
        return false;
    }

    let mut seed = 0_u64;
    let mut dimension_id = OVERWORLD_DIMENSION_ID.to_owned();
    let mut output = PathBuf::from("terrain-debug.png");
    let mut config = TerrainDebugConfig::default();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--terrain-debug" => {}
            "--seed" => {
                index += 1;
                seed = parse_cli_value(&arguments, index, "--seed");
            }
            "--dimension" => {
                index += 1;
                dimension_id = cli_value(&arguments, index, "--dimension").to_owned();
            }
            "--output" => {
                index += 1;
                output = PathBuf::from(cli_value(&arguments, index, "--output"));
            }
            "--center-x" => {
                index += 1;
                config.center_x = parse_cli_value(&arguments, index, "--center-x");
            }
            "--center-y" => {
                index += 1;
                config.center_y = parse_cli_value(&arguments, index, "--center-y");
            }
            "--center-z" => {
                index += 1;
                config.center_z = parse_cli_value(&arguments, index, "--center-z");
            }
            "--scale" => {
                index += 1;
                config.blocks_per_pixel = parse_cli_value(&arguments, index, "--scale");
            }
            "--width" => {
                index += 1;
                config.width = parse_cli_value(&arguments, index, "--width");
            }
            "--height" => {
                index += 1;
                config.height = parse_cli_value(&arguments, index, "--height");
            }
            unknown => panic!("unknown terrain-debug argument {unknown}"),
        }
        index += 1;
    }

    let loaded = content::read_content();
    let dimension = loaded.dimensions.get(&dimension_id).unwrap_or_else(|| {
        panic!("terrain debug references missing dimension {dimension_id}")
    });
    let generator = WorldGenerator::new(seed, dimension, &loaded.biomes);
    let render = render_terrain_debug(generator.terrain(), &config);
    render
        .save(&output)
        .unwrap_or_else(|error| panic!("{error}"));
    if !render.validation_passed() {
        panic!(
            "terrain debug invariant validation failed; inspect {}",
            output.with_extension("debug.json").display()
        );
    }
    println!(
        "terrain debug saved: {} (density slice and metadata saved beside it)",
        output.display()
    );
    true
}

fn run_structure_debug_cli() -> bool {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if !arguments.iter().any(|argument| argument == "--structure-debug") {
        return false;
    }

    let mut seed = 0_u64;
    let mut dimension_id = OVERWORLD_DIMENSION_ID.to_owned();
    let mut output = PathBuf::from("structure-debug.json");
    let mut config = StructureDebugConfig::default();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--structure-debug" => {}
            "--seed" => {
                index += 1;
                seed = parse_cli_value(&arguments, index, "--seed");
            }
            "--dimension" => {
                index += 1;
                dimension_id = cli_value(&arguments, index, "--dimension").to_owned();
            }
            "--output" => {
                index += 1;
                output = PathBuf::from(cli_value(&arguments, index, "--output"));
            }
            "--center-x" => {
                index += 1;
                config.center_x = parse_cli_value(&arguments, index, "--center-x");
            }
            "--center-z" => {
                index += 1;
                config.center_z = parse_cli_value(&arguments, index, "--center-z");
            }
            "--search-radius" => {
                index += 1;
                config.search_radius = parse_cli_value(&arguments, index, "--search-radius");
            }
            "--window-size" => {
                index += 1;
                config.window_size = parse_cli_value(&arguments, index, "--window-size");
            }
            unknown => panic!("unknown structure-debug argument {unknown}"),
        }
        index += 1;
    }

    let loaded = content::read_content();
    let dimension = loaded.dimensions.get(&dimension_id).unwrap_or_else(|| {
        panic!("structure debug references missing dimension {dimension_id}")
    });
    let generator = WorldGenerator::new_with_structures(
        seed,
        dimension,
        &loaded.biomes,
        &loaded.structures,
        &loaded.structure_sets,
    );
    let report = validate_structure_debug(generator.biomes(), generator.structures(), &config);
    report
        .save(&output)
        .unwrap_or_else(|error| panic!("{error}"));
    if !report.validation_passed() {
        panic!(
            "Structure debug invariant validation failed; inspect {}",
            output.display()
        );
    }
    println!("Structure debug validation passed: {}", output.display());
    true
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkMeasurement {
    operations: u32,
    cold_ns: u64,
    warm_ns: u64,
    cold_ns_per_operation: f64,
    warm_ns_per_operation: f64,
}

impl BenchmarkMeasurement {
    fn new(operations: u32, cold: std::time::Duration, warm: std::time::Duration) -> Self {
        let operations_f64 = f64::from(operations.max(1));
        let cold_ns = duration_ns(cold);
        let warm_ns = duration_ns(warm);
        Self {
            operations,
            cold_ns,
            warm_ns,
            cold_ns_per_operation: cold_ns as f64 / operations_f64,
            warm_ns_per_operation: warm_ns as f64 / operations_f64,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WorldgenBenchmarkReport {
    seed: u64,
    dimension: String,
    center: [i32; 2],
    far_probe: [i32; 2],
    scalar_samples: u32,
    area_size: u32,
    search_radius: u32,
    destination_radius: i32,
    metrics: std::collections::BTreeMap<String, BenchmarkMeasurement>,
    notes: Vec<String>,
}

fn run_worldgen_benchmark_cli() -> bool {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if !arguments
        .iter()
        .any(|argument| argument == "--worldgen-benchmark")
    {
        return false;
    }

    let mut seed = 0_u64;
    let mut dimension_id = OVERWORLD_DIMENSION_ID.to_owned();
    let mut output = PathBuf::from("worldgen-benchmark.json");
    let mut center_x = 0_i32;
    let mut center_z = 0_i32;
    let mut far_x = 250_000_i32;
    let mut far_z = -250_000_i32;
    let mut scalar_samples = 4_096_u32;
    let mut area_size = 64_u32;
    let mut search_radius = 8_192_u32;
    let mut destination_radius = 64_i32;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--worldgen-benchmark" => {}
            "--seed" => {
                index += 1;
                seed = parse_cli_value(&arguments, index, "--seed");
            }
            "--dimension" => {
                index += 1;
                dimension_id = cli_value(&arguments, index, "--dimension").to_owned();
            }
            "--output" => {
                index += 1;
                output = PathBuf::from(cli_value(&arguments, index, "--output"));
            }
            "--center-x" => {
                index += 1;
                center_x = parse_cli_value(&arguments, index, "--center-x");
            }
            "--center-z" => {
                index += 1;
                center_z = parse_cli_value(&arguments, index, "--center-z");
            }
            "--far-x" => {
                index += 1;
                far_x = parse_cli_value(&arguments, index, "--far-x");
            }
            "--far-z" => {
                index += 1;
                far_z = parse_cli_value(&arguments, index, "--far-z");
            }
            "--samples" => {
                index += 1;
                scalar_samples = parse_cli_value(&arguments, index, "--samples");
            }
            "--area-size" => {
                index += 1;
                area_size = parse_cli_value(&arguments, index, "--area-size");
            }
            "--search-radius" => {
                index += 1;
                search_radius = parse_cli_value(&arguments, index, "--search-radius");
            }
            "--destination-radius" => {
                index += 1;
                destination_radius = parse_cli_value(&arguments, index, "--destination-radius");
            }
            unknown => panic!("unknown worldgen-benchmark argument {unknown}"),
        }
        index += 1;
    }

    assert!(
        (1..=100_000).contains(&scalar_samples),
        "--samples must be within 1..=100000"
    );
    assert!(
        (1..=512).contains(&area_size),
        "--area-size must be within 1..=512"
    );
    assert!(
        (1..=1_000_000).contains(&search_radius),
        "--search-radius must be within 1..=1000000"
    );
    assert!(
        (0..=4_096).contains(&destination_radius),
        "--destination-radius must be within 0..=4096"
    );

    let loaded = content::read_content();
    let dimension = loaded.dimensions.get(&dimension_id).unwrap_or_else(|| {
        panic!("worldgen benchmark references missing dimension {dimension_id}")
    });
    let generator = WorldGenerator::new_runtime(
        seed,
        dimension,
        &loaded.biomes,
        &loaded.blocks,
        &loaded.fluids,
        &loaded.structures,
        &loaded.structure_sets,
    );

    let mut metrics = std::collections::BTreeMap::new();
    let mut notes = Vec::new();
    let area_half = i32::try_from(area_size / 2).expect("validated area size must fit i32");
    let area_origin_x = center_x.saturating_sub(area_half);
    let area_origin_z = center_z.saturating_sub(area_half);

    metrics.insert(
        "biomeScalar".to_owned(),
        measure_benchmark(scalar_samples, || {
            let queries = generator.biomes();
            for sample in 0..scalar_samples {
                let (x, z) = benchmark_sample_position(center_x, center_z, sample);
                std::hint::black_box(queries.surface_biome_at(x, z));
            }
        }),
    );
    metrics.insert(
        "biomeArea".to_owned(),
        measure_benchmark(1, || {
            std::hint::black_box(generator.biomes().sample_surface_area(
                area_origin_x,
                area_origin_z,
                area_size,
                area_size,
            ));
        }),
    );

    let mut map_config = BiomeMapConfig::default();
    map_config.center_x = center_x;
    map_config.center_z = center_z;
    map_config.width = area_size;
    map_config.height = area_size;
    map_config.blocks_per_pixel = 4;
    metrics.insert(
        "biomeMapRender".to_owned(),
        measure_benchmark(1, || {
            std::hint::black_box(render_biome_map(generator.biomes(), &map_config));
        }),
    );

    metrics.insert(
        "terrainScalar".to_owned(),
        measure_benchmark(scalar_samples, || {
            let queries = generator.terrain();
            for sample in 0..scalar_samples {
                let (x, z) = benchmark_sample_position(center_x, center_z, sample);
                let surface = queries.surface_at(x, z);
                std::hint::black_box((surface, queries.density_at(x, surface, z)));
            }
        }),
    );
    metrics.insert(
        "terrainArea".to_owned(),
        measure_benchmark(1, || {
            std::hint::black_box(generator.terrain().sample_surface_area(
                area_origin_x,
                area_origin_z,
                area_size,
                area_size,
            ));
        }),
    );

    let locate_probe = IVec2::new(
        center_x.saturating_add(4_096),
        center_z.saturating_sub(4_096),
    );
    let locate_biome = generator
        .biomes()
        .surface_biome_at(locate_probe.x, locate_probe.y)
        .primary()
        .as_str()
        .to_owned();
    metrics.insert(
        "locateBiome".to_owned(),
        measure_benchmark(1, || {
            std::hint::black_box(generator.biomes().find_surface_biome(
                &locate_biome,
                center_x,
                center_z,
                search_radius,
            ));
        }),
    );

    if let Some(root) = dimension.generated_surface_structures.first() {
        let reference = root.structure.clone();
        metrics.insert(
            "structureSearch".to_owned(),
            measure_benchmark(1, || {
                std::hint::black_box(generator.structures().find_nearest(
                    &reference,
                    center_x,
                    center_z,
                    search_radius,
                ));
            }),
        );
        metrics.insert(
            "locateStructure".to_owned(),
            measure_benchmark(1, || {
                std::hint::black_box(generator.structures().find_nearest(
                    &reference,
                    far_x,
                    far_z,
                    search_radius,
                ));
            }),
        );
    } else {
        notes.push("dimension has no authored generated surface Structure roots; structure search metrics omitted".to_owned());
    }

    metrics.insert(
        "farDestination".to_owned(),
        measure_benchmark(1, || {
            std::hint::black_box(find_generated_surface_destination(
                &generator,
                IVec2::new(far_x, far_z),
                destination_radius,
                |_| true,
            ));
        }),
    );

    let near_chunk = benchmark_surface_chunk(&generator, center_x, center_z);
    metrics.insert(
        "chunkSynthesisNear".to_owned(),
        measure_benchmark(1, || {
            std::hint::black_box(generator.materialize_chunk(near_chunk));
        }),
    );
    let far_chunk = benchmark_surface_chunk(&generator, far_x, far_z);
    metrics.insert(
        "chunkSynthesisFar".to_owned(),
        measure_benchmark(1, || {
            std::hint::black_box(generator.materialize_chunk(far_chunk));
        }),
    );

    notes.push(
        "No thresholds are applied: this command records the first evidence-based cold/warm baseline."
            .to_owned(),
    );
    notes.push(
        "Initial playable-area wall-clock and peak temporary memory remain runtime-pipeline measurements and are not approximated by this query benchmark."
            .to_owned(),
    );

    let report = WorldgenBenchmarkReport {
        seed,
        dimension: dimension_id,
        center: [center_x, center_z],
        far_probe: [far_x, far_z],
        scalar_samples,
        area_size,
        search_radius,
        destination_radius,
        metrics,
        notes,
    };
    let serialized = serde_json::to_string_pretty(&report)
        .expect("worldgen benchmark report must serialize to JSON");
    std::fs::write(&output, serialized).unwrap_or_else(|error| {
        panic!(
            "failed to write worldgen benchmark {}: {error}",
            output.display()
        )
    });
    println!("worldgen benchmark saved: {}", output.display());
    true
}

fn measure_benchmark(operations: u32, mut run: impl FnMut()) -> BenchmarkMeasurement {
    let cold_start = std::time::Instant::now();
    run();
    let cold = cold_start.elapsed();
    let warm_start = std::time::Instant::now();
    run();
    let warm = warm_start.elapsed();
    BenchmarkMeasurement::new(operations, cold, warm)
}

fn duration_ns(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

fn benchmark_sample_position(center_x: i32, center_z: i32, sample: u32) -> (i32, i32) {
    let dx = i32::try_from(sample.wrapping_mul(73) % 4_096)
        .expect("bounded benchmark X offset must fit i32")
        - 2_048;
    let dz = i32::try_from(sample.wrapping_mul(151) % 4_096)
        .expect("bounded benchmark Z offset must fit i32")
        - 2_048;
    (center_x.saturating_add(dx), center_z.saturating_add(dz))
}

fn benchmark_surface_chunk(generator: &WorldGenerator, x: i32, z: i32) -> IVec3 {
    let chunk_size = voxel::chunk::CHUNK_SIZE as i32;
    let surface_y = generator.terrain().surface_at(x, z).max(0);
    IVec3::new(
        x.div_euclid(chunk_size),
        surface_y.div_euclid(chunk_size),
        z.div_euclid(chunk_size),
    )
}

fn cli_value<'a>(arguments: &'a [String], index: usize, flag: &str) -> &'a str {
    arguments
        .get(index)
        .unwrap_or_else(|| panic!("{flag} requires a value"))
}

fn parse_cli_value<T>(arguments: &[String], index: usize, flag: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = cli_value(arguments, index, flag);
    value
        .parse::<T>()
        .unwrap_or_else(|error| panic!("invalid {flag} value {value}: {error}"))
}

fn log_game_state(state: Res<State<GameState>>) {
    log_system_event(format!("state.game entered={:?}", state.get()));
}

fn log_pause_state(state: Res<State<PauseState>>) {
    log_system_event(format!("state.pause entered={:?}", state.get()));
}

#[cfg(target_os = "windows")]
fn windows_wgpu_settings() -> WgpuSettings {
    let mut settings = WgpuSettings::default();

    // Bevy 0.19 / wgpu 29 currently has a Vulkan VRAM-residency regression on
    // Windows. Prefer DX12 unless the user explicitly chose a backend through
    // WGPU_BACKEND, preserving the standard wgpu escape hatch for debugging or
    // unsupported hardware.
    if std::env::var_os("WGPU_BACKEND").is_none() {
        settings.backends = Some(Backends::DX12);
    }

    settings
}

fn primary_window() -> Window {
    let mut window = Window {
        title: "Asteria".into(),
        mode: bevy::window::WindowMode::Windowed,
        present_mode: primary_present_mode(),
        ..default()
    };
    window.set_maximized(true);
    window
}

fn primary_present_mode() -> PresentMode {
    // The Windows renderer is explicitly DX12. Mailbox is supported by the
    // DX11/12 presentation path and avoids FIFO's hard 60 -> 30 FPS step when
    // a frame narrowly misses a vblank, while still presenting without tearing.
    #[cfg(target_os = "windows")]
    {
        if std::env::var_os("WGPU_BACKEND").is_none() {
            PresentMode::Mailbox
        } else {
            PresentMode::AutoVsync
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        PresentMode::AutoVsync
    }
}

fn voxel_task_pool_options() -> TaskPoolOptions {
    let mut options = TaskPoolOptions::default();
    options.io.percent = 0.10;
    options.io.max_threads = 2;
    options.async_compute.percent = 0.50;
    options.async_compute.max_threads = 8;
    options
}