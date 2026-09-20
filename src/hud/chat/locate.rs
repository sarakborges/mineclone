use std::collections::HashSet;

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};

use crate::{
    content::{
        biome::{BiomeKind, BiomeRegistry},
        block::BlockRegistry,
        fluid::FluidRegistry,
        structure::StructureRegistry,
    },
    localization::ActiveLanguage,
    voxel::{
        chunk::CHUNK_SIZE,
        coordinates::chunk_coord_from_world,
    },
    world::{
        biome_field::BiomeField,
        current_context::CurrentDimensionContext,
        generation::{ChunkGenerationContext, located_structure_origins_in_chunk},
        terrain::surface_height,
        world_feature_fields::WorldFeatureFields,
    },
};

use super::{ChatMessage, ChatState};

const MAX_LOCATE_CHUNK_RADIUS: i32 = 2048;

#[derive(Clone, Copy)]
enum LocateTargetKind {
    SurfaceBiome,
    VolumeBiome,
    Structure,
}

struct LocateTaskResult {
    name: String,
    position: Option<IVec3>,
}

#[derive(Resource, Default)]
pub(super) struct PendingLocate {
    task: Option<Task<LocateTaskResult>>,
}

#[derive(Clone)]
struct LocateSnapshot {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    dimension: crate::content::dimension::DimensionDefinition,
    biomes: BiomeRegistry,
    structures: StructureRegistry,
    biome_field: BiomeField,
    feature_fields: WorldFeatureFields,
}

impl LocateSnapshot {
    fn generation_context(&self) -> ChunkGenerationContext<'_> {
        ChunkGenerationContext {
            blocks: &self.blocks,
            fluids: &self.fluids,
            dimension: &self.dimension,
            biomes: &self.biomes,
            structures: &self.structures,
            biome_field: &self.biome_field,
            feature_fields: &self.feature_fields,
        }
    }
}

#[derive(SystemParam)]
pub(super) struct ChatLocateContext<'w> {
    blocks: Res<'w, BlockRegistry>,
    fluids: Res<'w, FluidRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    structures: Res<'w, StructureRegistry>,
    biome_field: Res<'w, BiomeField>,
    feature_fields: Res<'w, WorldFeatureFields>,
    dimension: CurrentDimensionContext<'w>,
    language: Res<'w, ActiveLanguage>,
    pending: ResMut<'w, PendingLocate>,
 }

impl ChatLocateContext<'_> {
    pub(super) fn start(
        &mut self,
        target_kind: &str,
        id: &str,
        player_block: IVec3,
    ) -> String {
        let Some(dimension) = self.dimension.definition() else {
            return "Cannot locate: current dimension is unavailable.".to_owned();
        };

        let (kind, name) = match target_kind {
            "biome" => {
                let Some(biome) = self.biomes.get(id) else {
                    return format!("Unknown biome id: {id}");
                };
                if !dimension.biomes.iter().any(|entry| entry.id == id && entry.weight > 0.0) {
                    return format!("Biome is not active in this dimension: {id}");
                }
                let kind = match biome.kind {
                    BiomeKind::Surface => LocateTargetKind::SurfaceBiome,
                    BiomeKind::Volume => LocateTargetKind::VolumeBiome,
                    BiomeKind::Hydrology => {
                        return format!("Biome cannot be located by this command: {id}");
                    }
                };
                (kind, biome.name.text(self.language.get()).to_owned())
            }
            "structure" => {
                let Some(structure) = self.structures.get(id) else {
                    return format!("Unknown structure id: {id}");
                };
                if !structure.locatable {
                    return format!("Structure cannot be located by command: {id}");
                }
                let generated_here = dimension.biomes.iter().any(|dimension_biome| {
                    self.biomes
                        .get(&dimension_biome.id)
                        .is_some_and(|biome| biome.structures.iter().any(|entry| entry.id == id))
                });
                if !generated_here {
                    return format!("Structure is not generated in this dimension: {id}");
                }
                (
                    LocateTargetKind::Structure,
                    structure.name.text(self.language.get()).to_owned(),
                )
            }
            _ => return "Usage: /locate <biome|structure> <id>".to_owned(),
        };

        let snapshot = LocateSnapshot {
            blocks: self.blocks.as_ref().clone(),
            fluids: self.fluids.as_ref().clone(),
            dimension: dimension.clone(),
            biomes: self.biomes.as_ref().clone(),
            structures: self.structures.as_ref().clone(),
            biome_field: self.biome_field.as_ref().clone(),
            feature_fields: self.feature_fields.as_ref().clone(),
        };
        let id = id.to_owned();
        let response = format!("Locating {name}...");
        self.pending.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let position = locate_target(&snapshot, kind, &id, player_block);
            LocateTaskResult { name, position }
        }));

        response
    }
}

pub(super) fn poll_locate_task(
    mut pending: ResMut<PendingLocate>,
    mut chat: ResMut<ChatState>,
) {
    let Some(task) = pending.task.as_mut() else {
        return;
    };
    let Some(result) = check_ready(task) else {
        return;
    };
    pending.task = None;

    if let Some(position) = result.position {
        chat.append(ChatMessage::Located {
            prefix: format!(
                "{} found at X: {} Z: {} Y: {}. ",
                result.name, position.x, position.z, position.y
            ),
            target: position,
        });
    } else {
        chat.append(ChatMessage::Text(format!(
            "{} could not be found within {} chunks.",
            result.name, MAX_LOCATE_CHUNK_RADIUS
        )));
    }
}

