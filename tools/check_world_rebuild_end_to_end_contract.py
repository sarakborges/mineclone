#!/usr/bin/env python3
"""Cross-owner Phase 12 audit for the rebuilt world pipeline.

This intentionally does not reimplement generation. It verifies that the same
immutable query/materialization owners flow through streaming, persistence,
loading, warp and locate, and that authored fixture families required for visual
validation remain present.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GENERATOR = (ROOT / "src/world/generator.rs").read_text(encoding="utf-8")
CHUNK = (ROOT / "src/world/generator/chunk.rs").read_text(encoding="utf-8")
SELECTION = (ROOT / "src/world/streaming/selection.rs").read_text(encoding="utf-8")
MATERIALIZATION = (ROOT / "src/world/streaming/generation.rs").read_text(encoding="utf-8")
PERSISTENCE = (ROOT / "src/voxel/world/persistence.rs").read_text(encoding="utf-8")
LOADING = (ROOT / "src/world/loading.rs").read_text(encoding="utf-8")
DESTINATION = (ROOT / "src/world/destination.rs").read_text(encoding="utf-8")
WARP = (ROOT / "src/world/warp.rs").read_text(encoding="utf-8")
LOCATE = (ROOT / "src/hud/chat/locate.rs").read_text(encoding="utf-8")
MAIN = (ROOT / "src/main.rs").read_text(encoding="utf-8")


def compact(source: str) -> str:
    return "".join(source.split())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"World rebuild end-to-end audit failed: {message}")


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


generator = compact(GENERATOR)
chunk = compact(CHUNK)
selection = compact(SELECTION)
materialization = compact(MATERIALIZATION)
persistence = compact(PERSISTENCE)
loading = compact(LOADING)
destination = compact(DESTINATION)
warp = compact(WARP)
locate = compact(LOCATE)
main = compact(MAIN)

# One immutable generated-world owner must feed both query-only consumers and
# runtime chunk synthesis. No Phase 12 fixture is allowed to introduce a second
# world-generation implementation.
for description, fragment in {
    "runtime generator freezes all semantic owners": "pub(crate)fnnew_runtime(",
    "biome queries come from the generator owner": "pub(crate)fnbiomes(&self)->BiomeQueries",
    "terrain queries come from the generator owner": "pub(crate)fnterrain(&self)->TerrainQueries",
    "Structure queries come from the generator owner": "pub(crate)fnstructures(&self)->StructureQueries",
    "chunk synthesis is a generator capability": "pub(crate)fnmaterialize_chunk(&self,coord:IVec3)->MaterializedChunk",
}.items():
    require(fragment in generator, description)

# Streaming selection derives residency from authoritative generated terrain and
# Structure queries, then async materialization calls the same WorldGenerator.
for description, fragment in {
    "streaming vertical selection samples generator terrain": "generator.terrain().sample_surface_area(",
    "streaming includes authoritative Structure reach": "generator.structures().placements_intersecting(",
}.items():
    require(fragment in selection, description)

for description, fragment in {
    "async work clones the immutable generator": "letgenerator=generator.clone();",
    "async work invokes authoritative chunk synthesis": "generator.materialize_chunk(coord)",
    "persisted chunks restore before generated publication": "ifruntime.world.restore_chunk(coord)",
    "new generated chunks publish through VoxelWorld": ".insert_chunk(coord,completed.output.into_chunk());",
}.items():
    require(fragment in materialization, description)

# Chunk synthesis remains world-coordinate anchored and bounds checked, including
# far coordinates. Request/chunk order must not become semantic input.
require(
    "GenerationChunkCoord::new(runtime_coord.x,runtime_coord.y,runtime_coord.z).world_bounds()"
    in chunk,
    "chunk synthesis must derive checked world bounds from the requested coordinate",
)
require(
    "generationorder" not in chunk.lower(),
    "chunk synthesis must not encode generation-order state",
)

# Materialization is the persistence boundary: resident chunks are save-visible,
# archived materialized chunks retain identity/content, and never-materialized
# coordinates are rejected by the spatial save API.
for description, fragment in {
    "resident materialized chunks participate in persistent enumeration": "coords.extend(self.resident.coords());",
    "archiving materialized chunks preserves them": "archive_if_persistent",
    "saved chunks restore into persistence ownership": "self.persistence.insert_saved(coord,archived);",
    "never-materialized chunks cannot be serialized as spatial state": "chunk{coord:?}wasnevermaterialized",
}.items():
    require(fragment in persistence, description)

# Loading and warp share generator-backed destination preparation and the same
# Loading state transition. Query-only destination/locate paths must not
# materialize chunks as a side effect.
for description, fragment in {
    "loading spawn selection consumes generator queries": "find_safe_spawn_position(&context.generator,IVec2::ZERO,|_|true)",
    "loading publishes authoritative residency counts": "streaming.desired_residency_counts(&world)",
    "loading enters Gameplay only after the required residency gate": "next_state.set(GameState::Gameplay);",
}.items():
    require(fragment in loading, description)

for description, fragment in {
    "generated destination search consumes immutable generator": "find_generated_surface_destination(generator:&WorldGenerator",
    "far-coordinate search bounds are saturating": "center.saturating_sub(radius)",
    "far-coordinate search width uses widened arithmetic": "i64::from(maximum)-i64::from(minimum)+1",
}.items():
    require(fragment in destination, description)

for fragment, consumer in {
    "materialize_chunk(": "destination query",
    "VoxelWorld": "destination query",
}.items():
    require(fragment not in DESTINATION, f"{consumer} must stay query-only: found {fragment}")

require(
    "dimension.next_game_state.set(GameState::Loading);" in WARP,
    "dimension travel must re-enter the shared Loading pipeline",
)
require(
    "find_generated_surface_destination(&dimension.generator" in warp,
    "warp must use the shared generated destination query",
)
require(
    "generator:Res<'w,WorldGenerator>" in locate,
    "/locate must consume the installed immutable generator",
)
require(
    "VoxelWorld" not in LOCATE and "materialize_chunk(" not in LOCATE,
    "/locate must remain query-only and must not scan/materialize runtime chunks",
)

# The existing on-demand visual diagnostics are part of Phase 12 fixture
# coverage. They must stay seed/dimension/coordinate addressable so near/far,
# multi-seed and multi-dimension fixtures can be reproduced without entering the
# game runtime.
for flag in ("--biome-map", "--terrain-debug", "--structure-debug"):
    require(flag in MAIN, f"missing on-demand diagnostic CLI {flag}")
for flag in ("--seed", "--dimension", "--center-x", "--center-z"):
    require(flag in MAIN, f"diagnostic CLIs must remain addressable by {flag}")

# Ensure the authored fixture families named by the Phase 12 design still exist.
dimension_files = sorted((ROOT / "data/dimensions").glob("*/dimension.json"))
require(dimension_files, "no authored dimensions found")
dimensions = [load_json(path) for path in dimension_files]
dimension_ids = {definition.get("id") for definition in dimensions}
require("asteria:overworld" in dimension_ids, "Overworld fixture dimension is missing")
require("asteria:umbral" in dimension_ids, "Umbral fixture dimension is missing")
require(
    any(definition.get("generatedOcean") for definition in dimensions),
    "Ocean/generated-coast fixture authoring is missing",
)
require(
    any(definition.get("generatedSurfaceStructures") for definition in dimensions),
    "Structure-heavy fixture authoring is missing",
)

biome_files = sorted((ROOT / "data/dimensions").glob("*/biomes/*.json"))
require(biome_files, "no authored biome fixtures found")
biomes = [load_json(path) for path in biome_files]
require(
    any((biome.get("surfaceLayout") or {}).get("cannotBorder") for biome in biomes),
    "cannotBorder fixture authoring is missing",
)
require(
    any((biome.get("terrain3d") or {}).get("floatingFormation") for biome in biomes),
    "floating-terrain fixture authoring is missing",
)

print(
    "World rebuild end-to-end audit passed: immutable queries -> streaming selection -> "
    "chunk materialization -> persistent spatial state -> shared loading/destination consumers, "
    "with reproducible Overworld/Umbral visual fixture families"
)
