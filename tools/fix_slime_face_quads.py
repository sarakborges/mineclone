#!/usr/bin/env python3
"""Normalize every slime SlimeFace mesh to a 1:1 quad.

The face textures are square (64x64). Existing GLBs authored the SlimeFace
primitive as a wide rectangle, which geometrically squashes the texture. This
script patches POSITION accessors only, preserving the full existing UV range.
"""
from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CREATURES = ROOT / "assets" / "models" / "creatures"
JSON_CHUNK = b"JSON"
BIN_CHUNK = b"BIN\0"
EPS = 1e-6


def is_slime_glb(path: Path) -> bool:
    try:
        rel = path.relative_to(CREATURES)
    except ValueError:
        return False
    return any(part.startswith("slime") for part in rel.parts[:-1])


def read_glb(path: Path) -> tuple[dict, bytearray, list[tuple[bytes, bytes]]]:
    raw = path.read_bytes()
    if len(raw) < 12:
        raise RuntimeError(f"{path}: truncated GLB")
    magic, version, declared = struct.unpack_from("<4sII", raw, 0)
    if magic != b"glTF" or version != 2 or declared != len(raw):
        raise RuntimeError(f"{path}: invalid GLB header")

    chunks: list[tuple[bytes, bytes]] = []
    offset = 12
    gltf = None
    binary = None
    while offset < len(raw):
        if offset + 8 > len(raw):
            raise RuntimeError(f"{path}: truncated GLB chunk header")
        length, kind = struct.unpack_from("<I4s", raw, offset)
        offset += 8
        data = raw[offset:offset + length]
        if len(data) != length:
            raise RuntimeError(f"{path}: truncated GLB chunk")
        offset += length
        chunks.append((kind, data))
        if kind == JSON_CHUNK:
            gltf = json.loads(data.decode("utf-8").rstrip(" \t\r\n\0"))
        elif kind == BIN_CHUNK:
            binary = bytearray(data)

    if gltf is None or binary is None:
        raise RuntimeError(f"{path}: GLB must contain JSON and BIN chunks")
    return gltf, binary, chunks


