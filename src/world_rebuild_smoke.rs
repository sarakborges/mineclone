#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
#![allow(
    dead_code,
    unused_imports,
    reason = "this diagnostic binary reuses the application module tree, so main-only code is intentionally unreachable here"
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

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetApp, AssetPlugin},
    ecs::system::SystemState,
    prelude::*,
    render::storage::ShaderBuffer,
};

use app::{game_state::GameState, runtime_paths::prepare_runtime_directory};
use content::{
    builtin_ids::{OVERWORLD_DIMENSION_ID, UMBRAL_DIMENSION_ID},
    loader::LoadedContent,
};
use creatures::PendingCreatureRestores;
use gameplay::storage_box::StorageBoxStorage;
use player::{
    PlayerEntity,
    game_mode::GameMode,
    hotbar::PlayerHotbar,
    player_id::LOCAL_PLAYER_ID,
};
use rendering::terrain_material::TerrainMaterial;
use world::{
    InMemoryWorldSave, NewWorldConfig, WorldLoadMode, WorldPlugin, WorldSeed,
    destination::find_generated_surface_destination,
    dimension::{CurrentDimension, DimensionId},
    dimension_persistence::{InactiveDimensionState, InactiveDimensionStates},
    fluid_updates::PendingFluidUpdates,
    game_rules::GameRules,
    generator::WorldGenerator,
    render_distance::RenderDistanceSettings,
    save_catalog::{SaveRegistries, delete_world, load_world},
    save_session::{WorldSaveContext, WorldSession},
    warp::{PendingWarp, WarpOutcome},
};

const SMOKE_SEED: u64 = 0xA57E_12A5_5EED_u64;
const SMOKE_TIMEOUT: Duration = Duration::from_secs(45);
const SAME_DIMENSION_PROBE: IVec2 = IVec2::new(96, -96);
const UMBRAL_PROBE: IVec2 = IVec2::new(-128, 128);

fn main() {
    prepare_runtime_directory();
    run_world_rebuild_smoke();
}

fn run_world_rebuild_smoke() {
    let mut cleanup = SmokeWorldCleanup::default();
    let mut app = build_smoke_app(content::read_content());

    {
        let mut config = app.world_mut().resource_mut::<NewWorldConfig>();
        config.set_name(format!("World Rebuild Smoke {}", std::process::id()));
        config.set_seed(SMOKE_SEED);
    }
    app.world_mut()
        .resource_mut::<RenderDistanceSettings>()
        .set_chunks(4);
    request_state(&mut app, GameState::Loading);
    pump_until_state(&mut app, GameState::Gameplay, "new-world loading");

    let world_id = app
        .world()
        .resource::<WorldSession>()
        .id()
        .expect("new-world smoke must establish a world session")
        .to_owned();
    cleanup.id = Some(world_id.clone());

    let same_dimension_target = generated_destination(
        &app,
        OVERWORLD_DIMENSION_ID,
        SAME_DIMENSION_PROBE,
    );
    app.world_mut()
        .resource_mut::<PendingWarp>()
        .request(same_dimension_target, None);
    pump_until_warp_outcome(&mut app, "same-dimension warp");

    let umbral_target = generated_destination(&app, UMBRAL_DIMENSION_ID, UMBRAL_PROBE);
    app.world_mut()
        .resource_mut::<PendingWarp>()
        .request(umbral_target, Some(UMBRAL_DIMENSION_ID));
    pump_dimension_transition(&mut app, UMBRAL_DIMENSION_ID);

    persist_active_world(&mut app);
    drop(app);

    let (mut loaded_app, expected_player_position) = load_smoke_app(&world_id);
    loaded_app
        .world_mut()
        .resource_mut::<RenderDistanceSettings>()
        .set_chunks(4);
    request_state(&mut loaded_app, GameState::Loading);
    pump_until_state(&mut loaded_app, GameState::Gameplay, "save-load loading");

    assert_eq!(
        loaded_app
            .world()
            .resource::<CurrentDimension>()
            .id
            .as_str(),
        UMBRAL_DIMENSION_ID,
        "loaded smoke world must resume in the saved dimension"
    );
    assert!(
        loaded_app
            .world()
            .resource::<InactiveDimensionStates>()
            .get(OVERWORLD_DIMENSION_ID)
            .is_some(),
        "loaded smoke world must restore the inactive Overworld runtime state"
    );

    let restored_position = single_player_position(&mut loaded_app);
    assert!(
        restored_position.distance(Vec3::from_array(expected_player_position)) <= 0.001,
        "loaded player position must match the persisted position: expected={:?} actual={:?}",
        expected_player_position,
        restored_position
    );

    drop(loaded_app);
    cleanup.remove_now();
    println!(
        "world rebuild smoke passed: new -> warp -> dimension -> save -> load (seed={SMOKE_SEED})"
    );
}

