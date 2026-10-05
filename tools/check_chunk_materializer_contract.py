#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHUNK = ROOT / "src/world/generator/chunk.rs"
GENERATOR = ROOT / "src/world/generator.rs"

chunk = CHUNK.read_text(encoding="utf-8")
generator = GENERATOR.read_text(encoding="utf-8")

required_chunk_markers = [
    "sample_solid_volume(",
    "edit_initial_blocks(",
    "sample_generated_fluid_volume(",
    "edit_initial_fluids(",
    "placements_intersecting(",
    "rasterize_structures(",
    "collect_generated_fluid_frontiers(",
]
missing = [marker for marker in required_chunk_markers if marker not in chunk]
if missing:
    raise SystemExit(f"chunk materializer is missing required stages: {missing}")

positions = [chunk.index(marker) for marker in required_chunk_markers]
if positions != sorted(positions):
    raise SystemExit(
        "chunk materialization stages must remain ordered solids -> generated fluids -> "
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
    "Chunk materializer contract audit passed: solids -> fluids -> Structures -> frontier, "
    "no VoxelWorld ownership"
)