def write_glb(path: Path, gltf: dict, binary: bytearray, chunks: list[tuple[bytes, bytes]]) -> None:
    json_bytes = json.dumps(gltf, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    json_bytes += b" " * (-len(json_bytes) % 4)
    bin_bytes = bytes(binary)
    bin_bytes += b"\0" * (-len(bin_bytes) % 4)

    rebuilt: list[tuple[bytes, bytes]] = []
    seen_json = seen_bin = False
    for kind, data in chunks:
        if kind == JSON_CHUNK:
            if not seen_json:
                rebuilt.append((kind, json_bytes))
                seen_json = True
        elif kind == BIN_CHUNK:
            if not seen_bin:
                rebuilt.append((kind, bin_bytes))
                seen_bin = True
        else:
            rebuilt.append((kind, data))

    if not seen_json or not seen_bin:
        raise RuntimeError(f"{path}: missing required chunks during rebuild")

    total = 12 + sum(8 + len(data) for _, data in rebuilt)
    out = bytearray(struct.pack("<4sII", b"glTF", 2, total))
    for kind, data in rebuilt:
        out += struct.pack("<I4s", len(data), kind)
        out += data
    path.write_bytes(out)


def accessor_vec3(gltf: dict, binary: bytearray, accessor_index: int, path: Path):
    acc = gltf["accessors"][accessor_index]
    if acc.get("componentType") != 5126 or acc.get("type") != "VEC3":
        raise RuntimeError(f"{path}: SlimeFace POSITION accessor {accessor_index} must be float32 VEC3")
    if "sparse" in acc:
        raise RuntimeError(f"{path}: sparse SlimeFace POSITION accessor is unsupported")
    view = gltf["bufferViews"][acc["bufferView"]]
    if view.get("buffer", 0) != 0:
        raise RuntimeError(f"{path}: SlimeFace POSITION accessor must use buffer 0")
    base = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    stride = view.get("byteStride", 12)
    if stride < 12:
        raise RuntimeError(f"{path}: invalid POSITION byteStride {stride}")
    count = acc["count"]
    positions = [list(struct.unpack_from("<3f", binary, base + i * stride)) for i in range(count)]
    return acc, base, stride, positions


def primitive_face_extents(gltf: dict, binary: bytearray, primitive: dict, path: Path):
    attrs = primitive.get("attributes", {})
    if "POSITION" not in attrs:
        raise RuntimeError(f"{path}: SlimeFace primitive has no POSITION accessor")
    acc, base, stride, positions = accessor_vec3(gltf, binary, attrs["POSITION"], path)
    mins = [min(p[a] for p in positions) for a in range(3)]
    maxs = [max(p[a] for p in positions) for a in range(3)]
    extents = [maxs[a] - mins[a] for a in range(3)]
    ordered = sorted(range(3), key=lambda a: extents[a])
    plane = ordered[0]
    vary = ordered[1:]
    if extents[vary[0]] <= EPS or extents[vary[1]] <= EPS:
        raise RuntimeError(f"{path}: degenerate SlimeFace primitive extents={extents}")
    return acc, base, stride, positions, mins, maxs, extents, plane, vary


def iter_face_primitives(gltf: dict):
    materials = gltf.get("materials", [])
    face_materials = {i for i, mat in enumerate(materials) if mat.get("name") == "SlimeFace"}
    for mesh_i, mesh in enumerate(gltf.get("meshes", [])):
        for prim_i, primitive in enumerate(mesh.get("primitives", [])):
            if primitive.get("material") in face_materials:
                yield mesh_i, prim_i, primitive


def patch_file(path: Path, check_only: bool) -> tuple[int, int]:
    gltf, binary, chunks = read_glb(path)
    face_primitives = list(iter_face_primitives(gltf))
    patched = 0

    for mesh_i, prim_i, primitive in face_primitives:
        acc, base, stride, positions, mins, maxs, extents, _plane, vary = primitive_face_extents(
            gltf, binary, primitive, path
        )
        a, b = vary
        target = max(extents[a], extents[b])
        tolerance = max(1e-5, target * 1e-4)
        square = abs(extents[a] - extents[b]) <= tolerance
        if check_only:
            if not square:
                raise RuntimeError(
                    f"{path}: SlimeFace mesh {mesh_i} primitive {prim_i} is not square: "
                    f"{extents[a]:.6f} x {extents[b]:.6f}"
                )
            continue
        if square:
            continue

        centers = [(mins[axis] + maxs[axis]) * 0.5 for axis in range(3)]
        scale_a = target / extents[a]
        scale_b = target / extents[b]
        for i, pos in enumerate(positions):
            pos[a] = centers[a] + (pos[a] - centers[a]) * scale_a
            pos[b] = centers[b] + (pos[b] - centers[b]) * scale_b
            struct.pack_into("<3f", binary, base + i * stride, *pos)

        new_positions = [list(struct.unpack_from("<3f", binary, base + i * stride)) for i in range(len(positions))]
        if "min" in acc:
            acc["min"] = [float(min(p[axis] for p in new_positions)) for axis in range(3)]
        if "max" in acc:
            acc["max"] = [float(max(p[axis] for p in new_positions)) for axis in range(3)]
        patched += 1

    if patched and not check_only:
        write_glb(path, gltf, binary, chunks)
    return len(face_primitives), patched


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="verify only; do not modify assets")
    args = parser.parse_args()

    glbs = sorted(p for p in CREATURES.rglob("*.glb") if is_slime_glb(p))
    if not glbs:
        raise RuntimeError(f"No slime GLBs found under {CREATURES}")

    assets_with_face = 0
    total_faces = 0
    total_patched = 0
    for path in glbs:
        faces, patched = patch_file(path, args.check)
        if faces:
            assets_with_face += 1
            total_faces += faces
            total_patched += patched
            verb = "checked" if args.check else ("patched" if patched else "already-square")
            print(f"{verb}: {path.relative_to(ROOT)} ({faces} SlimeFace primitive(s))")

    if not assets_with_face:
        raise RuntimeError("No SlimeFace primitives found in slime GLBs")
    if args.check:
        print(f"Slime face check passed: {assets_with_face} asset(s), {total_faces} face primitive(s), all 1:1")
    else:
        print(f"Slime face normalization complete: {assets_with_face} asset(s), {total_faces} face primitive(s), {total_patched} patched")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
