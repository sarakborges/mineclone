#!/usr/bin/env python3
"""Fast CI audit for StructureField bounded-query determinism contracts.

This intentionally does not execute the game binary. It guards the authoritative
Rust owner shape directly, verifies the current authored probe families remain
present, and exercises the request-window algebra with representative complete
logical graphs. The heavier --structure-debug path remains available on demand.
"""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STRUCTURE_RS = ROOT / "src/world/generator/structure.rs"
OVERWORLD = ROOT / "data/dimensions/overworld/dimension.json"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"Structure query contract audit failed: {message}")


def audit_authoritative_owner_shape() -> None:
    source = STRUCTURE_RS.read_text(encoding="utf-8")
    required_fragments = {
        "bounded queries collect world-space candidates":
            "self.collect_candidates_intersecting_bounds(query_minimum, query_maximum);",
        "bounded queries arbitrate before piece filtering":
            "for candidate in self.resolve_conflicts(direct_candidates)",
        "bounded queries filter pieces only after planning":
            "rectangles_overlap(minimum, maximum, query_minimum, query_maximum)",
        "bounded query results have an explicit stable sort":
            "placements.sort_by(|left, right| placement_sort_key(left).cmp(&placement_sort_key(right)))",
        "candidate collection uses complete logical bounds":
            "let (placement_minimum, placement_maximum) = candidate.horizontal_bounds();",
        "nearest distance uses the logical placement anchor":
            "let delta = candidate.placement_anchor - origin;",
        "nearest lookup reuses accepted arbitration":
            "!self.candidate_is_accepted(&candidate)",
        "nearest ties use the same deterministic placement key":
            "placement_sort_key(&placement) < placement_sort_key(current)",
        "accepted candidates inspect overlapping competitors":
            "self.collect_candidates_intersecting_bounds(minimum, maximum)",
    }
    for description, fragment in required_fragments.items():
        require(fragment in source, description)

    start = source.index("pub(super) struct StructureField")
    end = source.index("struct RootStructureRule", start)
    field_block = source[start:end]
    for forbidden in ("Mutex<", "RwLock<", "RefCell<", "Cell<", "UnsafeCell<", "Atomic"):
        require(forbidden not in field_block, f"StructureField must remain immutable; found {forbidden}")


def audit_current_authored_probe_families() -> None:
    dimension = json.loads(OVERWORLD.read_text(encoding="utf-8"))
    rules = dimension.get("generatedSurfaceStructures", [])

    def matching(reference: str) -> list[dict[str, object]]:
        return [rule for rule in rules if rule.get("structure") == reference]

    require(matching("asteria:tree_oak"), "ordinary root probe asteria:tree_oak is no longer authored")
    require(matching("asteria:enchanted_heart"), "StructureSet probe asteria:enchanted_heart is no longer authored")
    require(matching("asteria:lake"), "lake connector probe is no longer authored")

    mouth = matching("asteria:river_ocean_mouth")
    require(len(mouth) == 1, "river_ocean_mouth must have exactly one authored root rule")
    require(mouth[0].get("placement") == "biomeMargin", "river_ocean_mouth must remain a biomeMargin root")

    waterfall_biomes = {str(rule.get("biome")) for rule in matching("asteria:mountain_waterfall")}
    require(
        waterfall_biomes
        == {
            "asteria:overworld/mountains",
            "asteria:overworld/alps",
            "asteria:overworld/mountain_belt",
        },
        "mountain waterfall probe roots changed unexpectedly",
    )


@dataclass(frozen=True, order=True)
class Rect:
    min_x: int
    min_z: int
    max_x: int
    max_z: int

    def __post_init__(self) -> None:
        require(self.min_x <= self.max_x and self.min_z <= self.max_z, "fixture rectangle is inverted")

    def overlaps(self, other: "Rect") -> bool:
        return (
            self.min_x <= other.max_x
            and self.max_x >= other.min_x
            and self.min_z <= other.max_z
            and self.max_z >= other.min_z
        )


@dataclass(frozen=True, order=True)
class Piece:
    root_reference: str
    piece_id: str
    bounds: Rect


def query(pieces: tuple[Piece, ...] | list[Piece], window: Rect) -> tuple[Piece, ...]:
    return tuple(sorted(piece for piece in pieces if piece.bounds.overlaps(window)))


