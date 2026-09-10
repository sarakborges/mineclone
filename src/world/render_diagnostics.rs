use std::collections::HashMap;

use bevy::{ecs::system::SystemParam, prelude::*, text::FontAtlasSet};

use crate::{app::game_state::GameState, rendering::terrain_material::TerrainMaterial};

use super::chunk_rendering::ChunkRenderPool;

const RENDER_DIAGNOSTIC_INTERVAL_SECONDS: f32 = 2.0;
const MESH_ASSET_OVERHEAD_WARNING: usize = 128;

#[derive(SystemParam)]
pub(super) struct RenderDiagnosticAssets<'w> {
    state: Res<'w, State<GameState>>,
    time: Res<'w, Time<Real>>,
    asset_server: Res<'w, AssetServer>,
    pool: Res<'w, ChunkRenderPool>,
    meshes: Res<'w, Assets<Mesh>>,
    images: Res<'w, Assets<Image>>,
    font_atlases: Res<'w, FontAtlasSet>,
    standard_materials: Res<'w, Assets<StandardMaterial>>,
    terrain_materials: Res<'w, Assets<TerrainMaterial>>,
}

pub(super) fn log_render_asset_pressure(
    assets: RenderDiagnosticAssets,
    mut timer: Local<Option<Timer>>,
) {
    if !matches!(assets.state.get(), GameState::Loading | GameState::Gameplay) {
        return;
    }

    let timer = timer.get_or_insert_with(|| {
        Timer::from_seconds(RENDER_DIAGNOSTIC_INTERVAL_SECONDS, TimerMode::Repeating)
    });

    if !timer.tick(assets.time.delta()).just_finished() {
        return;
    }

    let active_chunks = assets.pool.active_count();
    let pooled_meshes = assets.pool.mesh_count();
    let pooled_mesh_bytes = assets.pool.mesh_bytes();
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
    let runtime_top_shape = runtime_image_shapes
        .into_iter()
        .max_by_key(|(_, count)| *count);
    let font_atlas_keys = assets.font_atlases.len();
    let font_atlas_count = assets.font_atlases.values().map(Vec::len).sum::<usize>();
    let font_atlas_bytes = assets.font_atlases.total_bytes(&assets.images);

    info!(
        "render assets: state={:?} active_chunks={active_chunks} pooled_meshes={pooled_meshes} pooled_mesh_bytes={pooled_mesh_bytes} mesh_assets={mesh_assets} images={image_assets} file_images={file_images} runtime_images={runtime_images} runtime_top_shape={runtime_top_shape:?} font_atlas_keys={font_atlas_keys} font_atlases={font_atlas_count} font_atlas_bytes={font_atlas_bytes} standard_materials={} terrain_materials={}",
        assets.state.get(),
        assets.standard_materials.len(),
        assets.terrain_materials.len(),
    );

    if mesh_overhead > MESH_ASSET_OVERHEAD_WARNING {
        warn!(
            "render asset pressure: mesh asset overhead is {mesh_overhead} above the chunk render pool"
        );
    }
}
