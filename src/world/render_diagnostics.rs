use bevy::prelude::*;

use crate::rendering::terrain_material::TerrainMaterial;

use super::chunk_rendering::ChunkRenderPool;

const RENDER_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 2.0;
const MESH_ASSET_OVERHEAD_WARNING: usize = 128;

pub(super) fn log_render_asset_pressure(
    time: Res<Time<Real>>,
    pool: Res<ChunkRenderPool>,
    meshes: Res<Assets<Mesh>>,
    standard_materials: Res<Assets<StandardMaterial>>,
    terrain_materials: Res<Assets<TerrainMaterial>>,
    mut timer: Local<Option<Timer>>,
) {
    let timer = timer.get_or_insert_with(|| {
        Timer::from_seconds(RENDER_DIAGNOSTIC_INTERVAL_SECONDS, TimerMode::Repeating)
    });

    if !timer.tick(time.delta()).just_finished() {
        return;
    }

    let active_chunks = pool.active_count();
    let pooled_meshes = pool.mesh_count();
    let mesh_assets = meshes.len();
    let mesh_overhead = mesh_assets.saturating_sub(pooled_meshes);

    info!(
        "render assets: active_chunks={active_chunks} pooled_meshes={pooled_meshes} mesh_assets={mesh_assets} standard_materials={} terrain_materials={}",
        standard_materials.len(),
        terrain_materials.len(),
    );

    if mesh_overhead > MESH_ASSET_OVERHEAD_WARNING {
        warn!(
            "render asset pressure: mesh asset overhead is {mesh_overhead} above the chunk render pool"
        );
    }
}
