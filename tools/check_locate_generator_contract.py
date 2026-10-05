#!/usr/bin/env python3
"""Fast CI audit for /locate ownership during the world-systems rebuild."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCATE_RS = ROOT / "src/hud/chat/locate.rs"
STRUCTURE_RS = ROOT / "src/world/generator/structure.rs"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"Locate generator contract audit failed: {message}")


def main() -> None:
    locate = LOCATE_RS.read_text(encoding="utf-8")
    structure = STRUCTURE_RS.read_text(encoding="utf-8")

    required_locate = {
        "locate consumes the immutable generator resource": "generator: Res<'w, WorldGenerator>",
        "biome locate uses the biome search capability": ".find_surface_biome(",
        "biome locate gets Y from the terrain capability": "generator.terrain().surface_at(x, z)",
        "structure locate uses authoritative nearest-root search": "queries.find_nearest(",
        "structure variation locate stays in the Structure owner": "queries.find_nearest_variant(",
        "large locate queries run off the main thread": "AsyncComputeTaskPool::get().spawn",
    }
    for description, fragment in required_locate.items():
        require(fragment in locate, description)

    forbidden_locate = {
        "VoxelWorld": "runtime voxel state",
        "WorldSeed": "raw seed access",
        "materialize_chunk(": "chunk materialization",
        "placements_intersecting(": "consumer-side structure area scanning",
        "chunk_coord": "chunk-grid traversal",
        "GenerationEntropy": "raw generation entropy",
        "BiomeLayout": "biome owner internals",
        "StructureField": "structure owner internals",
    }
    for fragment, description in forbidden_locate.items():
        require(fragment not in locate, f"locate must not depend on {description}: found {fragment}")

    required_structure = {
        "root-reference availability is owned by StructureField": "fn has_root_reference(&self, reference: &str) -> bool",
        "variation filtering shares nearest-root search": "fn find_nearest_matching(",
        "variation filtering uses the resolved authoritative root piece": "placement.structure_id() != structure_id",
    }
    for description, fragment in required_structure.items():
        require(fragment in structure, description)

    print("Locate generator contract audit passed: query capabilities only, no chunk/runtime scan")


if __name__ == "__main__":
    main()
