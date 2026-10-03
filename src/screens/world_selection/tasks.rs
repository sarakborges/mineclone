use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use crate::{
    app::crash_log::{log_system_error, log_system_event, log_system_warn},
    content::{
        biome::BiomeRegistry, block::BlockRegistry, creature::CreatureRegistry,
        day_night_cycle::DayNightCycleRegistry, dimension::DimensionRegistry,
        fluid::FluidRegistry, item::ItemRegistry, layer::LayerRegistry,
        object::ObjectRegistry, tool::ToolRegistry,
    },
    voxel::world::VoxelWorld,
    world::save_catalog::{
        SaveRegistries, WorldDirectoryLock, WorldSnapshot, WorldSummary, list_verified_worlds,
        load_world,
    },
};

type WorldScanResult = Arc<Mutex<Option<io::Result<Vec<WorldSummary>>>>>;
type WorldLoadResult = Arc<Mutex<WorldLoadSlot>>;
type LoadedWorld = (
    WorldSnapshot,
    VoxelWorld,
    HashMap<String, VoxelWorld>,
    WorldDirectoryLock,
);

#[derive(Default)]
struct WorldLoadSlot {
    abandoned: bool,
    complete: bool,
    result: Option<io::Result<LoadedWorld>>,
}

pub(super) struct PendingWorldScan {
    result: WorldScanResult,
}

impl PendingWorldScan {
    pub(super) fn start(registries: SaveRegistries<'_>) -> io::Result<Self> {
        let copy_started = Instant::now();
        let owned = registries.owned_for_pruning();
        log_system_event(format!(
            "world_catalog.scan definitions_copied duration_ms={:.2}",
            copy_started.elapsed().as_secs_f64() * 1_000.0,
        ));

        let result: WorldScanResult = Arc::new(Mutex::new(None));
        let worker_result = Arc::clone(&result);
        thread::Builder::new()
            .name("asteria-world-scan".to_owned())
            .spawn(move || {
                let scan_started = Instant::now();
                let verified = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    list_verified_worlds(&owned)
                }))
                .unwrap_or_else(|_| {
                    Err(io::Error::other("saved-world verification worker panicked"))
                });
                let duration_ms = scan_started.elapsed().as_secs_f64() * 1_000.0;
                match &verified {
                    Ok(worlds) => log_system_event(format!(
                        "world_catalog.scan success worlds={} duration_ms={duration_ms:.2}",
                        worlds.len()
                    )),
                    Err(error) => log_system_error(format!(
                        "world_catalog.scan failed duration_ms={duration_ms:.2} error={error}"
                    )),
                }
                if let Ok(mut slot) = worker_result.lock() {
                    *slot = Some(verified);
                } else {
                    log_system_error("world_catalog.scan result_publish_failed reason=poisoned_mutex");
                }
            })?;

        Ok(Self { result })
    }

    pub(super) fn poll(&self) -> Option<io::Result<Vec<WorldSummary>>> {
        self.result
            .try_lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }
}

struct OwnedLoadContent {
    biomes: BiomeRegistry,
    blocks: BlockRegistry,
    items: ItemRegistry,
    layers: LayerRegistry,
    objects: ObjectRegistry,
    fluids: FluidRegistry,
    tools: ToolRegistry,
    creatures: CreatureRegistry,
    dimensions: DimensionRegistry,
    cycles: DayNightCycleRegistry,
}

