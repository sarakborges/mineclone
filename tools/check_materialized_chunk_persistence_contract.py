#!/usr/bin/env python3
"""Guard Phase 9 all-materialized-chunk persistence semantics."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PERSISTENCE = (ROOT / "src/voxel/world/persistence.rs").read_text(encoding="utf-8")
STORAGE = (ROOT / "src/world/chunk_storage.rs").read_text(encoding="utf-8")
DIMENSIONS = (ROOT / "src/world/save_catalog/dimensions.rs").read_text(encoding="utf-8")
UNLOADING = (ROOT / "src/world/chunk_unloading.rs").read_text(encoding="utf-8")
SNAPSHOT = (ROOT / "src/world/save_catalog/snapshot.rs").read_text(encoding="utf-8")
DISK = (ROOT / "src/voxel/chunk_disk.rs").read_text(encoding="utf-8")


def compact(source: str) -> str:
    return "".join(source.split())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"Materialized chunk persistence audit failed: {message}")


persistence = compact(PERSISTENCE)
storage = compact(STORAGE)
dimensions = compact(DIMENSIONS)
unloading = compact(UNLOADING)
snapshot = compact(SNAPSHOT)
disk = compact(DISK)

required_persistence = {
    "unloaded materialized chunks become persistent before archival": "archive_if_persistent(&mutself,coord:IVec3,chunk:&VoxelChunk){self.mark_persistent(coord);self.archived_chunks.insert(",
    "save enumeration includes every currently resident chunk": "coords.extend(self.resident.coords())",
    "save enumeration preserves already-persistent/archived coordinates": "self.persistence.persistent_coords().collect::<HashSet<_>>()",
    "resident unedited chunks can serialize without a mutation flag": "ifresident.is_none()&&!self.persistence.is_persistent(coord)",
    "loaded saved chunks remain persistence-owned": "self.persistence.insert_saved(coord,archived)",
}
for description, fragment in required_persistence.items():
    require(fragment in persistence, description)

require(
    "Untoucheddeterministicterrainisreconstructed" not in persistence,
    "old regenerate-untouched-chunks contract must not remain",
)
require(
    "Capturesonlychunkswithpersistentmutations" not in persistence,
    "save enumeration must not describe mutation-only persistence",
)

required_storage = {
    "generation save enumerates the world persistence boundary": "world.persistent_chunk_coords().collect()",
    "generation save serializes each enumerated coordinate": "|coord|world.save_persistent_chunk(coord,fluids)",
}
for description, fragment in required_storage.items():
    require(fragment in storage, description)

require(
    "eviction.world.archive_chunk(coord)" in unloading,
    "runtime eviction must hand resident chunks to the archival boundary",
)
require(
    "for(dimension_id,world)inworlds" in dimensions
    and "publish_generation_world_chunks(&directory,generation,world,fluids)?" in dimensions,
    "every active/inactive dimension world must publish through the same chunk persistence boundary",
)

# v7 described mutation-only spatial persistence. The all-materialized contract
# is intentionally forward-only and must not silently reinterpret those saves.
require(
    "constSAVE_FORMAT_VERSION:u32=8" in snapshot,
    "all-materialized spatial persistence requires save format v8",
)

# Empty chunks still produce a DiskChunk entry because coord is mandatory while
# content payloads are allowed to remain empty. This preserves explored empty
# space across future generator revisions.
require("structDiskChunk{coord:[i32;3]," in disk, "DiskChunk must always encode chunk identity")
require("fnnew(coord:IVec3)->Self{Self{coord:[coord.x,coord.y,coord.z]" in disk, "empty DiskChunk builder must retain coordinate identity")
require("Ok(builder.finish())" in disk, "chunk serialization must emit a DiskChunk even with no voxel payload")

# Small semantic fixture for the set union used by persistent_chunk_coords.
archived_or_mutated = {(2, 0, 3), (8, 1, -4)}
resident = {(0, 0, 0), (8, 1, -4), (9, 0, 9)}
all_materialized = archived_or_mutated | resident
require((0, 0, 0) in all_materialized, "unedited resident chunk must be included")
require((2, 0, 3) in all_materialized, "archived chunk must be included")
require(len(all_materialized) == 4, "resident/archive overlap must deduplicate")

print(
    "Materialized chunk persistence audit passed: resident + archived chunks, "
    "unedited/empty identity, multi-dimension publication, and save format v8"
)
