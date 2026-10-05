#!/usr/bin/env python3
"""Guard the shared generated-destination query + small runtime validation contract."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLAYER = (ROOT / "src/player/mod.rs").read_text(encoding="utf-8")
DESTINATION = (ROOT / "src/world/destination.rs").read_text(encoding="utf-8")
WARP = (ROOT / "src/world/warp.rs").read_text(encoding="utf-8")
STREAMING = (ROOT / "src/world/streaming.rs").read_text(encoding="utf-8")


def compact(source: str) -> str:
    return "".join(source.split())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"Destination query contract audit failed: {message}")


player = compact(PLAYER)
destination = compact(DESTINATION)
warp = compact(WARP)
streaming = compact(STREAMING)

required_destination = {
    "shared destination query accepts immutable generator": "find_generated_surface_destination(generator:&WorldGenerator",
    "destination query preplans Structure occupancy once": ".placements_intersecting(origin_x,origin_z,width,depth)",
    "destination grounding uses authoritative Terrain": "generator.terrain().surface_at(column.x,column.y)",
    "destination support/head checks use generated Material": "letmaterials=generator.materials()",
    "destination rejects generated fluids": ".generated_fluid_at(column.x,y,column.y)",
}
for description, fragment in required_destination.items():
    require(fragment in destination, description)

for fragment, description in {
    "VoxelWorld": "mutable runtime world state",
    "materialize_chunk(": "chunk materialization",
    "WorldSeed": "raw seed reconstruction",
    "highest_loaded_world_y_in_column": "loaded-column scanning",
}.items():
    require(fragment not in DESTINATION, f"generated destination query must not use {description}: found {fragment}")

required_player = {
    "spawn delegates to the shared generated destination primitive": "find_generated_surface_destination(generator,preferred_column,SPAWN_SEARCH_RADIUS_BLOCKS,accepts_column,",
    "runtime player-clear checks remain runtime-owned": "pub(crate)fnplayer_position_is_clear(world:&VoxelWorld",
}
for description, fragment in required_player.items():
    require(fragment in player, description)

required_warp = {
    "warp owns the immutable generator for query-guided fallback": "generator:Res<'w,WorldGenerator>",
    "warp uses the shared generated destination primitive": "find_generated_surface_destination(&dimension.generator,target.xz(),WARP_QUERY_RADIUS_BLOCKS",
    "warp validates the requested XYZ before generated fallback": "matchcandidate_state(dimension.runtime.world(),target)",
    "warp runtime validation is intentionally small": "constWARP_RUNTIME_VALIDATION_RADIUS_BLOCKS:i32=2",
    "warp runtime validation reads current residency": "if!world.is_loaded_at(voxel)",
    "warp runtime validation observes current fluids": "ifworld.fluid_at(voxel).is_some()",
    "warp runtime validation observes current collision state": "collides_aabb(world,bounds.0,bounds.1)",
    "prepared destination redirects streaming": "self.prepared_surface.or(self.target)",
    "warp requests only a minimal horizontal neighborhood": "constWARP_STREAMING_HORIZONTAL_RADIUS_CHUNKS:i32=1",
}
for description, fragment in required_warp.items():
    require(fragment in warp, description)

for fragment, description in {
    "WARP_SEARCH_RADIUS_BLOCKS": "legacy large runtime BFS radius",
    "WARP_SEARCH_FRAME_BUDGET": "legacy frame-budgeted brute-force runtime search",
    "WARP_SEARCH_BUDGET_CHECK_INTERVAL": "legacy brute-force runtime budget loop",
}.items():
    require(fragment not in WARP, f"warp must not restore {description}: found {fragment}")

require(
    "pending_warp.streaming_horizontal_radius()" in streaming,
    "streaming must honor the minimal destination-preparation radius during warp",
)

print(
    "Destination query contract audit passed: shared generator query for spawn/warp, "
    "direct destination streaming, and small mutable-runtime validation"
)
