#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHUNK = ROOT / "src/world/generator/chunk.rs"
GENERATOR = ROOT / "src/world/generator.rs"

chunk = CHUNK.read_text(encoding="utf-8")
generator = GENERATOR.read_text(encoding="utf-8")

sample_markers = [
    "sample_solid_volume(",
    "sample_generated_fluid_volume(",
]
composition_markers = [
    "edit_initial_blocks(",
    "edit_initial_fluids(",
    "placements_intersecting(",
    "rasterize_structures(",
    "collect_generated_fluid_frontiers(",
]
missing = [
    marker
    for marker in sample_markers + composition_markers
    if marker not in chunk
]
if missing:
    raise SystemExit(f"chunk materializer is missing required stages: {missing}")

first_composition = chunk.index(composition_markers[0])
if any(chunk.index(marker) > first_composition for marker in sample_markers):
    raise SystemExit("dense solid/fluid samples must be resolved before VoxelChunk composition")

positions = [chunk.index(marker) for marker in composition_markers]
if positions != sorted(positions):
    raise SystemExit(
        "VoxelChunk composition must remain ordered blocks -> generated fluids -> "
        "planned Structures -> generated-fluid frontier"
    )

for marker in [
    "StructureReplacePolicy::Any",
    "StructureReplacePolicy::AirOnly",
    "StructureReplacePolicy::Terrain",
    "StructureFluidPolicy::Displace",
    "clear_above_positions",
    "visit_potential_fluid_frontier_sources",
    "content_at_local",
    "frontiers.sort_by",
    "frontiers.dedup_by",
]:
    if marker not in chunk:
        raise SystemExit(f"chunk materializer lost required Structure/frontier behavior: {marker}")

if "VoxelWorld" in chunk:
    raise SystemExit("chunk materializer must not depend on runtime VoxelWorld state")

for marker in ["mod chunk;", "new_runtime(", "materialize_chunk("]:
    if marker not in generator:
        raise SystemExit(f"WorldGenerator lost chunk materialization capability: {marker}")

if generator.index("let materials = Arc::new(MaterialField::new(") > generator.index(
    "let structures = Arc::new(StructureField::new("
):
    raise SystemExit("StructureField must consume the already-authoritative MaterialField")

if generator.index("let structures = Arc::new(StructureField::new(") > generator.index(
    "let chunk_materializer = chunk_runtime.map"
):
    raise SystemExit("ChunkMaterializer must consume finalized StructureField placements")

print(
    "Chunk materializer contract audit passed: dense samples -> blocks -> fluids -> "
    "Structures -> frontier, no VoxelWorld ownership"
)