fn build_smoke_app(content: LoadedContent) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_state::<GameState>()
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<TerrainMaterial>()
        .init_asset::<ShaderBuffer>()
        .init_resource::<PlayerHotbar>()
        .init_resource::<StorageBoxStorage>()
        .init_resource::<PendingCreatureRestores>()
        .add_plugins(WorldPlugin);

    insert_content(&mut app, content);
    app.finish();
    app.cleanup();
    app
}

fn insert_content(app: &mut App, content: LoadedContent) {
    let world = app.world_mut();
    world.insert_resource(content.ambient_particles);
    world.insert_resource(content.attacks);
    world.insert_resource(content.biomes);
    world.insert_resource(content.blocks);
    world.insert_resource(content.crafting_recipes);
    world.insert_resource(content.creatures);
    world.insert_resource(content.player);
    world.insert_resource(content.dimensions);
    world.insert_resource(content.day_night_cycles);
    world.insert_resource(content.fluids);
    world.insert_resource(content.inventory_categories);
    world.insert_resource(content.items);
    world.insert_resource(content.layers);
    world.insert_resource(content.objects);
    world.insert_resource(content.portals);
    world.insert_resource(content.secondary_properties);
    world.insert_resource(content.skies);
    world.insert_resource(content.structures);
    world.insert_resource(content.structure_sets);
    world.insert_resource(content.tools);
    world.insert_resource(content.tool_categories);
    world.insert_resource(content.world_recipes);
}

fn smoke_tick(app: &mut App) {
    app.world_mut().run_schedule(StateTransition);
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut().run_schedule(Update);
    std::thread::yield_now();
}

fn request_state(app: &mut App, state: GameState) {
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(state);
}

fn pump_until_state(app: &mut App, target: GameState, label: &str) {
    let started = Instant::now();
    loop {
        smoke_tick(app);
        if *app.world().resource::<State<GameState>>().get() == target {
            return;
        }
        assert!(
            started.elapsed() < SMOKE_TIMEOUT,
            "world rebuild smoke timed out during {label}; current_state={:?}",
            app.world().resource::<State<GameState>>().get()
        );
    }
}

fn pump_until_warp_outcome(app: &mut App, label: &str) {
    let started = Instant::now();
    loop {
        smoke_tick(app);
        if let Some(outcome) = app.world_mut().resource_mut::<PendingWarp>().take_outcome() {
            match outcome {
                WarpOutcome::Succeeded(_) => return,
                WarpOutcome::Failed => panic!("world rebuild smoke failed during {label}"),
            }
        }
        assert!(
            started.elapsed() < SMOKE_TIMEOUT,
            "world rebuild smoke timed out during {label}"
        );
    }
}

fn pump_dimension_transition(app: &mut App, target_dimension: &str) {
    let started = Instant::now();
    let mut saw_loading = false;
    loop {
        smoke_tick(app);
        let state = *app.world().resource::<State<GameState>>().get();
        saw_loading |= state == GameState::Loading;
        if saw_loading
            && state == GameState::Gameplay
            && app
                .world()
                .resource::<CurrentDimension>()
                .id
                .as_str()
                == target_dimension
        {
            return;
        }
        assert!(
            started.elapsed() < SMOKE_TIMEOUT,
            "world rebuild smoke timed out during dimension transition; target={target_dimension} state={state:?}"
        );
    }
}

fn generated_destination(app: &App, dimension_id: &str, column: IVec2) -> IVec3 {
    let world = app.world();
    let dimensions = world.resource::<content::dimension::DimensionRegistry>();
    let biomes = world.resource::<content::biome::BiomeRegistry>();
    let blocks = world.resource::<content::block::BlockRegistry>();
    let fluids = world.resource::<content::fluid::FluidRegistry>();
    let structures = world.resource::<content::structure::StructureRegistry>();
    let structure_sets = world.resource::<content::structure_set::StructureSetRegistry>();
    let dimension = dimensions
        .get(dimension_id)
        .unwrap_or_else(|| panic!("smoke references missing dimension {dimension_id}"));
    let seed = world.resource::<WorldSeed>().0;
    let generator = WorldGenerator::new_runtime(
        seed,
        dimension,
        biomes,
        blocks,
        fluids,
        structures,
        structure_sets,
    );
    find_generated_surface_destination(&generator, column, 64, |_| true).unwrap_or_else(|| {
        panic!("smoke could not resolve a generated destination near {column:?} in {dimension_id}")
    })
}

fn persist_active_world(app: &mut App) {
    let world = app.world_mut();
    let mut state = SystemState::<(Res<WorldSession>, WorldSaveContext)>::new(world);
    let result = {
        let (session, snapshot) = state
            .get(world)
            .expect("smoke world save params must be available");
        session.persist(&snapshot)
    };
    state.apply(world);
    result.unwrap_or_else(|error| panic!("world rebuild smoke save failed: {error}"));
}

