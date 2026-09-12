use bevy::{
    prelude::*,
    render::{Render, RenderApp, mesh::allocator::MeshAllocator},
};

const LOG_INTERVAL_FRAMES: u32 = 120;

pub(super) struct MeshAllocatorDiagnosticsPlugin;

impl Plugin for MeshAllocatorDiagnosticsPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(Render, log_mesh_allocator_pressure);
    }
}

fn log_mesh_allocator_pressure(allocator: Res<MeshAllocator>, mut frames: Local<u32>) {
    *frames = frames.wrapping_add(1);

    if !(*frames).is_multiple_of(LOG_INTERVAL_FRAMES) {
        return;
    }

    info!(
        "render mesh allocator: slabs={} slab_bytes={} index_allocations={}",
        allocator.slab_count(),
        allocator.slabs_size(),
        allocator.index_allocation_count(),
    );
}
