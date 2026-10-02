use bevy::prelude::*;

#[cfg(test)]
use bevy::platform::collections::HashSet;

use crate::{
    player::{
        PlayerEntity,
        camera::GameplayWorldCamera,
    },
    voxel::{chunk::CHUNK_SIZE, coordinates::chunk_coord_from_position},
    world::{
        chunk_rendering::ChunkRenderPool,
        render_distance::RenderDistanceSettings,
    },
};

const FOG_START_RADIUS_FRACTION: f32 = 0.78;
const FOG_END_RADIUS_FRACTION: f32 = 0.98;
const FOG_STREAMING_GUARD_CHUNKS: f32 = 1.0;
const MIN_FOG_END_CHUNKS: f32 = 1.0;

#[derive(Default)]
pub(super) struct FogDistanceState {
    missing_columns: Vec<IVec2>,
    boundary_columns: Vec<IVec2>,
    frontier_center: Option<IVec2>,
    render_pool_revision: Option<u64>,
    render_distance_chunks: Option<i32>,
    player_horizontal: Option<Vec2>,
    camera_entity: Option<Entity>,
}

pub(super) fn fog_distances(render_distance_chunks: i32) -> (f32, f32) {
    let chunk_size = CHUNK_SIZE as f32;
    let radius = render_distance_chunks.max(1) as f32 * chunk_size;
    let start = (radius * FOG_START_RADIUS_FRACTION).max(chunk_size);
    let end = (radius * FOG_END_RADIUS_FRACTION).max(start + chunk_size);

    (start, end)
}

pub(super) fn fog_falloff(render_distance_chunks: i32) -> FogFalloff {
    let (start, end) = fog_distances(render_distance_chunks);
    FogFalloff::Linear { start, end }
}

pub(super) fn update_fog_distance(
    player: Single<&Transform, With<PlayerEntity>>,
    camera: Single<Entity, With<GameplayWorldCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    render_pool: Res<ChunkRenderPool>,
    mut fogs: Query<&mut DistanceFog, With<GameplayWorldCamera>>,
    mut state: Local<FogDistanceState>,
) {
    let player = *player;
    let camera_entity = *camera;
    let render_pool_revision = render_pool.membership_revision();
    let render_distance_chunks = render_distance.chunks();
    let frontier_center = chunk_coord_from_position(player.translation).xz();
    let player_horizontal = player.translation.xz();
    let missing_columns_changed =
        state.render_pool_revision != Some(render_pool_revision)
            || state.render_distance_chunks != Some(render_distance_chunks)
            || state.frontier_center != Some(frontier_center);

    if missing_columns_changed {
        collect_missing_columns(
            frontier_center,
            render_distance_chunks,
            |column| render_pool.contains_column(column),
            &mut state.missing_columns,
        );
        collect_boundary_columns(
            frontier_center,
            render_distance_chunks,
            &mut state.boundary_columns,
        );
    }

    let frontier_inputs_changed = missing_columns_changed
        || state.player_horizontal != Some(player_horizontal)
        || state.camera_entity != Some(camera_entity);
    if !frontier_inputs_changed {
        return;
    }

    state.frontier_center = Some(frontier_center);
    state.render_pool_revision = Some(render_pool_revision);
    state.render_distance_chunks = Some(render_distance_chunks);
    state.player_horizontal = Some(player_horizontal);
    state.camera_entity = Some(camera_entity);

    let (_, target_end) = fog_distances(render_distance_chunks);
    let boundary_end = nearest_column_distance(player_horizontal, &state.boundary_columns);
    let guard_end = nearest_column_distance(player_horizontal, &state.missing_columns)
        .map(|distance| distance - FOG_STREAMING_GUARD_CHUNKS * CHUNK_SIZE as f32);
    let minimum_end = minimum_fog_end();
    let end = target_end
        .min(boundary_end.unwrap_or(target_end))
        .min(guard_end.unwrap_or(target_end))
        .max(minimum_end);
    let (start, end) = guarded_fog_distances(render_distance_chunks, end);

    for mut fog in &mut fogs {
        // Linear fog reaches the fully opaque fog color at `end`. Clamp that
        // point to the nearest render-boundary chunk so loaded edge geometry
        // cannot remain visible behind fully opaque fog. Missing chunks can
        // still pull the endpoint farther inward while streaming catches up.
        fog.falloff = FogFalloff::Linear { start, end };
    }
}

fn guarded_fog_distances(render_distance_chunks: i32, end: f32) -> (f32, f32) {
    let (target_start, target_end) = fog_distances(render_distance_chunks);
    let minimum_end = minimum_fog_end();
    let end = end.min(target_end).max(minimum_end);
    let target_span = target_end - target_start;
    let start = (end - target_span).max(0.0);

    (start, end)
}

fn minimum_fog_end() -> f32 {
    MIN_FOG_END_CHUNKS * CHUNK_SIZE as f32
}

fn collect_missing_columns(
    center: IVec2,
    render_distance_chunks: i32,
    mut column_is_active: impl FnMut(IVec2) -> bool,
    missing: &mut Vec<IVec2>,
) {
    missing.clear();
    let radius = render_distance_chunks.max(1);

    for z in -radius..=radius {
        for x in -radius..=radius {
            let offset = IVec2::new(x, z);
            if offset.length_squared() > radius * radius {
                continue;
            }

            let column = center + offset;
            if !column_is_active(column) {
                missing.push(column);
            }
        }
    }
}

