use bevy::prelude::*;

use crate::rendering::GameplayAssetPreloads;

use super::super::{WorldLoadingPhase, system_params::WorldSetupProgress};

pub(super) fn wait_for_gameplay_assets(
    asset_server: &AssetServer,
    preloads: &GameplayAssetPreloads,
    progress: &mut WorldSetupProgress<'_>,
) {
    let load_progress = preloads.load_progress(asset_server);
    progress.loading_state.assets_loaded = load_progress.loaded;
    progress.loading_state.assets_total = load_progress.total;

    if load_progress.is_complete() {
        progress.loading_state.phase = WorldLoadingPhase::Finalizing;
    }
}
