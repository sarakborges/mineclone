use bevy::{
    prelude::*,
    render::{
        Render, RenderApp,
        mesh::allocator::{MeshAllocator, MeshAllocatorSettings},
        renderer::RenderAdapterInfo,
        slab_allocator::SlabAllocatorSettings,
    },
};

const LOG_INTERVAL_FRAMES: u32 = 120;
const MEBIBYTE: u64 = 1024 * 1024;
const MAX_GENERAL_MESH_SLAB_BYTES: u64 = 32 * MEBIBYTE;
const LARGE_MESH_THRESHOLD_BYTES: u64 = 8 * MEBIBYTE;

pub(super) struct MeshAllocatorDiagnosticsPlugin;

impl Plugin for MeshAllocatorDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, discard_empty_mesh_assets);
    }

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
    adapter: Res<RenderAdapterInfo>,
) {
    info!(
        "render mesh allocator: adapter={:?} backend={:?} device_type={:?} slabs={} slab_bytes={} index_allocations={} max_slab_bytes={} large_threshold_bytes={}",
        adapter.name,
        adapter.backend,
        adapter.device_type,
        allocator.slab_count(),
        allocator.slabs_size(),
        allocator.index_allocation_count(),
        settings.max_slab_size,
        settings.large_threshold,
    );
}

fn discard_empty_mesh_assets(
    mut events: MessageReader<AssetEvent<Mesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for event in events.read() {
        let id = match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => *id,
            AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => continue,
        };

        let empty = meshes
            .get(id)
            .is_some_and(|mesh| mesh.get_vertex_buffer_size() == 0);
        if !empty {
            continue;
        }

        warn!("discarding empty mesh asset {id:?} before render extraction");
        let _ = meshes.remove(id);
    }
}