impl OwnedLoadContent {
    fn capture(registries: SaveRegistries<'_>) -> Self {
        let mut items = ItemRegistry::default();
        for definition in registries.items.iter() {
            items.insert(definition.clone());
        }
        let mut objects = ObjectRegistry::default();
        for definition in registries.objects.iter() {
            objects.insert(definition.clone());
        }
        let mut tools = ToolRegistry::default();
        for definition in registries.tools.iter() {
            tools.insert(definition.clone());
        }
        let mut creatures = CreatureRegistry::default();
        for definition in registries.creatures.iter() {
            creatures.insert(definition.clone());
        }
        let mut dimensions = DimensionRegistry::default();
        let mut cycles = DayNightCycleRegistry::default();
        for definition in registries.dimensions.iter() {
            if let Some(cycle) = registries.cycles.get(&definition.day_night_cycle) {
                cycles.insert(cycle.clone());
            }
            dimensions.insert(definition.clone());
        }

        Self {
            biomes: registries.biomes.clone(),
            blocks: registries.blocks.clone(),
            items,
            layers: registries.layers.clone(),
            objects,
            fluids: registries.fluids.clone(),
            tools,
            creatures,
            dimensions,
            cycles,
        }
    }

    fn registries(&self) -> SaveRegistries<'_> {
        SaveRegistries {
            biomes: &self.biomes,
            blocks: &self.blocks,
            items: &self.items,
            layers: &self.layers,
            objects: &self.objects,
            fluids: &self.fluids,
            tools: &self.tools,
            creatures: &self.creatures,
            dimensions: &self.dimensions,
            cycles: &self.cycles,
        }
    }
}

pub(super) struct WorldLoadCompletion {
    pub(super) abandoned: bool,
    pub(super) result: Option<io::Result<LoadedWorld>>,
}

pub(super) struct PendingWorldLoad {
    id: String,
    result: WorldLoadResult,
}

impl PendingWorldLoad {
    pub(super) fn start(id: String, registries: SaveRegistries<'_>) -> io::Result<Self> {
        let copy_started = Instant::now();
        let owned = OwnedLoadContent::capture(registries);
        log_system_event(format!(
            "world.load definitions_copied id={} duration_ms={:.2}",
            id,
            copy_started.elapsed().as_secs_f64() * 1_000.0,
        ));

        let result: WorldLoadResult = Arc::new(Mutex::new(WorldLoadSlot::default()));
        let worker_result = Arc::clone(&result);
        let worker_id = id.clone();
        thread::Builder::new()
            .name("asteria-world-load".to_owned())
            .spawn(move || {
                let load_started = Instant::now();
                let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    load_world(&worker_id, owned.registries())
                }))
                .unwrap_or_else(|_| Err(io::Error::other("saved-world loading worker panicked")));
                let duration_ms = load_started.elapsed().as_secs_f64() * 1_000.0;
                match &loaded {
                    Ok(_) => log_system_event(format!(
                        "world.load success id={worker_id} duration_ms={duration_ms:.2}"
                    )),
                    Err(error) => log_system_error(format!(
                        "world.load failed id={worker_id} duration_ms={duration_ms:.2} error={error}"
                    )),
                }
                let mut loaded = Some(loaded);
                {
                    let mut slot = worker_result
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    slot.complete = true;
                    if !slot.abandoned {
                        slot.result = loaded.take();
                    } else {
                        log_system_event(format!(
                            "world.load discarded id={worker_id} reason=abandoned"
                        ));
                    }
                }
                drop(loaded);
            })?;

        Ok(Self { id, result })
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }

    pub(super) fn poll(&self) -> Option<WorldLoadCompletion> {
        let mut slot = self.result.try_lock().ok()?;
        if !slot.complete {
            return None;
        }
        Some(WorldLoadCompletion {
            abandoned: slot.abandoned,
            result: slot.result.take(),
        })
    }

    pub(super) fn abandon(&self) {
        log_system_event(format!("world.load cancel_requested id={}", self.id));
        let stale = {
            let mut slot = self
                .result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            slot.abandoned = true;
            slot.result.take()
        };
        if let Some(stale) = stale
            && let Err(error) = thread::Builder::new()
                .name("asteria-discard-world".to_owned())
                .spawn(move || drop(stale))
        {
            log_system_warn(format!(
                "world.load abandoned_cleanup_worker_failed id={} error={error}",
                self.id
            ));
        }
    }
}
