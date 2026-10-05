pub(crate) mod block_face;
pub(crate) mod block_gravity;
// Block metadata/state plumbing is intentionally staged ahead of all runtime consumers.
// Keep strict dead-code linting everywhere else while these modules are being wired in.
#[allow(dead_code)]
pub(crate) mod block_metadata;
#[allow(dead_code)]
pub(crate) mod block_state;
#[allow(dead_code)]
pub(crate) mod cell;
#[allow(dead_code)]
pub(crate) mod chunk;
// Phase 1 removes world materialization while retaining the voxel runtime that
// the replacement pipeline will reconnect. Allow only the temporarily dormant
// runtime capabilities instead of disabling dead-code linting for the crate.
#[allow(dead_code)]
pub(crate) mod chunk_archive;
pub(crate) mod chunk_disk;
pub(crate) mod collision;
pub(crate) mod coordinates;
#[allow(dead_code)]
pub(crate) mod deduplicated_queue;
pub(crate) mod edit;
pub(crate) mod fluid;
pub(crate) mod fluid_mesh;
pub(crate) mod layer;
pub(crate) mod layer_mesh;
pub(crate) mod light;
#[allow(dead_code)]
pub(crate) mod lighting;
pub(crate) mod log_variant;
pub(crate) mod mesh;
pub(crate) mod mesh_buffer;
pub(crate) mod mesh_lighting;
#[allow(dead_code)]
pub(crate) mod mesh_snapshot;
pub(crate) mod meshlet;
pub(crate) mod microblock;
pub(crate) mod neighbors;
pub(crate) mod object;
pub(crate) mod orientation;
pub(crate) mod quad;
pub(crate) mod raycast;
pub(crate) mod read;
pub(crate) mod revision;
#[allow(dead_code)]
pub(crate) mod spatial_search;
pub(crate) mod stackable_layer;
pub(crate) mod texture_rotation;
#[allow(dead_code)]
pub(crate) mod update_queue;
#[allow(dead_code)]
pub(crate) mod world;