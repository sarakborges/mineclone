#!/usr/bin/env python3
"""Validate committed GLB assets without third-party dependencies.

The audit intentionally covers both the binary container and the semantic accessor
relationships that Bevy's glTF loader relies on. It catches truncated containers,
out-of-range buffer views/accessors, stale mesh index metadata, mismatched vertex
attribute counts, invalid accessor bounds, and non-increasing animation key times.
"""

from __future__ import annotations

import json
import math
import struct
import sys
from pathlib import Path
from typing import Any


GLB_MAGIC = b"glTF"
GLB_VERSION = 2
JSON_CHUNK = b"JSON"
BIN_CHUNK = b"BIN\x00"

COMPONENT_FORMAT = {
    5120: "b",  # BYTE
    5121: "B",  # UNSIGNED_BYTE
    5122: "h",  # SHORT
    5123: "H",  # UNSIGNED_SHORT
    5125: "I",  # UNSIGNED_INT
    5126: "f",  # FLOAT
}

COMPONENT_SIZE = {
    5120: 1,
    5121: 1,
    5122: 2,
    5123: 2,
    5125: 4,
    5126: 4,
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

VECTOR_TYPES = {"SCALAR", "VEC2", "VEC3", "VEC4"}
FLOAT_COMPONENT = 5126
UNSIGNED_INDEX_COMPONENTS = {5121, 5123, 5125}


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


def accessor_layout(
    path: Path,
    accessor_index: int,
    accessor: dict[str, Any],
    views: list[dict[str, Any]],
) -> tuple[dict[str, Any], int, int, int]:
    view_index = accessor.get("bufferView")
    if not isinstance(view_index, int) or not 0 <= view_index < len(views):
        fail(path, f"accessor {accessor_index} references invalid bufferView {view_index!r}")

    component_type = accessor.get("componentType")
    accessor_type = accessor.get("type")
    count = accessor.get("count")
    if component_type not in COMPONENT_SIZE:
        fail(path, f"accessor {accessor_index} has unsupported componentType {component_type!r}")
    if accessor_type not in TYPE_COMPONENTS:
        fail(path, f"accessor {accessor_index} has unsupported type {accessor_type!r}")
    if not isinstance(count, int) or count < 0:
        fail(path, f"accessor {accessor_index} has invalid count {count!r}")

    view = views[view_index]
    element_size = COMPONENT_SIZE[component_type] * TYPE_COMPONENTS[accessor_type]
    stride = view.get("byteStride", element_size)
    if not isinstance(stride, int) or stride < element_size:
        fail(path, f"accessor {accessor_index} has invalid stride {stride!r}")

    accessor_offset = accessor.get("byteOffset", 0)
    if not isinstance(accessor_offset, int) or accessor_offset < 0:
        fail(path, f"accessor {accessor_index} has invalid byteOffset {accessor_offset!r}")

    required = accessor_offset
    if count:
        required += (count - 1) * stride + element_size
    if required > view["byteLength"]:
        fail(
            path,
            f"accessor {accessor_index} exceeds bufferView {view_index}: "
            f"required={required} view={view['byteLength']}",
        )

    return view, element_size, stride, accessor_offset


def read_accessor_values(
    path: Path,
    accessor_index: int,
    accessors: list[dict[str, Any]],
    views: list[dict[str, Any]],
    bin_data: bytes,
) -> list[tuple[int | float, ...]]:
    accessor = accessors[accessor_index]
    if accessor.get("sparse") is not None:
        fail(path, f"accessor {accessor_index} uses unsupported sparse storage")

    view, _element_size, stride, accessor_offset = accessor_layout(
        path, accessor_index, accessor, views
    )
    component_type = accessor["componentType"]
    component_count = TYPE_COMPONENTS[accessor["type"]]
    component_format = COMPONENT_FORMAT[component_type]
    component_size = COMPONENT_SIZE[component_type]
    value_format = "<" + component_format * component_count

    start = view.get("byteOffset", 0) + accessor_offset
    values: list[tuple[int | float, ...]] = []
    for element in range(accessor["count"]):
        offset = start + element * stride
        end = offset + component_size * component_count
        if end > len(bin_data):
            fail(path, f"accessor {accessor_index} reads past BIN chunk at byte {end}")
        values.append(struct.unpack_from(value_format, bin_data, offset))
    return values


def numbers_close(actual: int | float, declared: int | float) -> bool:
    if isinstance(actual, float) or isinstance(declared, float):
        return math.isclose(float(actual), float(declared), rel_tol=1e-5, abs_tol=1e-6)
    return actual == declared


def validate_accessor_bounds(
    path: Path,
    accessor_index: int,
    accessor: dict[str, Any],
    values: list[tuple[int | float, ...]],
) -> None:
    if not values or accessor.get("type") not in VECTOR_TYPES:
        return

    declared_min = accessor.get("min")
    declared_max = accessor.get("max")
    if declared_min is None and declared_max is None:
        return

    width = TYPE_COMPONENTS[accessor["type"]]
    if declared_min is not None:
        if not isinstance(declared_min, list) or len(declared_min) != width:
            fail(path, f"accessor {accessor_index} has invalid min {declared_min!r}")
        actual_min = [min(row[axis] for row in values) for axis in range(width)]
        for axis, (actual, declared) in enumerate(zip(actual_min, declared_min)):
            if not numbers_close(actual, declared):
                fail(
                    path,
                    f"accessor {accessor_index} min[{axis}]={declared!r} "
                    f"does not match decoded value {actual!r}",
                )

    if declared_max is not None:
        if not isinstance(declared_max, list) or len(declared_max) != width:
            fail(path, f"accessor {accessor_index} has invalid max {declared_max!r}")
        actual_max = [max(row[axis] for row in values) for axis in range(width)]
        for axis, (actual, declared) in enumerate(zip(actual_max, declared_max)):
            if not numbers_close(actual, declared):
                fail(
                    path,
                    f"accessor {accessor_index} max[{axis}]={declared!r} "
                    f"does not match decoded value {actual!r}",
                )


def validate_meshes(
    path: Path,
    document: dict[str, Any],
    accessor_values: dict[int, list[tuple[int | float, ...]]],
) -> None:
    accessors = document.get("accessors", [])
    for mesh_index, mesh in enumerate(document.get("meshes", [])):
        for primitive_index, primitive in enumerate(mesh.get("primitives", [])):
            attributes = primitive.get("attributes", {})
            position_index = attributes.get("POSITION")
            if position_index is None:
                continue
            if not isinstance(position_index, int) or not 0 <= position_index < len(accessors):
                fail(
                    path,
                    f"mesh {mesh_index} primitive {primitive_index} has invalid POSITION accessor "
                    f"{position_index!r}",
                )

            position_count = accessors[position_index].get("count")
            for semantic, attribute_index in attributes.items():
                if not isinstance(attribute_index, int) or not 0 <= attribute_index < len(accessors):
                    fail(
                        path,
                        f"mesh {mesh_index} primitive {primitive_index} {semantic} references "
                        f"invalid accessor {attribute_index!r}",
                    )
                attribute_count = accessors[attribute_index].get("count")
                if attribute_count != position_count:
                    fail(
                        path,
                        f"mesh {mesh_index} primitive {primitive_index} {semantic} count "
                        f"{attribute_count} does not match POSITION count {position_count}",
                    )

                if semantic == "COLOR_0" and accessors[attribute_index].get("componentType") == FLOAT_COMPONENT:
                    for vertex, color in enumerate(accessor_values.get(attribute_index, [])):
                        if any(not 0.0 <= float(component) <= 1.0 for component in color):
                            fail(
                                path,
                                f"mesh {mesh_index} primitive {primitive_index} COLOR_0 vertex "
                                f"{vertex} is outside [0, 1]: {color!r}",
                            )

            indices_index = primitive.get("indices")
            if indices_index is None:
                continue
            if not isinstance(indices_index, int) or not 0 <= indices_index < len(accessors):
                fail(
                    path,
                    f"mesh {mesh_index} primitive {primitive_index} references invalid indices "
                    f"accessor {indices_index!r}",
                )
            if accessors[indices_index].get("componentType") not in UNSIGNED_INDEX_COMPONENTS:
                fail(
                    path,
                    f"mesh {mesh_index} primitive {primitive_index} indices accessor "
                    f"{indices_index} is not unsigned integer",
                )

            for element, value in enumerate(accessor_values.get(indices_index, [])):
                index_value = int(value[0])
                if index_value < 0 or index_value >= position_count:
                    fail(
                        path,
                        f"mesh {mesh_index} primitive {primitive_index} index[{element}]="
                        f"{index_value} is outside POSITION count {position_count}",
                    )


def validate_animations(
    path: Path,
    document: dict[str, Any],
    accessor_values: dict[int, list[tuple[int | float, ...]]],
) -> None:
    accessors = document.get("accessors", [])
    for animation_index, animation in enumerate(document.get("animations", [])):
        for sampler_index, sampler in enumerate(animation.get("samplers", [])):
            input_index = sampler.get("input")
            output_index = sampler.get("output")
            for role, accessor_index in (("input", input_index), ("output", output_index)):
                if not isinstance(accessor_index, int) or not 0 <= accessor_index < len(accessors):
                    fail(
                        path,
                        f"animation {animation_index} sampler {sampler_index} {role} references "
                        f"invalid accessor {accessor_index!r}",
                    )

            input_accessor = accessors[input_index]
            if input_accessor.get("componentType") != FLOAT_COMPONENT or input_accessor.get("type") != "SCALAR":
                fail(
                    path,
                    f"animation {animation_index} sampler {sampler_index} input accessor "
                    f"{input_index} must be FLOAT SCALAR",
                )

            times = [float(value[0]) for value in accessor_values.get(input_index, [])]
            if any(not math.isfinite(value) for value in times):
                fail(
                    path,
                    f"animation {animation_index} sampler {sampler_index} contains non-finite key time",
                )
            for keyframe in range(1, len(times)):
                if times[keyframe] <= times[keyframe - 1]:
                    fail(
                        path,
                        f"animation {animation_index} sampler {sampler_index} key times are not "
                        f"strictly increasing at {keyframe - 1}->{keyframe}: "
                        f"{times[keyframe - 1]} >= {times[keyframe]}",
                    )


def validate_document(path: Path, document: dict[str, Any], bin_data: bytes) -> None:
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
        if declared > len(bin_data):
            fail(
                path,
                f"buffer declares {declared} bytes but BIN chunk contains {len(bin_data)}",
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
        if offset + length > len(bin_data):
            fail(
                path,
                f"bufferView {index} exceeds BIN chunk: "
                f"offset={offset} length={length} bin={len(bin_data)}",
            )

    accessors = document.get("accessors", [])
    accessor_values: dict[int, list[tuple[int | float, ...]]] = {}
    for index, accessor in enumerate(accessors):
        if accessor.get("bufferView") is None:
            if accessor.get("sparse") is None:
                continue
            fail(path, f"accessor {index} uses unsupported sparse-only storage")
        accessor_layout(path, index, accessor, views)
        values = read_accessor_values(path, index, accessors, views, bin_data)
        accessor_values[index] = values
        validate_accessor_bounds(path, index, accessor, values)

    validate_meshes(path, document, accessor_values)
    validate_animations(path, document, accessor_values)


def validate_glb(path: Path) -> None:
    data = path.read_bytes()
    chunks = read_chunks(path, data)

    json_bytes = chunks[0][1].rstrip(b" \t\r\n\x00")
    try:
        document = json.loads(json_bytes.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(path, f"invalid JSON chunk: {error}")

    bin_data = chunks[1][1] if len(chunks) == 2 else b""
    validate_document(path, document, bin_data)


def main() -> int:
    paths = sorted(Path("assets").rglob("*.glb"))
    if not paths:
        print("GLB audit: no .glb assets found")
        return 0

    errors: list[str] = []
    for path in paths:
        try:
            validate_glb(path)
        except (OSError, ValueError, struct.error) as error:
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
