#!/usr/bin/env python3
"""Validate structure references without compiling or launching the game."""

from __future__ import annotations

import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data"


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"{path.relative_to(ROOT)}: {error}") from error


def definition_ids(directory: str) -> set[str]:
    root = DATA / directory
    ids: set[str] = set()
    for path in sorted(root.rglob("*.json")):
        definition = load_json(path)
        content_id = definition.get("id")
        if not isinstance(content_id, str) or not content_id:
            raise RuntimeError(f"{path.relative_to(ROOT)}: missing non-empty id")
        if content_id in ids:
            raise RuntimeError(f"duplicate {directory} id: {content_id}")
        ids.add(content_id)
    return ids


def require(
    errors: list[str],
    structure: str,
    location: str,
    content_id: object,
    known_ids: set[str],
    kind: str,
) -> None:
    if isinstance(content_id, str) and content_id in known_ids:
        return
    errors.append(
        f"{structure} {location} references missing {kind}: {content_id!r}"
    )


def main() -> int:
    blocks = definition_ids("blocks")
    fluids = definition_ids("fluids")
    layers = definition_ids("layers")
    objects = definition_ids("objects")
    errors: list[str] = []

    structure_definitions: list[tuple[Path, dict]] = []
    structure_ids: set[str] = set()
    structure_groups: set[str] = set()
    for path in sorted((DATA / "structures").rglob("*.json")):
        definition = load_json(path)
        structure_id = definition.get("id")
        if not isinstance(structure_id, str) or not structure_id:
            errors.append(f"{path.relative_to(ROOT)}: missing non-empty structure id")
            continue
        if structure_id in structure_ids:
            errors.append(f"duplicate structures id: {structure_id}")
            continue
        structure_ids.add(structure_id)
        group_id = definition.get("group_id")
        if isinstance(group_id, str) and group_id:
            if ":" in structure_id:
                namespace, _ = structure_id.split(":", 1)
                structure_groups.add(f"{namespace}:{group_id}")
            else:
                structure_groups.add(group_id)
        structure_definitions.append((path, definition))

    structure_references = structure_ids | structure_groups

    for path, definition in structure_definitions:
        structure = definition.get("id", str(path.relative_to(ROOT)))
        restrictions = definition.get("restrictions") or {}

        for block in restrictions.get("groundBlocks") or []:
            require(
                errors,
                structure,
                "restrictions.groundBlocks",
                block,
                blocks,
                "block",
            )

        for index, proximity in enumerate(restrictions.get("proximity") or []):
            target = (proximity or {}).get("target") or {}
            if "block" in target:
                require(
                    errors,
                    structure,
                    f"restrictions.proximity[{index}].target.block",
                    target["block"],
                    blocks,
                    "block",
                )
            if "fluid" in target:
                require(
                    errors,
                    structure,
                    f"restrictions.proximity[{index}].target.fluid",
                    target["fluid"],
                    fluids,
                    "fluid",
                )

        for symbol, entry in (definition.get("palette") or {}).items():
            if "block" in entry:
                require(
                    errors,
                    structure,
                    f"palette[{symbol!r}].block",
                    entry["block"],
                    blocks,
                    "block",
                )
            if "fluid" in entry:
                require(
                    errors,
                    structure,
                    f"palette[{symbol!r}].fluid",
                    entry["fluid"],
                    fluids,
                    "fluid",
                )
            if "object" in entry:
                require(
                    errors,
                    structure,
                    f"palette[{symbol!r}].object",
                    entry["object"],
                    objects,
                    "object",
                )
            connector = entry.get("connector")
            if isinstance(connector, dict) and "target" in connector:
                require(
                    errors,
                    structure,
                    f"palette[{symbol!r}].connector.target",
                    connector["target"],
                    structure_references,
                    "structure or structure group",
                )
            for index, surface in enumerate(entry.get("surfaceLayers") or []):
                require(
                    errors,
                    structure,
                    f"palette[{symbol!r}].surfaceLayers[{index}].layer",
                    (surface or {}).get("layer"),
                    layers,
                    "layer",
                )

    if errors:
        print("Structure content reference validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("Structure content references are valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