def validate_axis(name: str, pieces: tuple[Piece, ...], axis: str) -> None:
    if axis == "x":
        first_window = Rect(-64, -64, 16, 64)
        second_window = Rect(-16, -64, 64, 64)
        overlap = Rect(-16, -64, 16, 64)
    else:
        first_window = Rect(-64, -64, 64, 16)
        second_window = Rect(-64, -16, 64, 64)
        overlap = Rect(-64, -16, 64, 16)

    first = query(pieces, first_window)
    second = query(pieces, second_window)
    second_repeat = query(pieces, second_window)
    first_repeat = query(pieces, first_window)

    require(first == first_repeat, f"{name}/{axis}: first window changed after reverse query order")
    require(second == second_repeat, f"{name}/{axis}: second window changed after reverse query order")

    first_overlap = query(list(first), overlap)
    second_overlap = query(list(second), overlap)
    require(first_overlap, f"{name}/{axis}: fixture does not actually exercise an overlap seam")
    require(first_overlap == second_overlap, f"{name}/{axis}: overlapping windows disagree")


def audit_graph_window_algebra() -> None:
    graphs: dict[str, tuple[Piece, ...]] = {
        "ordinary-root": (
            Piece("asteria:tree_oak", "tree", Rect(-4, -4, 4, 4)),
        ),
        "structure-set": (
            Piece("asteria:enchanted_heart", "world_tree", Rect(-7, -7, 7, 7)),
            Piece("asteria:enchanted_heart", "spring_west", Rect(-28, -5, -18, 5)),
            Piece("asteria:enchanted_heart", "spring_east", Rect(18, -5, 28, 5)),
            Piece("asteria:enchanted_heart", "tree_north", Rect(-4, 18, 4, 28)),
        ),
        "ocean-mouth-river": (
            Piece("asteria:river_ocean_mouth", "mouth", Rect(-6, -6, 6, 6)),
            Piece("asteria:river_ocean_mouth", "river_1", Rect(4, -3, 24, 3)),
            Piece("asteria:river_ocean_mouth", "river_2", Rect(20, -3, 42, 10)),
            Piece("asteria:river_ocean_mouth", "river_turn", Rect(36, 8, 44, 30)),
        ),
        "lake-river": (
            Piece("asteria:lake", "lake", Rect(-12, -12, 12, 12)),
            Piece("asteria:lake", "river_1", Rect(10, -3, 34, 3)),
            Piece("asteria:lake", "river_turn", Rect(30, -3, 38, 24)),
        ),
        "waterfall-pond-river": (
            Piece("asteria:mountain_waterfall", "waterfall", Rect(-5, -9, 5, 9)),
            Piece("asteria:mountain_waterfall", "mountain_pond", Rect(3, -11, 19, 11)),
            Piece("asteria:mountain_waterfall", "river_1", Rect(16, -3, 40, 3)),
            Piece("asteria:mountain_waterfall", "river_turn", Rect(36, -3, 44, 24)),
        ),
    }
    for name, pieces in graphs.items():
        validate_axis(name, pieces, "x")
        validate_axis(name, pieces, "z")


@dataclass(frozen=True)
class RootCandidate:
    key: str
    anchor_x: int
    anchor_z: int
    representative_x: int
    representative_z: int


def root_distance_squared(candidate: RootCandidate, origin_x: int, origin_z: int) -> int:
    dx = candidate.anchor_x - origin_x
    dz = candidate.anchor_z - origin_z
    return dx * dx + dz * dz


def nearest(candidates: tuple[RootCandidate, ...], origin_x: int, origin_z: int) -> RootCandidate:
    return min(candidates, key=lambda candidate: (root_distance_squared(candidate, origin_x, origin_z), candidate.key))


def audit_nearest_anchor_contract() -> None:
    candidates = (
        RootCandidate("far-root-close-child", 50, 0, 1, 0),
        RootCandidate("near-root-far-child", 10, 0, 100, 0),
    )
    selected = nearest(candidates, 0, 0)
    require(
        selected.key == "near-root-far-child",
        "nearest selection must use logical root anchor, not representative/connector-piece origin",
    )

    tied = (
        RootCandidate("b", 10, 0, -100, 0),
        RootCandidate("a", -10, 0, 100, 0),
    )
    require(nearest(tied, 0, 0).key == "a", "nearest equal-distance ties must be deterministic")


def main() -> None:
    audit_authoritative_owner_shape()
    audit_current_authored_probe_families()
    audit_graph_window_algebra()
    audit_nearest_anchor_contract()
    print("Structure query contract audit passed: owner shape + authored probes + seam/order fixtures")


if __name__ == "__main__":
    main()
