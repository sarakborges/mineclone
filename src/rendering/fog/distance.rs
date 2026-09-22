use bevy::prelude::*;

#[cfg(test)]
use bevy::platform::collections::HashSet;

use crate::{
    player::camera::GameplayCamera,
    voxel::{chunk::CHUNK_SIZE, coordinates::chunk_coord_from_position},
    world::{
        chunk_rendering::ChunkRenderPool,
        render_distance::RenderDistanceSettings,
    },
};

const FOG_START_RADIUS_FRACTION: f32 = 0.78;
const FOG_END_RADIUS_FRACTION: f32 = 0.98;
const FOG_STREAMING_GUARD_CHUNKS: f32 = 1.0;
const MIN_FOG_END_RADIUS_FRACTION: f32 = 0.80;

#[derive(Default)]
pub(super) struct FogDistanceState {
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
    player: Single<(Entity, &Transform), With<GameplayCamera>>,
    render_distance: Res<RenderDistanceSettings>,
    render_pool: Res<ChunkRenderPool>,
    mut fogs: Query<&mut DistanceFog, With<GameplayCamera>>,
    mut state: Local<FogDistanceState>,
) {
    let (camera_entity, player) = *player;
    let render_pool_revision = render_pool.membership_revision();
    let membership_changed = state.render_pool_revision != Some(render_pool_revision);

    let render_distance_chunks = render_distance.chunks();
    let player_horizontal = player.translation.xz();
    let frontier_inputs_changed = membership_changed
        || state.render_distance_chunks != Some(render_distance_chunks)
        || state.player_horizontal != Some(player_horizontal)
        || state.camera_entity != Some(camera_entity);

    if !frontier_inputs_changed {
        return;
    }

    state.render_pool_revision = Some(render_pool_revision);
    state.render_distance_chunks = Some(render_distance_chunks);
    state.player_horizontal = Some(player_horizontal);
    state.camera_entity = Some(camera_entity);

    let (_, target_end) = fog_distances(render_distance_chunks);
    let guard_end = nearest_missing_column_distance(
        player.translation,
        render_distance_chunks,
        |column| render_pool.contains_column(column),
    )
    .map(|distance| distance - FOG_STREAMING_GUARD_CHUNKS * CHUNK_SIZE as f32);
    let minimum_end = minimum_fog_end(render_distance_chunks);
    let end = guard_end
        .map_or(target_end, |guard_end| guard_end.min(target_end))
        .max(minimum_end);
    let (start, end) = guarded_fog_distances(render_distance_chunks, end);

    for mut fog in &mut fogs {
        fog.falloff = FogFalloff::Linear { start, end };
    }
}

fn guarded_fog_distances(render_distance_chunks: i32, end: f32) -> (f32, f32) {
    let (target_start, target_end) = fog_distances(render_distance_chunks);
    let minimum_end = minimum_fog_end(render_distance_chunks);
    let end = end.min(target_end).max(minimum_end);
    let target_span = target_end - target_start;
    let start = (end - target_span).max(0.0);

    (start, end)
}

fn minimum_fog_end(render_distance_chunks: i32) -> f32 {
    render_distance_chunks.max(1) as f32
        * CHUNK_SIZE as f32
        * MIN_FOG_END_RADIUS_FRACTION
}

fn nearest_missing_column_distance(
    player_position: Vec3,
    render_distance_chunks: i32,
    mut column_is_active: impl FnMut(IVec2) -> bool,
) -> Option<f32> {
    let radius = render_distance_chunks.max(1);
    let center = chunk_coord_from_position(player_position).xz();
    let player = player_position.xz();
    let mut nearest: Option<f32> = None;

    for z in -radius..=radius {
        for x in -radius..=radius {
            let offset = IVec2::new(x, z);
            if offset.length_squared() > radius * radius {
                continue;
            }

            let column = center + offset;
            if column_is_active(column) {
                continue;
            }

            let distance = horizontal_distance_to_chunk(player, column);
            nearest = Some(nearest.map_or(distance, |current| current.min(distance)));
        }
    }

    nearest
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
    fn complete_render_columns_do_not_limit_fog() {
        let radius = 4;
        let columns = filled_columns(radius);

        assert_eq!(
            nearest_missing_column_distance(
                Vec3::new(8.0, 0.0, 8.0),
                radius,
                |column| columns.contains(&column),
            ),
            None,
        );
    }

    #[test]
    fn missing_column_uses_distance_to_nearest_chunk_edge() {
        let radius = 4;
        let mut columns = filled_columns(radius);
        columns.remove(&IVec2::new(3, 0));

        let distance = nearest_missing_column_distance(
            Vec3::new(8.0, 0.0, 8.0),
            radius,
            |column| columns.contains(&column),
        )
        .expect("missing column should constrain the fog frontier");

        assert_eq!(distance, 40.0);
    }

    #[test]
    fn guarded_fog_preserves_span_while_receding() {
        let render_distance_chunks = 12;
        let (target_start, target_end) = fog_distances(render_distance_chunks);
        let minimum_end = minimum_fog_end(render_distance_chunks);
        let (start, end) = guarded_fog_distances(render_distance_chunks, minimum_end);

        assert!((end - minimum_end).abs() < 0.001);
        assert!((end - start - (target_end - target_start)).abs() < 0.001);
        assert!(end < target_end);
    }

    #[test]
    fn streaming_guard_never_pulls_fog_into_the_players_face_at_high_distance() {
        let render_distance_chunks = 24;
        let (_, end) = guarded_fog_distances(render_distance_chunks, 0.0);

        assert!(end >= 24.0 * CHUNK_SIZE as f32 * 0.79);
    }
}