fn locate_target(
    snapshot: &LocateSnapshot,
    kind: LocateTargetKind,
    id: &str,
    player: IVec3,
) -> Option<IVec3> {
    match kind {
        LocateTargetKind::SurfaceBiome => locate_surface_biome(snapshot, id, player),
        LocateTargetKind::VolumeBiome => locate_volume_biome(snapshot, id, player),
        LocateTargetKind::Structure => locate_structure(snapshot, id, player),
    }
}

fn locate_surface_biome(
    snapshot: &LocateSnapshot,
    id: &str,
    player: IVec3,
) -> Option<IVec3> {
    let center = chunk_coord_from_world(player).xz();
    let mut best: Option<(i64, IVec3)> = None;

    for radius in 0..=MAX_LOCATE_CHUNK_RADIUS {
        visit_square_chunk_ring(center, radius, |chunk| {
            let origin = chunk * CHUNK_SIZE as i32;
            for local_z in 0..CHUNK_SIZE as i32 {
                for local_x in 0..CHUNK_SIZE as i32 {
                    let horizontal = origin + IVec2::new(local_x, local_z);
                    let sample = snapshot
                        .biome_field
                        .sample_surface(horizontal.as_vec2() + Vec2::splat(0.5));
                    if sample.primary_id != id {
                        continue;
                    }

                    let y = surface_height(
                        horizontal,
                        &snapshot.dimension,
                        &snapshot.biomes,
                        &snapshot.biome_field,
                    );
                    let position = IVec3::new(horizontal.x, y, horizontal.y);
                    consider_nearest(&mut best, player, position);
                }
            }
        });

        if best_is_final(best, radius) {
            break;
        }
    }

    best.map(|(_, position)| position)
}

fn locate_volume_biome(
    snapshot: &LocateSnapshot,
    id: &str,
    player: IVec3,
) -> Option<IVec3> {
    let biome = snapshot.biomes.get(id)?;
    let range = biome.vertical_range?;
    let center = chunk_coord_from_world(player).xz();
    let mut seen = HashSet::new();
    let mut best: Option<(i64, IVec3)> = None;

    for radius in 0..=MAX_LOCATE_CHUNK_RADIUS {
        visit_square_chunk_ring(center, radius, |chunk| {
            let origin = chunk * CHUNK_SIZE as i32;
            let minimum = Vec3::new(origin.x as f32, range.min, origin.y as f32);
            let maximum = Vec3::new(
                (origin.x + CHUNK_SIZE as i32) as f32,
                range.max,
                (origin.y + CHUNK_SIZE as i32) as f32,
            );
            let region = snapshot
                .biome_field
                .volume_region_in_bounds(minimum, maximum);
            for anchor in snapshot.biome_field.volume_anchors_in_region(&region) {
                if anchor.id != id {
                    continue;
                }
                let position = anchor.position.floor().as_ivec3();
                if seen.insert(position) {
                    consider_nearest(&mut best, player, position);
                }
            }
        });

        if best_is_final(best, radius) {
            break;
        }
    }

    best.map(|(_, position)| position)
}

fn locate_structure(
    snapshot: &LocateSnapshot,
    id: &str,
    player: IVec3,
) -> Option<IVec3> {
    let center = chunk_coord_from_world(player).xz();
    let context = snapshot.generation_context();
    let mut seen = HashSet::new();
    let mut best: Option<(i64, IVec3)> = None;

    for radius in 0..=MAX_LOCATE_CHUNK_RADIUS {
        visit_square_chunk_ring(center, radius, |chunk| {
            for position in located_structure_origins_in_chunk(chunk, id, &context) {
                if seen.insert(position) {
                    consider_nearest(&mut best, player, position);
                }
            }
        });

        if best_is_final(best, radius) {
            break;
        }
    }

    best.map(|(_, position)| position)
}

fn consider_nearest(best: &mut Option<(i64, IVec3)>, player: IVec3, candidate: IVec3) {
    let dx = i64::from(candidate.x) - i64::from(player.x);
    let dy = i64::from(candidate.y) - i64::from(player.y);
    let dz = i64::from(candidate.z) - i64::from(player.z);
    let distance_squared = dx * dx + dy * dy + dz * dz;
    if best
        .as_ref()
        .is_none_or(|(best_distance, best_position)| {
            distance_squared < *best_distance
                || (distance_squared == *best_distance
                    && (candidate.z, candidate.x, candidate.y)
                        < (best_position.z, best_position.x, best_position.y))
        })
    {
        *best = Some((distance_squared, candidate));
    }
}

fn best_is_final(best: Option<(i64, IVec3)>, radius: i32) -> bool {
    let Some((distance_squared, _)) = best else {
        return false;
    };
    let minimum_future_horizontal =
        i64::from(radius) * i64::from(CHUNK_SIZE as i32);
    minimum_future_horizontal * minimum_future_horizontal > distance_squared
}

fn visit_square_chunk_ring(center: IVec2, radius: i32, mut visit: impl FnMut(IVec2)) {
    for z in -radius..=radius {
        for x in -radius..=radius {
            if radius > 0 && x.abs() != radius && z.abs() != radius {
                continue;
            }
            visit(center + IVec2::new(x, z));
        }
    }
}
