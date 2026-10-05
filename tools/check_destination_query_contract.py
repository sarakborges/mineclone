#!/usr/bin/env python3
"""Guard generated spawn queries vs mutable runtime destination checks."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLAYER = (ROOT / "src/player/mod.rs").read_text(encoding="utf-8")
WARP = (ROOT / "src/world/warp.rs").read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"Destination query contract audit failed: {message}")


required_player = {
    "generated spawn accepts the immutable generator": "generator: &WorldGenerator",
    "spawn search preplans Structure occupancy once": ".placements_intersecting(origin_x, origin_z, width, depth)",
    "spawn grounding uses authoritative Terrain": "generator.terrain().surface_at(column.x, column.y)",
    "spawn support/head checks use generated Material": "let materials = generator.materials();",
    "spawn rejects generated fluids": ".generated_fluid_at(column.x, y, column.y)",
    "runtime player-clear checks remain runtime-owned": "pub(crate) fn player_position_is_clear(world: &VoxelWorld",
}
for description, fragment in required_player.items():
    require(fragment in PLAYER, description)

for fragment, description in {
    "highest_loaded_world_y_in_column": "loaded-column scanning for generated spawn height",
    "materialize_chunk(": "chunk materialization for a generated spawn query",
    "WorldSeed": "raw seed reconstruction in the spawn consumer",
}.items():
    require(fragment not in PLAYER, f"player spawn must not use {description}: found {fragment}")

required_warp = {
    "warp safe-position search reads mutable runtime state": "fn advance_safe_eye_position_search(\n    world: &VoxelWorld",
    "warp blocks on nonresident runtime voxels": "if !world.is_loaded_at(voxel)",
    "warp observes current runtime fluids": "if world.fluid_at(voxel).is_some()",
    "warp observes current collision state": "collides_aabb(world, bounds.0, bounds.1)",
}
for description, fragment in required_warp.items():
    require(fragment in WARP, description)

require(
    "WorldGenerator" not in WARP,
    "coordinate warp safety must not ignore runtime mutations by switching to generated-only truth",
)

print(
    "Destination query contract audit passed: generated spawn uses WorldGenerator; "
    "runtime warp/player-clear safety stays VoxelWorld-owned"
)
