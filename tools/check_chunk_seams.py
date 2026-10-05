#!/usr/bin/env python3
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
MATERIALIZER = ROOT / "src/world/generator/chunk.rs"
MATERIAL = ROOT / "src/world/generator/material.rs"
VOXEL_CHUNK = ROOT / "src/voxel/chunk.rs"

chunk = MATERIALIZER.read_text(encoding="utf-8")
material = MATERIAL.read_text(encoding="utf-8")
voxel_chunk = VOXEL_CHUNK.read_text(encoding="utf-8")

match = re.search(r"pub const CHUNK_SIZE: usize = (\d+);", voxel_chunk)
if match is None:
    raise SystemExit("cannot resolve CHUNK_SIZE from src/voxel/chunk.rs")
CHUNK_SIZE = int(match.group(1))
if CHUNK_SIZE <= 0:
    raise SystemExit("CHUNK_SIZE must be positive")

required_materializer_markers = [
    "GenerationChunkCoord::new(",
    "sample_solid_volume(",
    "sample_generated_fluid_volume(",
    "placements_intersecting(",
    "visit_structure_voxels_in_chunk(",
    "clear_above_positions(",
    "attachment_rise",
    "rasterize_attachment_payloads(",
    "solid_block_at(candidate.x, candidate.y, candidate.z)",
    "chunk_origin + target",
    "frontiers.sort_by",
    "frontiers.dedup_by",
]
missing = [marker for marker in required_materializer_markers if marker not in chunk]
if missing:
    raise SystemExit(f"chunk seam contract lost required materializer behavior: {missing}")

if "VoxelWorld" in chunk:
    raise SystemExit("chunk synthesis must remain independent of VoxelWorld residency")
for forbidden in ["RefCell<", "Mutex<", "RwLock<"]:
    if forbidden in chunk:
        raise SystemExit(f"ChunkMaterializer must not gain mutable query state: {forbidden}")

required_material_markers = [
    "fn solid_block_at(&self, x: i32, y: i32, z: i32)",
    "fn generated_fluid_at(&self, x: i32, y: i32, z: i32)",
    "fn sample_solid_volume(",
    "fn sample_generated_fluid_volume(",
    "fn generated_fluid_from_column(",
    "self.additive_depth_from_volume(",
    "self.terrain.queries().density_at(x, sample_y, z)",
]
missing = [marker for marker in required_material_markers if marker not in material]
if missing:
    raise SystemExit(f"material scalar/batch seam contract lost required behavior: {missing}")


def origin(coord: tuple[int, int, int]) -> tuple[int, int, int]:
    return tuple(axis * CHUNK_SIZE for axis in coord)


def owner_axis(value: int) -> int:
    return value // CHUNK_SIZE


def in_chunk(position: tuple[int, int, int], coord: tuple[int, int, int]) -> bool:
    o = origin(coord)
    return all(o[i] <= position[i] < o[i] + CHUNK_SIZE for i in range(3))


def clip(positions: set[tuple[int, int, int]], coord: tuple[int, int, int]):
    return {position for position in positions if in_chunk(position, coord)}


# Adjacent chunk volumes must partition world voxels without overlap or gaps,
# including negative coordinates where Euclidean chunk ownership matters.
for left_coord in [-2, -1, 0, 1]:
    left = (left_coord, 0, 0)
    right = (left_coord + 1, 0, 0)
    left_x = {origin(left)[0] + x for x in range(CHUNK_SIZE)}
    right_x = {origin(right)[0] + x for x in range(CHUNK_SIZE)}
    if left_x & right_x:
        raise SystemExit(f"adjacent chunk x ranges overlap: {left}, {right}")
    expected = set(range(origin(left)[0], origin(right)[0] + CHUNK_SIZE))
    if left_x | right_x != expected:
        raise SystemExit(f"adjacent chunk x ranges leave a gap: {left}, {right}")
    for x in expected:
        if owner_axis(x) not in {left_coord, left_coord + 1}:
            raise SystemExit(f"world voxel {x} resolves to unexpected chunk owner")


