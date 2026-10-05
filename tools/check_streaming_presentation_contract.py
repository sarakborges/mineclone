#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STREAMING = (ROOT / "src/world/streaming.rs").read_text(encoding="utf-8")
GENERATION = (ROOT / "src/world/streaming/generation.rs").read_text(encoding="utf-8")
PRESENTATION = (ROOT / "src/world/streaming/presentation.rs").read_text(encoding="utf-8")
VISIBILITY = (ROOT / "src/world/chunk_visibility.rs").read_text(encoding="utf-8")


def compact(source: str) -> str:
    return "".join(source.split())


streaming = compact(STREAMING)
generation = compact(GENERATION)
presentation = compact(PRESENTATION)
visibility = compact(VISIBILITY)

required_generation = [
    "seed_chunk_direct_lighting(",
    "enqueue_empty_chunk_relaxation(coord)",
    "enqueue_chunk_relaxation(coord)",
    "enqueue_loaded_column_below(runtime.world,coord)",
    "streaming.enqueue_presentation(coord)",
]
for marker in required_generation:
    if marker not in generation:
        raise SystemExit(f"streamed chunk activation lost lighting/presentation contract: {marker}")

insert = "runtime.world.insert_chunk(coord,completed.output.into_chunk())"
lighting = "activate_resident_lighting_and_presentation(streaming,runtime,coord)"
if generation.index(insert) > generation.index(lighting, generation.index(insert)):
    pass
else:
    raise SystemExit("new chunk lighting/presentation activation must happen after VoxelWorld publication")

required_streaming = [
    "presentation_pending:DeduplicatedQueue<IVec3>",
    "sync_from_streaming(Some(center),horizontal_radius)",
    "publish_pending_chunk_presentations.after(process_dynamic_lighting).before(process_chunk_remesh_queue)",
    "runtime.world.chunk(*coord).is_some()&&!inputs.render_pool.contains(*coord)",
]
for marker in required_streaming:
    if marker not in streaming:
        raise SystemExit(f"streaming presentation integration lost required behavior: {marker}")

required_presentation = [
    "spawn_built_chunk_meshes(",
    "Vec::new()",
    "remesh.enqueue_halo_change(coord,IVec3::ZERO,has_geometry,has_fluid)",
    "notify_presented_chunk_neighbors(",
]
for marker in required_presentation:
    if marker not in presentation:
        raise SystemExit(f"presentation adapter lost required behavior: {marker}")
if "build_chunk_render_meshes" in presentation:
    raise SystemExit("streaming presentation must not synchronously build chunk meshes")

if "pub(crate)fnsync_from_streaming" not in visibility:
    raise SystemExit("presentation selection must be synchronized by streaming")

print(
    "Streaming presentation contract audit passed: resident direct-light seed + relaxation, "
    "presentation selection sync, async remesh publication, and neighbor halo catch-up"
)
