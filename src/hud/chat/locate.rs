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
        generation::{
            ChunkGenerationContext, located_structure_origins_in_chunk,
            structure_candidate_anchor,
        },
        generation_region::generation_region_coord,
        hydrology::HydrologyWaterKind,
        terrain::surface_height,
        world_feature_fields::WorldFeatureFields,
    },
};

use super::{ChatMessage, ChatState};

const MAX_LOCATE_BLOCK_RADIUS: i32 = 32_768;
const MAX_LOCATE_CHUNK_RADIUS: i32 = MAX_LOCATE_BLOCK_RADIUS / CHUNK_SIZE as i32;

#[derive(Clone, Copy)]
enum LocateTargetKind {
    SurfaceBiome,
    VolumeBiome,
    Hydrology(HydrologyWaterKind),
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
                        return format!("Use /locate hydrology for hydrology targets: {id}");
                    }
                };
                (kind, biome.name.text(self.language.get()).to_owned())
            }
            "hydrology" => {
                let (kind, name, enabled) = match id {
                    "ocean" => (
                        HydrologyWaterKind::Ocean,
                        "Ocean",
                        dimension.hydrology.ocean_biome.is_some(),
                    ),
                    "river" => (
                        HydrologyWaterKind::River,
                        "River",
                        dimension.hydrology.river_weight > 0.0,
                    ),
                    "lake" => (
                        HydrologyWaterKind::Lake,
                        "Lake",
                        dimension.hydrology.lake_weight > 0.0,
                    ),
                    _ => return "Usage: /locate hydrology <ocean|river|lake>".to_owned(),
                };
                if !enabled {
                    return format!("{name} hydrology is disabled in this dimension.");
                }
                (LocateTargetKind::Hydrology(kind), name.to_owned())
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
                        .is_some_and(|biome| {
                            biome.structures.iter().any(|entry| {
                                self.structures
                                    .reference_contains_structure(&entry.id, id)
                            })
                        })
                });
                if !generated_here {
                    return format!("Structure is not generated in this dimension: {id}");
                }
                (
                    LocateTargetKind::Structure,
                    structure.name.text(self.language.get()).to_owned(),
                )
            }
            _ => return "Usage: /locate <biome|hydrology|structure> <id>".to_owned(),
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
            "{} could not be found within {} blocks.",
            result.name, MAX_LOCATE_BLOCK_RADIUS
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
        LocateTargetKind::Hydrology(kind) => locate_hydrology(snapshot, kind, player),
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
            let Some(origin) = chunk_block_origin(chunk) else {
                return;
            };
            for local_z in 0..CHUNK_SIZE as i32 {
                for local_x in 0..CHUNK_SIZE as i32 {
                    let (Some(x), Some(z)) = (
                        origin.x.checked_add(local_x),
                        origin.y.checked_add(local_z),
                    ) else {
                        continue;
                    };
                    let horizontal = IVec2::new(x, z);
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
            let Some(origin) = chunk_block_origin(chunk) else {
                return;
            };
            let minimum = Vec3::new(origin.x as f32, range.min, origin.y as f32);
            let maximum = Vec3::new(
                (i64::from(origin.x) + CHUNK_SIZE as i64) as f32,
                range.max,
                (i64::from(origin.y) + CHUNK_SIZE as i64) as f32,
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

fn locate_hydrology(
    snapshot: &LocateSnapshot,
    kind: HydrologyWaterKind,
    player: IVec3,
) -> Option<IVec3> {
    let center = chunk_coord_from_world(player).xz();
    let context = snapshot.generation_context();
    let mut best: Option<(i64, IVec3)> = None;

    for radius in 0..=MAX_LOCATE_CHUNK_RADIUS {
        visit_square_chunk_ring(center, radius, |chunk| {
            let Some(origin) = chunk_block_origin(chunk) else {
                return;
            };
            let region_coord = generation_region_coord(IVec3::new(chunk.x, 0, chunk.y));
            let region = context.region(region_coord);

            for local_z in 0..CHUNK_SIZE as i32 {
                for local_x in 0..CHUNK_SIZE as i32 {
                    let (Some(x), Some(z)) = (
                        origin.x.checked_add(local_x),
                        origin.y.checked_add(local_z),
                    ) else {
                        continue;
                    };
                    let horizontal = IVec2::new(x, z);
                    let surface_y = surface_height(
                        horizontal,
                        &snapshot.dimension,
                        &snapshot.biomes,
                        &snapshot.biome_field,
                    );
                    let sample_position = horizontal.as_vec2() + Vec2::splat(0.5);
                    let Some(water) =
                        region.hydrology.supported_water_at(sample_position, surface_y as f32)
                    else {
                        continue;
                    };
                    if water.kind != kind {
                        continue;
                    }

                    let y = water.water_level.floor() as i32 + 1;
                    consider_nearest(&mut best, player, IVec3::new(x, y, z));
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
    let structure = snapshot.structures.get(id)?;
    let probe_offset = *structure.horizontal_footprint().first()?;
    let context = snapshot.generation_context();
    let player_horizontal = player.xz();
    let maximum_distance_squared =
        i64::from(MAX_LOCATE_BLOCK_RADIUS) * i64::from(MAX_LOCATE_BLOCK_RADIUS);
    let mut seen = HashSet::new();
    let mut best: Option<(i64, IVec3)> = None;

    for biome_structure in snapshot.biomes.structure_placements() {
        if !snapshot
            .structures
            .reference_contains_structure(&biome_structure.structure_id, id)
            || !snapshot
                .dimension
                .biomes
                .iter()
                .any(|entry| entry.id == biome_structure.biome_id && entry.weight > 0.0)
        {
            continue;
        }

        let placement = biome_structure.placement;
        let spacing = placement.spacing;
        let center_cell = IVec2::new(
            player_horizontal.x.div_euclid(spacing),
            player_horizontal.y.div_euclid(spacing),
        );
        let maximum_cell_radius =
            (MAX_LOCATE_BLOCK_RADIUS + spacing - 1) / spacing + 2;

        for radius in 0..=maximum_cell_radius {
            visit_square_cell_ring(center_cell, radius, |cell| {
                let Some(anchor) = structure_candidate_anchor(
                    snapshot.biome_field.seed(),
                    &biome_structure.biome_id,
                    &biome_structure.structure_id,
                    placement,
                    cell,
                ) else {
                    return;
                };

                let dx = i64::from(anchor.x) - i64::from(player_horizontal.x);
                let dz = i64::from(anchor.y) - i64::from(player_horizontal.y);
                if dx * dx + dz * dz > maximum_distance_squared {
                    return;
                }

                let (Some(probe_x), Some(probe_z)) = (
                    anchor.x.checked_add(probe_offset.x),
                    anchor.y.checked_add(probe_offset.y),
                ) else {
                    return;
                };
                let probe_chunk =
                    chunk_coord_from_world(IVec3::new(probe_x, 0, probe_z)).xz();
                for position in located_structure_origins_in_chunk(probe_chunk, id, &context) {
                    if position.xz() == anchor && seen.insert(position) {
                        consider_nearest(&mut best, player, position);
                    }
                }
            });

            if structure_search_is_final(best, radius, spacing, placement.jitter) {
                break;
            }
        }
    }

    best.map(|(_, position)| position)
}

fn structure_search_is_final(
    best: Option<(i64, IVec3)>,
    visited_cell_radius: i32,
    spacing: i32,
    jitter: i32,
) -> bool {
    let Some((distance_squared, _)) = best else {
        return false;
    };
    let next_radius = i64::from(visited_cell_radius) + 1;
    let spacing = i64::from(spacing);
    let jitter = i64::from(jitter);
    let minimum_future_horizontal =
        (next_radius * spacing - spacing / 2 - jitter).max(0);
    minimum_future_horizontal * minimum_future_horizontal > distance_squared
}

fn consider_nearest(best: &mut Option<(i64, IVec3)>, player: IVec3, candidate: IVec3) {
    let dx = i64::from(candidate.x) - i64::from(player.x);
    let dy = i64::from(candidate.y) - i64::from(player.y);
    let dz = i64::from(candidate.z) - i64::from(player.z);
    let distance_squared = dx
        .saturating_mul(dx)
        .saturating_add(dy.saturating_mul(dy))
        .saturating_add(dz.saturating_mul(dz));
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

fn visit_square_cell_ring(center: IVec2, radius: i32, mut visit: impl FnMut(IVec2)) {
    visit_square_ring(center, radius, &mut visit);
}

fn visit_square_chunk_ring(center: IVec2, radius: i32, mut visit: impl FnMut(IVec2)) {
    visit_square_ring(center, radius, &mut visit);
}

fn visit_square_ring(center: IVec2, radius: i32, visit: &mut impl FnMut(IVec2)) {
    for z in -radius..=radius {
        for x in -radius..=radius {
            if radius > 0 && x.abs() != radius && z.abs() != radius {
                continue;
            }
            let (Some(candidate_x), Some(candidate_z)) =
                (center.x.checked_add(x), center.y.checked_add(z))
            else {
                continue;
            };
            visit(IVec2::new(candidate_x, candidate_z));
        }
    }
}

fn chunk_block_origin(chunk: IVec2) -> Option<IVec2> {
    let size = CHUNK_SIZE as i32;
    Some(IVec2::new(
        chunk.x.checked_mul(size)?,
        chunk.y.checked_mul(size)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_cell_search_stops_once_future_cells_cannot_beat_best() {
        let best = Some((900_i64 * 900_i64, IVec3::ZERO));

        assert!(!structure_search_is_final(best, 0, 1_000, 112));
        assert!(structure_search_is_final(best, 1, 1_000, 112));
    }

    #[test]
    fn locate_radius_keeps_existing_thirty_two_kiloblock_contract() {
        assert_eq!(MAX_LOCATE_BLOCK_RADIUS, 32_768);
        assert_eq!(MAX_LOCATE_CHUNK_RADIUS, 2_048);
    }

    #[test]
    fn square_ring_skips_candidates_outside_i32_domain() {
        let mut visited = Vec::new();
        visit_square_ring(IVec2::new(i32::MAX, 0), 1, &mut |candidate| {
            visited.push(candidate);
        });

        assert!(visited.iter().any(|candidate| candidate.x == i32::MAX));
        assert!(!visited.iter().any(|candidate| candidate.x == i32::MIN));
    }

    #[test]
    fn chunk_block_origin_rejects_unrepresentable_world_coordinates() {
        assert_eq!(chunk_block_origin(IVec2::ZERO), Some(IVec2::ZERO));
        assert!(chunk_block_origin(IVec2::new(i32::MAX, 0)).is_none());
    }
}
