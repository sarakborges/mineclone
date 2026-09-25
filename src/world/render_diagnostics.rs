use std::collections::HashMap;

use bevy::{ecs::system::SystemParam, prelude::*, text::FontAtlasSet};

use crate::{
    app::game_state::GameState,
    rendering::terrain_material::TerrainMaterial,
    world_objects::{
        ObjectMaterialCache, StackedSpriteMaterialCache, StackedSpriteMeshCache,
        WorldObjectStore,
    },
};

use super::{
    chunk_async_work::ChunkAsyncWorkLimiter,
    chunk_generation_tasks::ChunkGenerationTasks,
    chunk_mesh_tasks::ChunkMeshTasks,
    chunk_remesh::ChunkRemeshQueue,
    chunk_remesh_tasks::ChunkRemeshTasks,
    chunk_rendering::ChunkRenderPool,
    streaming::ChunkStreamingState,
};

const RENDER_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 10.0;
const MESH_ASSET_OVERHEAD_WARNING: usize = 128;
const RUNTIME_IMAGE_SHAPE_LIMIT: usize = 4;

#[derive(Clone, Copy, Debug)]
pub(super) struct RenderDiagnosticSnapshot {
    runtime_images: usize,
    font_atlases: usize,
    font_atlas_bytes: u64,
    non_font_runtime_images: usize,
}

#[derive(SystemParam)]
pub(super) struct RenderDiagnosticAssets<'w> {
    state: Res<'w, State<GameState>>,
    asset_server: Res<'w, AssetServer>,
    pool: Res<'w, ChunkRenderPool>,
    streaming: Res<'w, ChunkStreamingState>,
    generation_tasks: Res<'w, ChunkGenerationTasks>,
    async_work: Res<'w, ChunkAsyncWorkLimiter>,
    mesh_tasks: Res<'w, ChunkMeshTasks>,
    remesh_queue: Res<'w, ChunkRemeshQueue>,
    remesh_tasks: Res<'w, ChunkRemeshTasks>,
    meshes: Res<'w, Assets<Mesh>>,
    images: Res<'w, Assets<Image>>,
    font_atlases: Res<'w, FontAtlasSet>,
    standard_materials: Res<'w, Assets<StandardMaterial>>,
    terrain_materials: Res<'w, Assets<TerrainMaterial>>,
    world_objects: Res<'w, WorldObjectStore>,
    object_materials: Res<'w, ObjectMaterialCache>,
    stacked_object_meshes: Res<'w, StackedSpriteMeshCache>,
    stacked_object_materials: Res<'w, StackedSpriteMaterialCache>,
}

pub(super) fn render_diagnostics_due(
    state: Res<State<GameState>>,
    time: Res<Time<Real>>,
    mut timer: Local<Option<Timer>>,
) -> bool {
    if !matches!(state.get(), GameState::Loading | GameState::Gameplay) {
        return false;
    }

    let timer = timer.get_or_insert_with(|| {
        Timer::from_seconds(RENDER_DIAGNOSTIC_INTERVAL_SECONDS, TimerMode::Repeating)
    });
    timer.tick(time.delta()).just_finished()
}