fn collect_boundary_columns(
    center: IVec2,
    render_distance_chunks: i32,
    boundary: &mut Vec<IVec2>,
) {
    boundary.clear();
    let radius = render_distance_chunks.max(1);
    let radius_squared = radius * radius;
    let cardinal_neighbors = [
        IVec2::new(1, 0),
        IVec2::new(-1, 0),
        IVec2::new(0, 1),
        IVec2::new(0, -1),
    ];

    for z in -radius..=radius {
        for x in -radius..=radius {
            let offset = IVec2::new(x, z);
            if offset.length_squared() > radius_squared {
                continue;
            }

            let touches_outside = cardinal_neighbors
                .iter()
                .any(|neighbor| (offset + *neighbor).length_squared() > radius_squared);
            if touches_outside {
                boundary.push(center + offset);
            }
        }
    }
}

fn nearest_column_distance(player: Vec2, columns: &[IVec2]) -> Option<f32> {
    columns
        .iter()
        .copied()
        .map(|column| horizontal_distance_to_chunk(player, column))
        .reduce(f32::min)
}

fn horizontal_distance_to_chunk(player: Vec2, column: IVec2) -> f32 {
    let chunk_size = CHUNK_SIZE as f32;
    let minimum = column.as_vec2() * chunk_size;
    let maximum = minimum + Vec2::splat(chunk_size);
    let closest = player.clamp(minimum, maximum);
    player.distance(closest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled_columns(radius: i32) -> HashSet<IVec2> {
        let mut columns = HashSet::default();
        for z in -radius..=radius {
            for x in -radius..=radius {
                let offset = IVec2::new(x, z);
                if offset.length_squared() <= radius * radius {
                    columns.insert(offset);
                }
            }
        }
        columns
    }

    #[test]
    fn fog_stays_close_to_the_render_boundary() {
        let render_distance_chunks = 10;
        let radius = render_distance_chunks as f32 * CHUNK_SIZE as f32;
        let (start, end) = fog_distances(render_distance_chunks);

        assert!(start >= radius * 0.76);
        assert!(start <= radius * 0.80);
        assert!(end >= radius * 0.96);
        assert!(end <= radius);
        assert!(end > start);
    }

    #[test]
    fn render_boundary_can_pull_full_opacity_before_loaded_edge_chunks() {
        let render_distance_chunks = 10;
        let player = Vec2::new(15.5, 15.5);
        let mut boundary = Vec::new();
        collect_boundary_columns(
            IVec2::ZERO,
            render_distance_chunks,
            &mut boundary,
        );

        let boundary_end = nearest_column_distance(player, &boundary)
            .expect("render disk must have boundary columns");
        let (_, target_end) = fog_distances(render_distance_chunks);
        let (_, end) = guarded_fog_distances(
            render_distance_chunks,
            boundary_end.min(target_end),
        );

        assert!(boundary_end < target_end);
        assert!((end - boundary_end).abs() < 0.001);
        assert!(boundary.iter().all(|column| {
            horizontal_distance_to_chunk(player, *column) + 0.001 >= end
        }));
    }

    #[test]
    fn complete_render_columns_do_not_limit_fog() {
        let radius = 4;
        let columns = filled_columns(radius);

        let mut missing = Vec::new();
        collect_missing_columns(
            IVec2::ZERO,
            radius,
            |column| columns.contains(&column),
            &mut missing,
        );

        assert_eq!(
            nearest_column_distance(Vec2::new(8.0, 8.0), &missing),
            None,
        );
    }

    #[test]
    fn missing_column_uses_distance_to_nearest_chunk_edge() {
        let radius = 4;
        let mut columns = filled_columns(radius);
        columns.remove(&IVec2::new(3, 0));

        let mut missing = Vec::new();
        collect_missing_columns(
            IVec2::ZERO,
            radius,
            |column| columns.contains(&column),
            &mut missing,
        );
        let distance = nearest_column_distance(
            Vec2::new(8.0, 8.0),
            &missing,
        )
        .expect("missing column should constrain the fog frontier");

        assert_eq!(distance, 40.0);
    }

    #[test]
    fn guarded_fog_preserves_span_while_receding() {
        let render_distance_chunks = 12;
        let (target_start, target_end) = fog_distances(render_distance_chunks);
        let guarded_end = 4.0 * CHUNK_SIZE as f32;
        let (start, end) = guarded_fog_distances(render_distance_chunks, guarded_end);

        assert!((end - guarded_end).abs() < 0.001);
        assert!((end - start - (target_end - target_start)).abs() < 0.001);
        assert!(end < target_end);
    }

    #[test]
    fn streaming_guard_can_hide_missing_chunks_inside_previous_floor() {
        let render_distance_chunks = 24;
        let guarded_end = 3.0 * CHUNK_SIZE as f32;
        let (_, end) = guarded_fog_distances(render_distance_chunks, guarded_end);

        assert!((end - guarded_end).abs() < 0.001);
        assert!(end < render_distance_chunks as f32 * CHUNK_SIZE as f32 * 0.80);
    }
}