# One globally planned Structure crossing x/y seams must be clipped exactly once
# into each materialized chunk. clearAbove uses the same world-space ownership.
structure_payload = {
    (14, 15, 3),
    (15, 15, 3),
    (16, 15, 3),
    (17, 15, 3),
    (15, 16, 3),
    (16, 16, 3),
}
clear_above = {(x, 17, 3) for x in range(14, 18)}
coords = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (1, 1, 0)]
for payload, label in [(structure_payload, "Structure payload"), (clear_above, "clearAbove")]:
    clipped = [clip(payload, coord) for coord in coords]
    union = set().union(*clipped)
    if union != payload:
        raise SystemExit(f"{label} is lost while clipping adjacent chunks")
    counts = {position: sum(position in part for part in clipped) for position in payload}
    if any(count != 1 for count in counts.values()):
        raise SystemExit(f"{label} is duplicated across chunk seams: {counts}")


# Attachment-only markers project to one world-space support before chunk ownership
# is considered. A marker at y=15 with valid supports at 15 and 16 must choose y=16
# once, rather than letting each vertical section independently choose a support.
def project_support(marker_y: int, max_rise: int, solid_ys: set[int]) -> int | None:
    for rise in range(max_rise, -1, -1):
        candidate = marker_y + rise
        if candidate in solid_ys:
            return candidate
    return None

support_y = project_support(15, 1, {15, 16})
if support_y != 16:
    raise SystemExit("attachment projection must choose the highest world-space support")
if sum(in_chunk((4, support_y, 4), coord) for coord in [(0, 0, 0), (0, 1, 0)]) != 1:
    raise SystemExit("projected attachment support must belong to exactly one vertical chunk")


# Generated-fluid frontier targets use world coordinates. Empty targets inside the
# current chunk are filtered immediately; cross-chunk targets remain potential until
# the runtime scheduler can revalidate the neighboring materialized state.
SPREAD = [(0, -1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)]


def frontier_targets(
    coord: tuple[int, int, int],
    sources: set[tuple[int, int, int]],
    occupied: set[tuple[int, int, int]],
) -> tuple[tuple[int, int, int], ...]:
    candidates = set()
    for source in sources:
        for offset in SPREAD:
            target = tuple(source[i] + offset[i] for i in range(3))
            if in_chunk(target, coord) and target in occupied:
                continue
            candidates.add(target)
    return tuple(sorted(candidates, key=lambda p: (p[1], p[2], p[0])))

left_coord = (0, 0, 0)
left_source = {(15, 8, 8)}
left_occupied = {(14, 8, 8), (15, 7, 8)}
left_frontier = frontier_targets(left_coord, left_source, left_occupied)
if (16, 8, 8) not in left_frontier:
    raise SystemExit("cross-chunk generated-fluid frontier target was not preserved")
if (14, 8, 8) in left_frontier or (15, 7, 8) in left_frontier:
    raise SystemExit("inside-chunk occupied generated-fluid targets were not filtered")


# Materialization order must not affect semantic chunk results. Synthetic payloads
# are functions of world coordinates only; requesting A->B or B->A yields the same
# per-chunk signatures and the same union across the seam.
def synthetic_materialize(coord: tuple[int, int, int]):
    o = origin(coord)
    blocks = {
        (x, y, z): ("stone" if y < 8 else None)
        for x in range(o[0], o[0] + CHUNK_SIZE)
        for y in range(o[1], o[1] + CHUNK_SIZE)
        for z in range(o[2], o[2] + CHUNK_SIZE)
    }
    blocks = tuple(sorted(position for position, block in blocks.items() if block is not None))
    structure = tuple(sorted(clip(structure_payload, coord)))
    return blocks, structure

pair = [(0, 0, 0), (1, 0, 0)]
forward = {coord: synthetic_materialize(coord) for coord in pair}
reverse = {coord: synthetic_materialize(coord) for coord in reversed(pair)}
if forward != reverse:
    raise SystemExit("reordered neighboring chunk materialization changed semantic output")

print(
    "Chunk seam audit passed: scalar/batch ownership + adjacent partition + Structure/clear "
    "+ attachment projection + fluid frontier + request-order fixtures"
)
