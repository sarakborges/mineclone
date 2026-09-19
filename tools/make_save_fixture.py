#!/usr/bin/env python3
"""Make an isolated, intentionally damaged copy of a disposable Asteria save.

Never edits the source world or an existing output tree. Run the game against a
copy of the fixture only after backing up worlds/ with the game closed.
"""

import argparse
import copy
import json
from pathlib import Path
import re
import shutil
import sys

MAX_FIXTURE_SNAPSHOT_BYTES = 64 * 1024 * 1024
MANIFEST_PATTERN = re.compile(r"manifest-(\d{20})\.json\Z")
DAMAGES = (
    "snapshot-json",
    "manifest-json",
    "inventory-id",
    "clock",
    "duplicate-chunk",
    "player-null",
)


def read_json(path: Path, maximum: int) -> dict:
    if not path.is_file() or path.is_symlink():
        raise ValueError(f"Not a regular fixture file: {path}")
    if path.stat().st_size > maximum:
        raise ValueError(f"Fixture is too large for the safe generator limit: {path}")
    with path.open("r", encoding="utf-8") as stream:
        value = json.load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"Expected a JSON object: {path}")
    return value


def complete_generations(world: Path) -> list[tuple[int, Path, Path, dict]]:
    manifests = []
    for path in world.iterdir():
        match = MANIFEST_PATTERN.fullmatch(path.name)
        if match and int(match.group(1)) > 0:
            manifests.append((int(match.group(1)), path))
    manifests.sort(reverse=True)
    if len(manifests) < 2:
        raise ValueError("Create at least two committed save generations before making a fallback fixture")
    generations = []
    for number, manifest_path in manifests[:2]:
        manifest = read_json(manifest_path, 64 * 1024)
        expected = f"snapshot-{number:020}.json"
        if (manifest.get("generation") != number or manifest.get("id") != world.name
                or manifest.get("snapshot_file") != expected):
            raise ValueError(f"Invalid manifest in source fixture: {manifest_path}")
        snapshot_path = world / expected
        snapshot = read_json(snapshot_path, MAX_FIXTURE_SNAPSHOT_BYTES)
        if snapshot.get("id") != world.name:
            raise ValueError(f"Snapshot ID mismatch: {snapshot_path}")
        generations.append((number, manifest_path, snapshot_path, snapshot))
    return generations


def damaged_payload(snapshot: dict, damage: str) -> dict:
    result = copy.deepcopy(snapshot)
    if damage == "inventory-id":
        inventory = result.get("inventory")
        if not isinstance(inventory, list) or len(inventory) != 36:
            raise ValueError("Source snapshot does not have a 36-slot inventory")
        inventory[0] = "__asteria_invalid_fixture_item__"
    elif damage == "clock":
        result["tick_in_day"] = (1 << 64) - 1
    elif damage == "duplicate-chunk":
        chunks = result.get("chunks")
        if not isinstance(chunks, list) or not chunks:
            raise ValueError("Edit at least one chunk before generating a duplicate-chunk fixture")
        chunks.append(copy.deepcopy(chunks[0]))
    elif damage == "player-null":
        result["player"] = None
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source_world", type=Path, help="Disposable original worlds/<id> directory")
    parser.add_argument("output_root", type=Path, help="NEW output directory; must not exist")
    parser.add_argument("--damage", required=True, choices=DAMAGES)
    args = parser.parse_args()

    if args.source_world.is_symlink() or not args.source_world.is_dir():
        raise ValueError("Source must be an existing real world directory, not a symlink")
    source = args.source_world.resolve(strict=True)
    output_root = args.output_root.resolve(strict=False)
    if (output_root.exists() or output_root == source
            or output_root.is_relative_to(source) or source.is_relative_to(output_root)):
        raise ValueError("Output must be a new directory outside the source world")
    if any(entry.is_symlink() for entry in source.rglob("*")):
        raise ValueError("Refusing to copy a source world containing symlinks")

    generations = complete_generations(source)
    number, manifest_path, snapshot_path, snapshot = generations[0]
    if args.damage not in ("snapshot-json", "manifest-json"):
        snapshot = damaged_payload(snapshot, args.damage)

    destination = output_root / "worlds" / source.name
    destination.parent.mkdir(parents=True, exist_ok=False)
    shutil.copytree(source, destination)
    damaged_file = destination / (manifest_path.name if args.damage == "manifest-json" else snapshot_path.name)
    if args.damage in ("snapshot-json", "manifest-json"):
        damaged_file.write_bytes(b"{ deliberately malformed fixture\n")
    else:
        temporary = damaged_file.with_name(damaged_file.name + ".fixture.tmp")
        with temporary.open("w", encoding="utf-8") as stream:
            json.dump(snapshot, stream, ensure_ascii=False, separators=(",", ":"))
            stream.write("\n")
        temporary.replace(damaged_file)
    print(f"Fixture: {destination}")
    print(f"Damaged generation: {number}, mode: {args.damage}")
    print("The source was not modified. Keep the game closed while swapping a disposable fixture.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError) as error:
        print(f"Fixture generation failed: {error}", file=sys.stderr)
        sys.exit(1)