pub(super) fn log_render_asset_pressure(
    assets: RenderDiagnosticAssets,
    mut previous: Local<Option<RenderDiagnosticSnapshot>>,
) {
    let active_chunks = assets.pool.active_count();
    let pooled_meshes = assets.pool.mesh_count();
    let render_entities = assets.pool.entity_count();
    let (terrain_array_meshes, terrain_legacy_meshes, layer_meshes, fluid_meshes) =
        assets.pool.diagnostic_mesh_kind_counts();
    let pooled_mesh_bytes = assets.pool.mesh_bytes();
    let recomputed_mesh_bytes = assets.pool.diagnostic_recomputed_mesh_bytes();
    let (
        stream_pending,
        stream_ready,
        generation_wave_pending,
        generation_wave_targets,
        staged_generated_chunks,
        pressure_evicted_meshes,
    ) = assets.streaming.diagnostic_counts();
    let (pending_priority_scan, ready_priority_scan) =
        assets.streaming.take_priority_scan_diagnostics();
    let generation_tasks = assets.generation_tasks.pending_count();
    let async_chunk_work = assets.async_work.in_flight();
    let async_chunk_work_limit = assets.async_work.limit();
    let async_timings = assets.async_work.take_diagnostics();
    let mesh_tasks = assets.mesh_tasks.pending_count();
    let remesh_tasks = assets.remesh_tasks.pending_count();
    let (remesh_geometry, remesh_lighting, remesh_fluid) =
        assets.remesh_queue.diagnostic_counts();
    let mesh_assets = assets.meshes.len();
    let mesh_overhead = mesh_assets.saturating_sub(pooled_meshes);
    let mut file_images = 0;
    let mut runtime_image_shapes = HashMap::<(u32, u32), usize>::new();

    for (id, image) in assets.images.iter() {
        if assets.asset_server.get_path(id).is_some() {
            file_images += 1;
            continue;
        }

        let size = image.texture_descriptor.size;
        *runtime_image_shapes
            .entry((size.width, size.height))
            .or_default() += 1;
    }

    let image_assets = assets.images.len();
    let runtime_images = image_assets.saturating_sub(file_images);
    let mut runtime_top_shapes = runtime_image_shapes.into_iter().collect::<Vec<_>>();
    runtime_top_shapes.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    runtime_top_shapes.truncate(RUNTIME_IMAGE_SHAPE_LIMIT);

    let font_atlas_keys = assets.font_atlases.len();
    let font_atlas_count = assets.font_atlases.values().map(Vec::len).sum::<usize>();
    let font_atlas_bytes = assets.font_atlases.total_bytes(&assets.images);
    let non_font_runtime_images = runtime_images.saturating_sub(font_atlas_count);
    let snapshot = RenderDiagnosticSnapshot {
        runtime_images,
        font_atlases: font_atlas_count,
        font_atlas_bytes,
        non_font_runtime_images,
    };
    let deltas = previous.as_ref().map(|previous| {
        (
            signed_delta(snapshot.runtime_images, previous.runtime_images),
            signed_delta(snapshot.font_atlases, previous.font_atlases),
            signed_delta_u64(snapshot.font_atlas_bytes, previous.font_atlas_bytes),
            signed_delta(
                snapshot.non_font_runtime_images,
                previous.non_font_runtime_images,
            ),
        )
    });

    info!(
        "render assets: state={:?} active_chunks={active_chunks} pooled_meshes={pooled_meshes} render_entities={render_entities} terrain_array_meshes={terrain_array_meshes} terrain_legacy_meshes={terrain_legacy_meshes} layer_meshes={layer_meshes} fluid_meshes={fluid_meshes} pooled_mesh_bytes={pooled_mesh_bytes} stream_pending={stream_pending} stream_ready={stream_ready} pending_priority_scans={} pending_priority_avg_us={} pending_priority_max_us={} pending_priority_max_queue={} ready_priority_scans={} ready_priority_avg_us={} ready_priority_max_us={} ready_priority_max_queue={} generation_tasks={generation_tasks} async_chunk_work={async_chunk_work}/{async_chunk_work_limit} async_generation={:?} async_initial_mesh={:?} async_remesh={:?} generation_wave_pending={generation_wave_pending} generation_wave_targets={generation_wave_targets} staged_generated_chunks={staged_generated_chunks} pressure_evicted_meshes={pressure_evicted_meshes} mesh_tasks={mesh_tasks} remesh_tasks={remesh_tasks} remesh_geometry={remesh_geometry} remesh_lighting={remesh_lighting} remesh_fluid={remesh_fluid} mesh_assets={mesh_assets} images={image_assets} file_images={file_images} runtime_images={runtime_images} non_font_runtime_images={non_font_runtime_images} runtime_top_shapes={runtime_top_shapes:?} font_atlas_keys={font_atlas_keys} font_atlases={font_atlas_count} font_atlas_bytes={font_atlas_bytes} deltas={deltas:?} standard_materials={} terrain_materials={} world_objects={} world_object_chunks={} object_material_cache={} stacked_object_mesh_cache={} stacked_object_material_cache={}",
        assets.state.get(),
        pending_priority_scan.count,
        pending_priority_scan.average_micros,
        pending_priority_scan.max_micros,
        pending_priority_scan.max_queue_len,
        ready_priority_scan.count,
        ready_priority_scan.average_micros,
        ready_priority_scan.max_micros,
        ready_priority_scan.max_queue_len,
        async_timings.generation,
        async_timings.initial_mesh,
        async_timings.remesh,
        assets.standard_materials.len(),
        assets.terrain_materials.len(),
        assets.world_objects.materialized_object_count(),
        assets.world_objects.materialized_chunk_count(),
        assets.object_materials.len(),
        assets.stacked_object_meshes.len(),
        assets.stacked_object_materials.len(),
    );

    *previous = Some(snapshot);

    if pooled_mesh_bytes != recomputed_mesh_bytes {
        warn!(
            "render asset pressure: incremental mesh bytes drifted: tracked={pooled_mesh_bytes} recomputed={recomputed_mesh_bytes}"
        );
    }

    if !assets.pool.diagnostic_active_columns_are_consistent() {
        warn!("render asset pressure: incremental active render-column counts drifted");
    }

    if mesh_overhead > MESH_ASSET_OVERHEAD_WARNING {
        warn!(
            "render asset pressure: mesh asset overhead is {mesh_overhead} above the chunk render pool"
        );
    }
}

fn signed_delta(current: usize, previous: usize) -> i64 {
    if current >= previous {
        current.saturating_sub(previous).min(i64::MAX as usize) as i64
    } else {
        -(previous.saturating_sub(current).min(i64::MAX as usize) as i64)
    }
}

fn signed_delta_u64(current: u64, previous: u64) -> i64 {
    if current >= previous {
        current.saturating_sub(previous).min(i64::MAX as u64) as i64
    } else {
        -(previous.saturating_sub(current).min(i64::MAX as u64) as i64)
    }
}
