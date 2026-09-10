use std::collections::HashMap;

use bevy::prelude::*;

use crate::{app::game_state::GameState, rendering::terrain_material::TerrainMaterial};

use super::chunk_rendering::ChunkRenderPool;

const RENDER_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 2.0;
const MESH_ASSET_OVERHEAD_WARNING: usize = 128;

pub(super) fn log_render_asset_pressure(
    state: Res<State<GameState>>,
    time: Res<Time<Real>>,
    asset_server: Res<AssetServer>,
    pool: Res<ChunkRenderPool>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    standard_materials: Res<Assets<StandardMaterial>>,
    terrain_materials: Res<Assets<TerrainMaterial>>,
    mut timer: Local<Option<Timer>>,
) {
    if !matches!(state.get(), GameState::Loading | GameState::Gameplay) {
        return;
    }

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
    let mut file_images = 0;
    let mut runtime_image_shapes = HashMap::<(u32, u32), usize>::new();

    for (id, image) in images.iter() {
        if asset_server.get_path(id).is_some() {
            file_images += 1;
            continue;
        }

        let size = image.texture_descriptor.size;
        *runtime_image_shapes
            .entry((size.width, size.height))
            .or_default() += 1;
    }

    let image_assets = images.len();
    let runtime_images = image_assets.saturating_sub(file_images);
    let runtime_top_shape = runtime_image_shapes
        .into_iter()
        .max_by_key(|(_, count)| *count);

    info!(
        "render assets: state={:?} active_chunks={active_chunks} pooled_meshes={pooled_meshes} mesh_assets={mesh_assets} images={image_assets} file_images={file_images} runtime_images={runtime_images} runtime_top_shape={runtime_top_shape:?} standard_materials={} terrain_materials={}",
        state.get(),
        standard_materials.len(),
        terrain_materials.len(),
    );

    if mesh_overhead > MESH_ASSET_OVERHEAD_WARNING {
        warn!(
            "render asset pressure: mesh asset overhead is {mesh_overhead} above the chunk render pool"
        );
    }
}
