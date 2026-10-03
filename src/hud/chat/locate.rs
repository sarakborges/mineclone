use std::{
    collections::HashSet,
    panic::{AssertUnwindSafe, catch_unwind},
};

use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};

use crate::{
    app::crash_log::log_gameplay_event,
    content::{
        biome::{BiomeKind, BiomeRegistry, BiomeVerticalRange},
        biome_structure::BiomeStructurePlacementRules,
        block::BlockRegistry,
        dimension::DimensionDefinition,
        fluid::FluidRegistry,
        structure::StructureRegistry,
        structure_set::StructureSetRegistry,
    },
    localization::ActiveLanguage,
    voxel::coordinates::chunk_coord_from_world,
    world::{
        WorldGenerationMode, WorldGenerationSettings,
        biome_field::BiomeField,
        current_context::CurrentDimensionContext,
        generation::{
            ChunkGenerationContext, generation_surface_height, located_structure_origins_in_chunk,
            structure_candidate_anchor, structure_candidate_probe,
            volume_structure_candidate_probe,
        },
        world_feature_fields::WorldFeatureFields,
    },
};

use super::{ChatMessage, ChatState};

const MAX_LOCATE_BLOCK_RADIUS: i32 = 32_768;
const VOLUME_SEARCH_TILE_SIZE: i32 = 512;

