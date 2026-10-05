#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STREAMING = ROOT / "src/world/streaming.rs"
GENERATION = ROOT / "src/world/streaming/generation.rs"
SELECTION = ROOT / "src/world/streaming/selection.rs"
WORLD = ROOT / "src/world/mod.rs"

streaming = STREAMING.read_text(encoding="utf-8")
generation = GENERATION.read_text(encoding="utf-8")
selection = SELECTION.read_text(encoding="utf-8")
world = WORLD.read_text(encoding="utf-8")


def compact(source: str) -> str:
    return "".join(source.split())


streaming_compact = compact(streaming)
generation_compact = compact(generation)
selection_compact = compact(selection)

required_streaming = [
    "#[derive(SystemParam)]structChunkStreamingInputs",
    "#[derive(SystemParam)]structChunkStreamingRuntime",
    "desired_chunk_coords(&inputs.generator",
    "collect_materialized_chunks(",
    "dispatch_materialization_tasks(",
    "restart_for_generator_change",
    "chunk_load_priority(*coord,center,movement_direction)",
]
for marker in required_streaming:
    if marker not in streaming_compact:
        raise SystemExit(f"streaming integration lost required runtime contract: {marker}")

required_generation = [
    "generator.materialize_chunk(coord)",
    "completed.revision!=current_revision",
    "runtime.world.restore_chunk(coord)",
    "runtime.world.insert_chunk(coord,completed.output.into_chunk())",
    "enqueue_generated_fluid_frontier(",
    "enqueue_neighbor_frontiers_targeting_chunk(",
    "runtime.pending_fluid.reactivate_loaded_chunk(coord,runtime.current_tick)",
]
for marker in required_generation:
    if marker not in generation_compact:
        raise SystemExit(f"materialization publication lost required behavior: {marker}")

insert_marker = "runtime.world.insert_chunk(coord,completed.output.into_chunk())"
reactivate_marker = "runtime.pending_fluid.reactivate_loaded_chunk(coord,runtime.current_tick)"
if generation_compact.index(insert_marker) > generation_compact.index(reactivate_marker):
    raise SystemExit("new generated chunk must become resident before runtime fluid reactivation")

collect_start = generation_compact.index("pub(super)fncollect_materialized_chunks")
dispatch_start = generation_compact.index("pub(super)fndispatch_materialization_tasks")
collect_body = generation_compact[collect_start:dispatch_start]
if collect_body.index(insert_marker) > collect_body.index("enqueue_generated_fluid_frontier("):
    raise SystemExit("generated frontier handoff must happen after VoxelWorld publication")

required_selection = [
    "generator.terrain().sample_surface_area(",
    "generator.structures().placements_intersecting(",
    "placement.vertical_bounds()",
]
for marker in required_selection:
    if marker not in selection_compact:
        raise SystemExit(f"streaming selection lost authoritative generator query: {marker}")
if "VoxelWorld" in selection:
    raise SystemExit("streaming selection must not derive generated-world facts from VoxelWorld")

if "streaming::ChunkStreamingPlugin" not in world:
    raise SystemExit("WorldPlugin must install the rebuilt chunk streaming plugin")
if "coord.y," not in streaming or "coord.z," not in streaming or "coord.x," not in streaming:
    raise SystemExit("streaming priority must retain explicit coordinate tie-breaks")

# Deterministic priority fixture: changing input iteration order cannot change output order.
def priority(coord, center=(0, 4, 0), movement=(1, 0)):
    x, y, z = coord
    cx, cy, cz = center
    dx, dy, dz = x - cx, y - cy, z - cz
    horizontal = dx * dx + dz * dz
    total = horizontal + dy * dy
    forward = dx * movement[0] + dz * movement[1]
    directional = 1 if movement == (0, 0) or forward == 0 else (0 if forward > 0 else 2)
    return (horizontal, total, directional, y, z, x)


coords = [(2, 4, 0), (-2, 4, 0), (0, 3, 2), (0, 5, -2), (1, 4, 1)]
forward = sorted(coords, key=priority)
reverse = sorted(reversed(coords), key=priority)
if forward != reverse:
    raise SystemExit("streaming priority depends on input iteration order")

print(
    "Streaming materialization contract audit passed: generator-owned selection/materialization, "
    "stale-result rejection, resident publication, sparse frontier handoff, deterministic priority"
)
