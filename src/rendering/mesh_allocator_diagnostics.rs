use bevy::{
    prelude::*,
    render::{
        Render, RenderApp,
        mesh::allocator::{MeshAllocator, MeshAllocatorSettings},
        slab_allocator::SlabAllocatorSettings,
    },
};

const LOG_INTERVAL_FRAMES: u32 = 120;
const MEBIBYTE: u64 = 1024 * 1024;
const MAX_GENERAL_MESH_SLAB_BYTES: u64 = 64 * MEBIBYTE;
const LARGE_MESH_THRESHOLD_BYTES: u64 = 16 * MEBIBYTE;

pub(super) struct MeshAllocatorDiagnosticsPlugin;

impl Plugin for MeshAllocatorDiagnosticsPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.insert_resource(MeshAllocatorSettings {
            slab_allocator_settings: SlabAllocatorSettings {
                min_slab_size: MEBIBYTE,
                max_slab_size: MAX_GENERAL_MESH_SLAB_BYTES,
                large_threshold: LARGE_MESH_THRESHOLD_BYTES,
                growth_factor: 1.5,
            },
            ..default()
        });
        render_app.add_systems(
            Render,
            log_mesh_allocator_pressure.run_if(mesh_allocator_diagnostics_due),
        );
    }
}

fn mesh_allocator_diagnostics_due(mut frames: Local<u32>) -> bool {
    *frames = frames.wrapping_add(1);
    (*frames).is_multiple_of(LOG_INTERVAL_FRAMES)
}

fn log_mesh_allocator_pressure(
    allocator: Res<MeshAllocator>,
    settings: Res<MeshAllocatorSettings>,
) {
    info!(
        "render mesh allocator: slabs={} slab_bytes={} index_allocations={} max_slab_bytes={} large_threshold_bytes={}",
        allocator.slab_count(),
        allocator.slabs_size(),
        allocator.index_allocation_count(),
        settings.max_slab_size,
        settings.large_threshold,
    );
}