#[derive(Clone, Copy)]
enum LocateTargetKind {
    SurfaceBiome,
    VolumeBiome,
    Structure,
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

#[derive(Clone)]
struct LocateSnapshot {
    blocks: BlockRegistry,
    fluids: FluidRegistry,
    dimension: DimensionDefinition,
    biomes: BiomeRegistry,
    structures: StructureRegistry,
    structure_sets: StructureSetRegistry,
    biome_field: BiomeField,
    feature_fields: WorldFeatureFields,
    world_generation: WorldGenerationSettings,
}

impl LocateSnapshot {
    fn generation_context(&self) -> ChunkGenerationContext<'_> {
        ChunkGenerationContext {
            blocks: &self.blocks,
            fluids: &self.fluids,
            dimension: &self.dimension,
            biomes: &self.biomes,
            structures: &self.structures,
            structure_sets: &self.structure_sets,
            world_generation: self.world_generation,
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
    structure_sets: Res<'w, StructureSetRegistry>,
    biome_field: Res<'w, BiomeField>,
    feature_fields: Res<'w, WorldFeatureFields>,
    world_generation: Res<'w, WorldGenerationSettings>,
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

        let (kind, name, search_id) = match target_kind {
            "biome" => {
                let Some(biome) = self.biomes.get(id) else {
                    return format!("Unknown biome id: {id}");
                };
                if !biome_is_generated_in_dimension(id, dimension, *self.world_generation) {
                    return format!("Biome is not active in this dimension: {id}");
                }
                let kind = match biome.kind {
                    BiomeKind::Surface => LocateTargetKind::SurfaceBiome,
                    BiomeKind::Volume => LocateTargetKind::VolumeBiome,
                };
                (
                    kind,
                    biome.name.text(self.language.get()).to_owned(),
                    id.to_owned(),
                )
            }
            "structure" => {
                if !self.world_generation.spawn_structures()
                    || self.world_generation.mode() == WorldGenerationMode::Void
                {
                    return format!("Structure is not generated in this dimension: {id}");
                }

                if let Some(set) = self.structure_sets.get(id) {
                    if variation.is_some() {
                        return format!("Structure set {id} does not have variations.");
                    }
                    if !set.locatable {
                        return format!("Structure set cannot be located by command: {id}");
                    }
                    if !structure_reference_is_generated(
                        id,
                        dimension,
                        &self.biomes,
                        &self.structures,
                        &self.structure_sets,
                        &self.feature_fields,
                    ) {
                        return format!("Structure set is not generated in this dimension: {id}");
                    }
                    (
                        LocateTargetKind::Structure,
                        set.name.text(self.language.get()).to_owned(),
                        id.to_owned(),
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

                    let search_id = variation
                        .map(|_| structure.id.clone())
                        .unwrap_or_else(|| id.to_owned());
                    if !structure_reference_is_generated(
                        &search_id,
                        dimension,
                        &self.biomes,
                        &self.structures,
                        &self.structure_sets,
                        &self.feature_fields,
                    ) {
                        return format!("Structure is not generated in this dimension: {id}");
                    }

                    (
                        LocateTargetKind::Structure,
                        structure.name.text(self.language.get()).to_owned(),
                        search_id,
                    )
                }
            }
            _ => {
                return "Usage: /locate biome <id> | /locate structure <id> [variation]".to_owned();
            }
        };

        let snapshot = LocateSnapshot {
            blocks: self.blocks.as_ref().clone(),
            fluids: self.fluids.as_ref().clone(),
            dimension: dimension.clone(),
            biomes: self.biomes.as_ref().clone(),
            structures: self.structures.as_ref().clone(),
            structure_sets: self.structure_sets.as_ref().clone(),
            biome_field: self.biome_field.as_ref().clone(),
            // Locate may resolve a large amount of deterministic structure
            // metadata. Keep those disposable caches out of the live world.
            feature_fields: self.feature_fields.clone_with_fresh_caches(),
            world_generation: *self.world_generation,
        };
        log_gameplay_event(format!(
            "command.locate.start kind={} id={} variation={:?} player={:?}",
            target_kind, search_id, variation, player_block
        ));
        let response = format!("Locating {name}...");
        self.pending.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let outcome = match catch_unwind(AssertUnwindSafe(|| {
                locate_target(&snapshot, kind, &search_id, player_block)
            })) {
                Ok(Some(position)) => LocateTaskOutcome::Found(position),
                Ok(None) => LocateTaskOutcome::NotFound,
                Err(payload) => LocateTaskOutcome::Failed(panic_payload_message(payload)),
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

fn locate_surface_biome(snapshot: &LocateSnapshot, id: &str, player: IVec3) -> Option<IVec3> {
    let context = snapshot.generation_context();
    let player_column = player.xz();
    if snapshot
        .biome_field
        .sample_surface(player_column.as_vec2() + Vec2::splat(0.5))
        .primary_id
        == id
    {
        return Some(IVec3::new(
            player_column.x,
            generation_surface_height(player_column, &context),
            player_column.y,
        ));
    }

    let spacing = snapshot.biome_field.surface_site_spacing();
    let center = IVec2::new(
        (player.x as f32 / spacing.x).floor() as i32,
        (player.z as f32 / spacing.y).floor() as i32,
    );
    let maximum_cell_radius =
        (MAX_LOCATE_BLOCK_RADIUS as f32 / spacing.min_element()).ceil() as i32 + 3;
    let mut best: Option<(i64, IVec3)> = None;
    let mut seen_columns = HashSet::new();

    for radius in 0..=maximum_cell_radius {
        visit_square_ring(center, radius, |cell| {
            let site = snapshot.biome_field.surface_site_position(cell);
            let column = site.floor().as_ivec2();
            if !seen_columns.insert(column)
                || !within_horizontal_radius(player_column, column, MAX_LOCATE_BLOCK_RADIUS)
            {
                return;
            }

            let sample = snapshot.biome_field.sample_surface(site);
            if sample.primary_id != id {
                return;
            }

            let position = IVec3::new(
                column.x,
                generation_surface_height(column, &context),
                column.y,
            );
            consider_nearest(&mut best, player, position);
        });

        if lattice_search_is_final(best, radius, spacing.min_element().floor() as i32, 2) {
            break;
        }
    }

    best.map(|(_, position)| position)
}

fn locate_volume_biome(snapshot: &LocateSnapshot, id: &str, player: IVec3) -> Option<IVec3> {
    let biome = snapshot.biomes.get(id)?;
    let range = biome.vertical_range?;
    let center = IVec2::new(
        player.x.div_euclid(VOLUME_SEARCH_TILE_SIZE),
        player.z.div_euclid(VOLUME_SEARCH_TILE_SIZE),
    );
    let maximum_tile_radius = MAX_LOCATE_BLOCK_RADIUS.div_euclid(VOLUME_SEARCH_TILE_SIZE) + 2;
    let mut seen = HashSet::new();
    let mut best: Option<(i64, IVec3)> = None;

    for radius in 0..=maximum_tile_radius {
        visit_square_ring(center, radius, |tile| {
            let Some((minimum, maximum)) = volume_tile_bounds(tile, range) else {
                return;
            };
            let region = snapshot
                .biome_field
                .volume_region_in_bounds(minimum, maximum);

            for anchor in snapshot.biome_field.volume_anchors_in_region(&region) {
                if anchor.id != id {
                    continue;
                }
                let position = anchor.position.floor().as_ivec3();
                if !seen.insert(position)
                    || !within_horizontal_radius(
                        player.xz(),
                        position.xz(),
                        MAX_LOCATE_BLOCK_RADIUS,
                    )
                {
                    continue;
                }

                let surface = snapshot.biome_field.sample_surface(anchor.position.xz());
                let Some(selection) = snapshot.biome_field.volume_selection_in_region_for_surface(
                    anchor.position,
                    &region,
                    surface.identity_surface_index,
                ) else {
                    continue;
                };
                if snapshot.biome_field.volume_biome_id(selection) != id {
                    continue;
                }

                consider_nearest(&mut best, player, position);
            }
        });

        if lattice_search_is_final(best, radius, VOLUME_SEARCH_TILE_SIZE, 2) {
            break;
        }
    }

    best.map(|(_, position)| position)
}

fn locate_structure(snapshot: &LocateSnapshot, id: &str, player: IVec3) -> Option<IVec3> {
    let context = snapshot.generation_context();
    let mut best: Option<(i64, IVec3)> = None;
    let mut seen = HashSet::new();

    locate_surface_structures(snapshot, &context, id, player, &mut seen, &mut best);
    locate_volume_structures(snapshot, &context, id, player, &mut seen, &mut best);

    best.map(|(_, position)| position)
}

fn locate_surface_structures(
    snapshot: &LocateSnapshot,
    context: &ChunkGenerationContext<'_>,
    id: &str,
    player: IVec3,
    seen: &mut HashSet<IVec3>,
    best: &mut Option<(i64, IVec3)>,
) {
    let player_horizontal = player.xz();

    for entry in snapshot
        .feature_fields
        .structure_metadata()
        .surface_entries()
    {
        if !dimension_has_active_biome(&snapshot.dimension, &entry.biome_id)
            || !structure_reference_matches(
                &entry.reference,
                id,
                &snapshot.structures,
                &snapshot.structure_sets,
            )
        {
            continue;
        }

        let placement = entry.placement;
        let spacing = placement.spacing;
        let center = IVec2::new(
            player_horizontal.x.div_euclid(spacing),
            player_horizontal.y.div_euclid(spacing),
        );
        let maximum_cell_radius = (MAX_LOCATE_BLOCK_RADIUS + spacing - 1).div_euclid(spacing) + 2;

        for radius in 0..=maximum_cell_radius {
            visit_square_ring(center, radius, |cell| {
                let Some(anchor) = structure_candidate_anchor(
                    snapshot.biome_field.seed(),
                    &entry.biome_id,
                    &entry.reference,
                    placement,
                    cell,
                ) else {
                    return;
                };
                if !within_horizontal_radius(player_horizontal, anchor, MAX_LOCATE_BLOCK_RADIUS) {
                    return;
                }

                let Some(probe) = structure_candidate_probe(
                    &entry.biome_id,
                    &entry.reference,
                    id,
                    anchor,
                    context,
                ) else {
                    return;
                };
                let probe_chunk = chunk_coord_from_world(IVec3::new(probe.x, 0, probe.y)).xz();
                for position in
                    located_structure_origins_in_chunk(probe_chunk, id, anchor, 0, context)
                {
                    if seen.insert(position)
                        && within_horizontal_radius(
                            player_horizontal,
                            position.xz(),
                            MAX_LOCATE_BLOCK_RADIUS,
                        )
                    {
                        consider_nearest(best, player, position);
                    }
                }
            });

            if structure_search_is_final(*best, radius, spacing, placement.jitter) {
                break;
            }
        }
    }
}

fn locate_volume_structures(
    snapshot: &LocateSnapshot,
    context: &ChunkGenerationContext<'_>,
    id: &str,
    player: IVec3,
    seen: &mut HashSet<IVec3>,
    best: &mut Option<(i64, IVec3)>,
) {
    let player_horizontal = player.xz();

    for biome_structure in snapshot.biomes.structure_placements() {
        let BiomeStructurePlacementRules::Volume(placement) = biome_structure.placement else {
            continue;
        };
        if !dimension_has_active_biome(&snapshot.dimension, &biome_structure.biome_id)
            || !structure_reference_matches(
                &biome_structure.structure_id,
                id,
                &snapshot.structures,
                &snapshot.structure_sets,
            )
        {
            continue;
        }
        let Some(biome) = snapshot.biomes.get(&biome_structure.biome_id) else {
            continue;
        };
        let Some(range) = biome.vertical_range else {
            continue;
        };

        let center = IVec2::new(
            player.x.div_euclid(VOLUME_SEARCH_TILE_SIZE),
            player.z.div_euclid(VOLUME_SEARCH_TILE_SIZE),
        );
        let maximum_tile_radius = MAX_LOCATE_BLOCK_RADIUS.div_euclid(VOLUME_SEARCH_TILE_SIZE) + 2;
        let mut visited_anchors = HashSet::new();

        for radius in 0..=maximum_tile_radius {
            visit_square_ring(center, radius, |tile| {
                let Some((minimum, maximum)) = volume_tile_bounds(tile, range) else {
                    return;
                };
                let region = snapshot
                    .biome_field
                    .volume_region_in_bounds(minimum, maximum);

                for volume_anchor in snapshot.biome_field.volume_anchors_in_region(&region) {
                    if volume_anchor.id != biome_structure.biome_id {
                        continue;
                    }
                    let anchor = volume_anchor.position.floor().as_ivec3();
                    if !visited_anchors.insert(anchor)
                        || !within_horizontal_radius(
                            player_horizontal,
                            anchor.xz(),
                            MAX_LOCATE_BLOCK_RADIUS,
                        )
                    {
                        continue;
                    }

                    let Some(selection) = snapshot
                        .biome_field
                        .volume_selection_in_region(volume_anchor.position, &region)
                    else {
                        continue;
                    };
                    if snapshot.biome_field.volume_biome_id(selection) != biome_structure.biome_id {
                        continue;
                    }

                    let Some(probe) = volume_structure_candidate_probe(
                        &biome_structure.biome_id,
                        &biome_structure.structure_id,
                        id,
                        anchor,
                        placement,
                        context,
                    ) else {
                        continue;
                    };
                    let probe_chunk = chunk_coord_from_world(IVec3::new(probe.x, 0, probe.y)).xz();
                    for position in located_structure_origins_in_chunk(
                        probe_chunk,
                        id,
                        anchor.xz(),
                        anchor.y,
                        context,
                    ) {
                        if seen.insert(position)
                            && within_horizontal_radius(
                                player_horizontal,
                                position.xz(),
                                MAX_LOCATE_BLOCK_RADIUS,
                            )
                        {
                            consider_nearest(best, player, position);
                        }
                    }
                }
            });

            if lattice_search_is_final(*best, radius, VOLUME_SEARCH_TILE_SIZE, 2) {
                break;
            }
        }
    }
}

fn biome_is_generated_in_dimension(
    id: &str,
    dimension: &DimensionDefinition,
    world_generation: WorldGenerationSettings,
) -> bool {
    if world_generation.mode() == WorldGenerationMode::Void {
        return false;
    }
    if !dimension_has_active_biome(dimension, id) {
        return false;
    }
    if !world_generation.spawn_oceans() && dimension.ocean_biome.as_deref() == Some(id) {
        return false;
    }
    true
}

fn structure_reference_is_generated(
    target: &str,
    dimension: &DimensionDefinition,
    biomes: &BiomeRegistry,
    structures: &StructureRegistry,
    structure_sets: &StructureSetRegistry,
    feature_fields: &WorldFeatureFields,
) -> bool {
    feature_fields
        .structure_metadata()
        .surface_entries()
        .iter()
        .any(|entry| {
            dimension_has_active_biome(dimension, &entry.biome_id)
                && structure_reference_matches(&entry.reference, target, structures, structure_sets)
        })
        || biomes.structure_placements().iter().any(|entry| {
            matches!(entry.placement, BiomeStructurePlacementRules::Volume(_))
                && dimension_has_active_biome(dimension, &entry.biome_id)
                && structure_reference_matches(
                    &entry.structure_id,
                    target,
                    structures,
                    structure_sets,
                )
        })
}

fn structure_reference_matches(
    placement_reference: &str,
    target: &str,
    structures: &StructureRegistry,
    structure_sets: &StructureSetRegistry,
) -> bool {
    placement_reference == target
        || structures.references_overlap(placement_reference, target)
        || structure_sets
            .get(placement_reference)
            .is_some_and(|set| set.references_reference(target, structures))
}

fn dimension_has_active_biome(dimension: &DimensionDefinition, id: &str) -> bool {
    dimension
        .biomes
        .iter()
        .any(|entry| entry.id == id && entry.weight > 0.0)
}

fn volume_tile_bounds(tile: IVec2, range: BiomeVerticalRange) -> Option<(Vec3, Vec3)> {
    let minimum_x = i64::from(tile.x).checked_mul(i64::from(VOLUME_SEARCH_TILE_SIZE))?;
    let minimum_z = i64::from(tile.y).checked_mul(i64::from(VOLUME_SEARCH_TILE_SIZE))?;
    let maximum_x = minimum_x.checked_add(i64::from(VOLUME_SEARCH_TILE_SIZE))?;
    let maximum_z = minimum_z.checked_add(i64::from(VOLUME_SEARCH_TILE_SIZE))?;
    if minimum_x < i64::from(i32::MIN)
        || minimum_z < i64::from(i32::MIN)
        || maximum_x > i64::from(i32::MAX)
        || maximum_z > i64::from(i32::MAX)
    {
        return None;
    }

    Some((
        Vec3::new(minimum_x as f32, range.min, minimum_z as f32),
        Vec3::new(maximum_x as f32, range.max, maximum_z as f32),
    ))
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
    let minimum_future_horizontal = (next_radius * spacing - spacing / 2 - jitter).max(0);
    minimum_future_horizontal * minimum_future_horizontal > distance_squared
}

fn lattice_search_is_final(
    best: Option<(i64, IVec3)>,
    visited_radius: i32,
    step: i32,
    padding_steps: i32,
) -> bool {
    let Some((distance_squared, _)) = best else {
        return false;
    };
    let future_radius = visited_radius.saturating_sub(padding_steps).max(0);
    let minimum_future_horizontal = i64::from(future_radius) * i64::from(step.max(1));
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
    if best.as_ref().is_none_or(|(best_distance, best_position)| {
        distance_squared < *best_distance
            || (distance_squared == *best_distance
                && (candidate.z, candidate.x, candidate.y)
                    < (best_position.z, best_position.x, best_position.y))
    }) {
        *best = Some((distance_squared, candidate));
    }
}

fn within_horizontal_radius(origin: IVec2, candidate: IVec2, radius: i32) -> bool {
    let dx = i64::from(candidate.x) - i64::from(origin.x);
    let dz = i64::from(candidate.y) - i64::from(origin.y);
    let radius = i64::from(radius);
    dx.saturating_mul(dx).saturating_add(dz.saturating_mul(dz)) <= radius.saturating_mul(radius)
}

fn visit_square_ring(center: IVec2, radius: i32, mut visit: impl FnMut(IVec2)) {
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

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_owned();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_radius_keeps_existing_thirty_two_kiloblock_contract() {
        assert_eq!(MAX_LOCATE_BLOCK_RADIUS, 32_768);
    }

    #[test]
    fn square_ring_skips_candidates_outside_i32_domain() {
        let mut visited = Vec::new();
        visit_square_ring(IVec2::new(i32::MAX, 0), 1, |candidate| {
            visited.push(candidate);
        });

        assert!(visited.iter().any(|candidate| candidate.x == i32::MAX));
        assert!(!visited.iter().any(|candidate| candidate.x == i32::MIN));
    }

    #[test]
    fn volume_tile_bounds_reject_world_coordinate_overflow() {
        let range = BiomeVerticalRange {
            min: 0.0,
            max: 128.0,
        };
        assert!(volume_tile_bounds(IVec2::ZERO, range).is_some());
        assert!(volume_tile_bounds(IVec2::new(i32::MAX, 0), range).is_none());
    }

    #[test]
    fn horizontal_radius_uses_xz_only() {
        assert!(within_horizontal_radius(IVec2::ZERO, IVec2::new(3, 4), 5));
        assert!(!within_horizontal_radius(IVec2::ZERO, IVec2::new(4, 4), 5));
    }

    #[test]
    fn panic_payloads_are_reported_without_rethrowing() {
        let result = catch_unwind(AssertUnwindSafe(|| panic!("locate test panic")));
        let payload = result.expect_err("test panic must be caught");
        assert_eq!(panic_payload_message(payload), "locate test panic");
    }
}
