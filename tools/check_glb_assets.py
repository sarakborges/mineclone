#!/usr/bin/env python3
"""Validate committed GLB assets without third-party dependencies.

This catches malformed/truncated containers before Bevy's GltfLoader reaches them at
runtime. It intentionally validates container structure and buffer ranges rather
than rendering semantics.
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path


GLB_MAGIC = b"glTF"
GLB_VERSION = 2
JSON_CHUNK = b"JSON"
BIN_CHUNK = b"BIN\x00"

COMPONENT_SIZE = {
    5120: 1,  # BYTE
    5121: 1,  # UNSIGNED_BYTE
    5122: 2,  # SHORT
    5123: 2,  # UNSIGNED_SHORT
    5125: 4,  # UNSIGNED_INT
    5126: 4,  # FLOAT
}

TYPE_COMPONENTS = {
    "SCALAR": 1,
    "VEC2": 2,
    "VEC3": 3,
    "VEC4": 4,
    "MAT2": 4,
    "MAT3": 9,
    "MAT4": 16,
}


def fail(path: Path, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def read_chunks(path: Path, data: bytes) -> list[tuple[bytes, bytes]]:
    if len(data) < 12:
        fail(path, f"file too short for GLB header ({len(data)} bytes)")

    magic, version, declared_length = struct.unpack_from("<4sII", data, 0)
    if magic != GLB_MAGIC:
        fail(path, f"invalid magic {magic!r}")
    if version != GLB_VERSION:
        fail(path, f"unsupported GLB version {version}")
    if declared_length != len(data):
        fail(
            path,
            f"header length {declared_length} does not match file length {len(data)}",
        )

    chunks: list[tuple[bytes, bytes]] = []
    offset = 12
    while offset < len(data):
        if offset + 8 > len(data):
            fail(path, f"truncated chunk header at byte {offset}")

        chunk_length, chunk_type = struct.unpack_from("<I4s", data, offset)
        offset += 8
        end = offset + chunk_length
        if end > len(data):
            fail(
                path,
                f"chunk {chunk_type!r} declares {chunk_length} bytes but only "
                f"{len(data) - offset} remain",
            )
        if chunk_length % 4 != 0:
            fail(path, f"chunk {chunk_type!r} length {chunk_length} is not 4-byte aligned")

        chunks.append((chunk_type, data[offset:end]))
        offset = end

    if offset != len(data):
        fail(path, f"chunk walk ended at {offset}, file length is {len(data)}")
    if not chunks or chunks[0][0] != JSON_CHUNK:
        fail(path, "first chunk is not JSON")
    if len(chunks) > 2:
        fail(path, f"unexpected GLB chunk count {len(chunks)}")
    if len(chunks) == 2 and chunks[1][0] != BIN_CHUNK:
        fail(path, f"second chunk is {chunks[1][0]!r}, expected BIN")

    return chunks


def validate_document(path: Path, document: dict, bin_length: int) -> None:
    buffers = document.get("buffers", [])
    if len(buffers) > 1:
        fail(path, f"GLB contains {len(buffers)} buffers; expected at most one embedded buffer")

    if buffers:
        buffer = buffers[0]
        if "uri" in buffer:
            fail(path, "GLB buffer unexpectedly contains a URI")
        declared = buffer.get("byteLength")
        if not isinstance(declared, int) or declared < 0:
            fail(path, f"invalid buffer byteLength {declared!r}")
        if declared > bin_length:
            fail(
                path,
                f"buffer declares {declared} bytes but BIN chunk contains {bin_length}",
            )

    views = document.get("bufferViews", [])
    for index, view in enumerate(views):
        if view.get("buffer", 0) != 0:
            fail(path, f"bufferView {index} references unsupported buffer {view.get('buffer')}")
        offset = view.get("byteOffset", 0)
        length = view.get("byteLength")
        if not isinstance(offset, int) or offset < 0:
            fail(path, f"bufferView {index} has invalid byteOffset {offset!r}")
        if not isinstance(length, int) or length < 0:
            fail(path, f"bufferView {index} has invalid byteLength {length!r}")
        if offset + length > bin_length:
            fail(
                path,
                f"bufferView {index} exceeds BIN chunk: "
                f"offset={offset} length={length} bin={bin_length}",
            )

    for index, accessor in enumerate(document.get("accessors", [])):
        view_index = accessor.get("bufferView")
        if view_index is None:
            continue
        if not isinstance(view_index, int) or not 0 <= view_index < len(views):
            fail(path, f"accessor {index} references invalid bufferView {view_index!r}")

        component_type = accessor.get("componentType")
        accessor_type = accessor.get("type")
        count = accessor.get("count")
        if component_type not in COMPONENT_SIZE:
            fail(path, f"accessor {index} has unsupported componentType {component_type!r}")
        if accessor_type not in TYPE_COMPONENTS:
            fail(path, f"accessor {index} has unsupported type {accessor_type!r}")
        if not isinstance(count, int) or count < 0:
            fail(path, f"accessor {index} has invalid count {count!r}")

        view = views[view_index]
        element_size = COMPONENT_SIZE[component_type] * TYPE_COMPONENTS[accessor_type]
        stride = view.get("byteStride", element_size)
        if not isinstance(stride, int) or stride < element_size:
            fail(path, f"accessor {index} has invalid stride {stride!r}")

        accessor_offset = accessor.get("byteOffset", 0)
        if not isinstance(accessor_offset, int) or accessor_offset < 0:
            fail(path, f"accessor {index} has invalid byteOffset {accessor_offset!r}")

        required = accessor_offset
        if count:
            required += (count - 1) * stride + element_size
        if required > view["byteLength"]:
            fail(
                path,
                f"accessor {index} exceeds bufferView {view_index}: "
                f"required={required} view={view['byteLength']}",
            )


def validate_glb(path: Path) -> None:
    data = path.read_bytes()
    chunks = read_chunks(path, data)

    json_bytes = chunks[0][1].rstrip(b" \t\r\n\x00")
    try:
        document = json.loads(json_bytes.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(path, f"invalid JSON chunk: {error}")

    bin_length = len(chunks[1][1]) if len(chunks) == 2 else 0
    validate_document(path, document, bin_length)


def main() -> int:
    paths = sorted(Path("assets").rglob("*.glb"))
    if not paths:
        print("GLB audit: no .glb assets found")
        return 0

    errors: list[str] = []
    for path in paths:
        try:
            validate_glb(path)
        except (OSError, ValueError) as error:
            errors.append(str(error))

    if errors:
        print("GLB audit failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"GLB audit passed: {len(paths)} asset(s) validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
