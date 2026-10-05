use std::panic::{AssertUnwindSafe, catch_unwind};

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};

use crate::{
    app::crash_log::log_gameplay_event,
    content::{
        biome::BiomeRegistry, structure::StructureRegistry, structure_set::StructureSetRegistry,
    },
    localization::ActiveLanguage,
    world::{current_context::CurrentDimensionContext, generator::WorldGenerator},
};

use super::{ChatMessage, ChatState};

const MAX_LOCATE_BLOCK_RADIUS: u32 = 32_768;

#[derive(Clone, Debug)]
enum LocateTarget {
    SurfaceBiome(String),
    Structure {
        reference: String,
        structure_id: Option<String>,
    },
}

enum LocateTaskOutcome {
    Found(IVec3),
    NotFound,
    Failed(String),
}

struct LocateTaskResult {
    name: String,
    outcome: LocateTaskOutcome,
}

#[derive(Resource, Default)]
pub(super) struct PendingLocate {
    task: Option<Task<LocateTaskResult>>,
}

impl PendingLocate {
    fn is_running(&self) -> bool {
        self.task.is_some()
    }
}

#[derive(SystemParam)]
pub(super) struct ChatLocateContext<'w> {
    generator: Res<'w, WorldGenerator>,
    biomes: Res<'w, BiomeRegistry>,
    structures: Res<'w, StructureRegistry>,
    structure_sets: Res<'w, StructureSetRegistry>,
    dimension: CurrentDimensionContext<'w>,
    language: Res<'w, ActiveLanguage>,
    pending: ResMut<'w, PendingLocate>,
}

impl ChatLocateContext<'_> {
    pub(super) fn start(
        &mut self,
        target_kind: &str,
        id: &str,
        variation: Option<usize>,
        player_block: IVec3,
    ) -> String {
        let Some(dimension) = self.dimension.definition() else {
            return "Cannot locate: current dimension is unavailable.".to_owned();
        };

        if self.pending.is_running() {
            log_gameplay_event(format!(
                "command.locate.rejected reason=in_progress kind={} id={} player={:?}",
                target_kind, id, player_block
            ));
            return "Cannot locate: another locate command is already in progress.".to_owned();
        }

        let (name, target) = match target_kind {
            "biome" => {
                let Some(biome) = self.biomes.get(id) else {
                    return format!("Unknown biome id: {id}");
                };
                if !biome.belongs_to_dimension(&dimension.id) || biome.surface_layout.is_none() {
                    return format!("Biome is not active in this dimension: {id}");
                }
                (
                    biome.name.text(self.language.get()).to_owned(),
                    LocateTarget::SurfaceBiome(id.to_owned()),
                )
            }
            "structure" => {
                if let Some(set) = self.structure_sets.get(id) {
                    if variation.is_some() {
                        return format!("Structure set {id} does not have variations.");
                    }
                    if !set.locatable {
                        return format!("Structure set cannot be located by command: {id}");
                    }
                    if !self.generator.structures().has_root_reference(id) {
                        return format!("Structure set is not generated in this dimension: {id}");
                    }
                    (
                        set.name.text(self.language.get()).to_owned(),
                        LocateTarget::Structure {
                            reference: id.to_owned(),
                            structure_id: None,
                        },
                    )
                } else {
                    let Some(variation_count) = self.structures.variation_count(id) else {
                        return format!(
                            "Unknown structure, structure group, or structure set: {id}"
                        );
                    };
                    if let Some(variation) = variation
                        && variation > variation_count
                    {
                        return format!(
                            "Unknown variation {variation} for {id}; expected 1..={variation_count}."
                        );
                    }

                    let structure = match variation {
                        Some(variation) => self
                            .structures
                            .variation(id, variation)
                            .expect("validated structure variation must resolve"),
                        None => self
                            .structures
                            .get(id)
                            .or_else(|| self.structures.variation(id, 1))
                            .expect("validated structure reference must resolve"),
                    };
                    if !structure.locatable {
                        return format!("Structure cannot be located by command: {id}");
                    }
                    if !self.generator.structures().has_root_reference(id) {
                        return format!("Structure is not generated in this dimension: {id}");
                    }

                    (
                        structure.id.clone(),
                        LocateTarget::Structure {
                            reference: id.to_owned(),
                            structure_id: variation.map(|_| structure.id.clone()),
                        },
                    )
                }
            }
            _ => {
                return "Usage: /locate biome <id> | /locate structure <id> [variation]".to_owned();
            }
        };

        log_gameplay_event(format!(
            "command.locate.start kind={} id={} variation={:?} player={:?}",
            target_kind, id, variation, player_block
        ));
        let response = format!("Locating {name}...");
        let generator = self.generator.as_ref().clone();
        self.pending.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let outcome = match catch_unwind(AssertUnwindSafe(|| {
                locate_target(&generator, &target, player_block)
            })) {
                Ok(Some(position)) => LocateTaskOutcome::Found(position),
                Ok(None) => LocateTaskOutcome::NotFound,
                Err(payload) => LocateTaskOutcome::Failed(panic_payload_message(&payload)),
            };
            LocateTaskResult { name, outcome }
        }));

        response
    }
}

pub(super) fn poll_locate_task(mut pending: ResMut<PendingLocate>, mut chat: ResMut<ChatState>) {
    let Some(task) = pending.task.as_mut() else {
        return;
    };
    let Some(result) = check_ready(task) else {
        return;
    };
    pending.task = None;

    match result.outcome {
        LocateTaskOutcome::Found(position) => {
            log_gameplay_event(format!(
                "command.locate.success name={} position={:?}",
                result.name, position
            ));
            chat.append(ChatMessage::Located {
                prefix: format!(
                    "{} found at X: {} Z: {} Y: {}. ",
                    result.name, position.x, position.z, position.y
                ),
                target: position,
            });
        }
        LocateTaskOutcome::NotFound => {
            log_gameplay_event(format!(
                "command.locate.failed name={} reason=not_found radius={}",
                result.name, MAX_LOCATE_BLOCK_RADIUS
            ));
            chat.append(ChatMessage::Text(format!(
                "{} could not be found within {} blocks.",
                result.name, MAX_LOCATE_BLOCK_RADIUS
            )));
        }
        LocateTaskOutcome::Failed(reason) => {
            log_gameplay_event(format!(
                "command.locate.failed name={} reason=world_query_error detail={:?}",
                result.name, reason
            ));
            chat.append(ChatMessage::Error(
                "Locate failed because the searched world region could not be resolved.".to_owned(),
            ));
        }
    }
}

fn locate_target(
    generator: &WorldGenerator,
    target: &LocateTarget,
    player: IVec3,
) -> Option<IVec3> {
    match target {
        LocateTarget::SurfaceBiome(id) => generator
            .biomes()
            .find_surface_biome(id, player.x, player.z, MAX_LOCATE_BLOCK_RADIUS)
            .map(|result| {
                let (x, z) = result.position();
                IVec3::new(x, generator.terrain().surface_at(x, z), z)
            }),
        LocateTarget::Structure {
            reference,
            structure_id,
        } => {
            let queries = generator.structures();
            let placement = match structure_id.as_deref() {
                Some(structure_id) => queries.find_nearest_variant(
                    reference,
                    structure_id,
                    player.x,
                    player.z,
                    MAX_LOCATE_BLOCK_RADIUS,
                ),
                None => queries.find_nearest(
                    reference,
                    player.x,
                    player.z,
                    MAX_LOCATE_BLOCK_RADIUS,
                ),
            }?;
            Some(placement.origin())
        }
    }
}

fn panic_payload_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    "non-string panic payload".to_owned()
}