fn load_smoke_app(id: &str) -> (App, [f32; 3]) {
    let content = content::read_content();
    let registries = save_registries(&content);
    let (mut snapshot, world, mut inactive_worlds, session_lock) =
        load_world(id, registries).unwrap_or_else(|error| {
            panic!("world rebuild smoke load failed for {id}: {error}")
        });

    let expected_player_position = snapshot
        .player
        .as_ref()
        .expect("smoke save must contain a player")
        .position;
    let pending_fluids = PendingFluidUpdates::from_saved(&snapshot.fluid_updates, &content.fluids)
        .unwrap_or_else(|error| panic!("smoke could not restore pending fluids: {error}"));
    let inventory = PlayerHotbar::from_saved_items_and_selection(
        &snapshot.inventory,
        snapshot.selected_hotbar_slot,
        &content.items,
        &content.blocks,
        &content.layers,
        &content.objects,
        &content.tools,
    )
    .unwrap_or_else(|error| panic!("smoke could not restore inventory: {error}"));
    let storage_boxes = StorageBoxStorage::from_saved_boxes(
        &snapshot.storage_boxes,
        &content.items,
        &content.blocks,
        &content.layers,
        &content.objects,
        &content.tools,
    )
    .unwrap_or_else(|error| panic!("smoke could not restore storage boxes: {error}"));

    let mut inactive_dimensions = InactiveDimensionStates::default();
    for saved in std::mem::take(&mut snapshot.inactive_dimensions) {
        let inactive_world = inactive_worlds.remove(&saved.dimension_id).unwrap_or_else(|| {
            panic!(
                "smoke loaded snapshot without dimension world {}",
                saved.dimension_id
            )
        });
        let storage = StorageBoxStorage::from_saved_boxes(
            &saved.storage_boxes,
            &content.items,
            &content.blocks,
            &content.layers,
            &content.objects,
            &content.tools,
        )
        .unwrap_or_else(|error| panic!("smoke could not restore inactive storage: {error}"));
        inactive_dimensions.insert(
            saved.dimension_id,
            InactiveDimensionState::new(
                inactive_world,
                storage,
                saved.fluid_updates,
                saved.creatures,
            ),
        );
    }
    assert!(
        inactive_worlds.is_empty(),
        "smoke load returned dimension worlds absent from snapshot metadata"
    );

    let seed = WorldSeed(snapshot.seed);
    let dimension_id = snapshot.dimension_id.clone();
    let mut rules = GameRules::default();
    rules.set_ticks_per_second(snapshot.ticks_per_second);
    rules.set_spawn_creatures(snapshot.spawn_creatures);
    let mut save = InMemoryWorldSave::default();
    save.begin_new_world(seed, &dimension_id, rules);
    if let Some(player) = snapshot.player.as_ref() {
        let game_mode = if player.spectator {
            GameMode::Spectator
        } else if player.creative {
            GameMode::Creative
        } else {
            GameMode::Survival
        };
        save.save_player_state_with_health(
            LOCAL_PLAYER_ID,
            Vec3::from_array(player.position),
            game_mode,
            player.health,
            Some((player.yaw, player.pitch)),
            player.flying,
        );
    }
    let pending_creatures =
        PendingCreatureRestores::new(std::mem::take(&mut snapshot.creatures));
    let session = WorldSession::loaded(id.to_owned(), snapshot.day, snapshot.tick_in_day);

    let mut app = build_smoke_app(content);
    let runtime = app.world_mut();
    runtime.insert_resource(inventory);
    runtime.insert_resource(storage_boxes);
    runtime.insert_resource(save);
    runtime.insert_resource(session_lock);
    runtime.insert_resource(pending_fluids);
    runtime.insert_resource(pending_creatures);
    runtime.insert_resource(inactive_dimensions);
    runtime.insert_resource(seed);
    runtime.insert_resource(CurrentDimension {
        id: DimensionId::from(dimension_id),
    });
    runtime.insert_resource(rules);
    runtime.insert_resource(world);
    runtime.insert_resource(session);
    runtime.insert_resource(WorldLoadMode::Load);

    (app, expected_player_position)
}

fn save_registries(content: &LoadedContent) -> SaveRegistries<'_> {
    SaveRegistries {
        biomes: &content.biomes,
        blocks: &content.blocks,
        items: &content.items,
        layers: &content.layers,
        objects: &content.objects,
        fluids: &content.fluids,
        tools: &content.tools,
        creatures: &content.creatures,
        dimensions: &content.dimensions,
        cycles: &content.day_night_cycles,
    }
}

fn single_player_position(app: &mut App) -> Vec3 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<PlayerEntity>>();
    query
        .single(world)
        .expect("smoke gameplay must contain exactly one player")
        .translation
}

#[derive(Default)]
struct SmokeWorldCleanup {
    id: Option<String>,
}

impl SmokeWorldCleanup {
    fn remove_now(&mut self) {
        if let Some(id) = self.id.take() {
            delete_world(&id).unwrap_or_else(|error| {
                panic!("world rebuild smoke could not remove temporary world {id}: {error}")
            });
        }
    }
}

impl Drop for SmokeWorldCleanup {
    fn drop(&mut self) {
        let Some(id) = self.id.take() else {
            return;
        };
        let _ = delete_world(&id);
    }
}
