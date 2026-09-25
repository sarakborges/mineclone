#!/usr/bin/env python3
"""Generate Asteria's rigid pixel-art grass world-object model."""
from __future__ import annotations

import json
import math
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent

# x, z, height, width, yaw_degrees, brightness.
# Geometry stays rigid: silhouette detail comes from the nearest-filtered pixel texture.
BLADES = (
    (-.18, -.11, .40, .080, -18, .94),
    (-.08, -.13, .54, .085, 6, 1.00),
    (.03, -.13, .47, .075, 22, .98),
    (.15, -.09, .34, .070, -27, .93),
    (-.23, -.01, .30, .066, 14, .92),
    (-.12, .01, .46, .080, 31, .97),
    (0.00, .00, .58, .090, -5, 1.00),
    (.11, .02, .43, .078, -34, .96),
    (.22, .07, .28, .064, 18, .91),
    (-.19, .11, .35, .070, -35, .94),
    (-.05, .13, .48, .080, 39, .98),
    (.08, .14, .37, .070, 12, .94),
    (.18, .16, .27, .060, -12, .90),
)

TEXTURE_WIDTH = 8
TEXTURE_HEIGHT = 16


def png_chunk(kind: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    )


def grass_texture_png() -> bytes:
    # Neutral grayscale is intentional: biome tint multiplies this texture while
    # dark/light pixels survive, avoiding the previous flat-color appearance.
    rows = []
    widths = (2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 6)
    for y, blade_width in enumerate(widths):
        start = (TEXTURE_WIDTH - blade_width) // 2
        row = bytearray([0])
        for x in range(TEXTURE_WIDTH):
            if start <= x < start + blade_width:
                local_x = x - start
                edge = local_x == 0 or local_x == blade_width - 1
                if edge:
                    value = 150 + ((x + y) & 1) * 10
                elif (x + y * 2) % 5 == 0:
                    value = 232
                elif (x * 3 + y) % 4 == 0:
                    value = 194
                else:
                    value = 214
                row.extend((value, value, value, 255))
            else:
                row.extend((0, 0, 0, 0))
        rows.append(bytes(row))

    raw = b"".join(rows)
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(
            b"IHDR",
            struct.pack(
                ">IIBBBBB",
                TEXTURE_WIDTH,
                TEXTURE_HEIGHT,
                8,
                6,
                0,
                0,
                0,
            ),
        )
        + png_chunk(b"IDAT", zlib.compress(raw, 9))
        + png_chunk(b"IEND", b"")
    )


positions: list[float] = []
normals: list[float] = []
colors: list[float] = []
uvs: list[float] = []
indices: list[int] = []


def add_blade(
    x: float,
    z: float,
    height: float,
    width: float,
    yaw_degrees: float,
    brightness: float,
) -> None:
    yaw = math.radians(yaw_degrees)
    right_x = math.cos(yaw)
    right_z = -math.sin(yaw)
    normal = (math.sin(yaw), 0.0, math.cos(yaw))
    half_width = width * 0.5
    left_x = x - right_x * half_width
    left_z = z - right_z * half_width
    right_px = x + right_x * half_width
    right_pz = z + right_z * half_width
    start = len(positions) // 3
    points = (
        (left_x, 0.0, left_z),
        (right_px, 0.0, right_pz),
        (right_px, height, right_pz),
        (left_x, height, left_z),
    )

    for px, py, pz in points:
        positions.extend((px, py, pz))
        normals.extend(normal)
        colors.extend((brightness, brightness, brightness, 1.0))

    uvs.extend((0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0))
    indices.extend((start, start + 1, start + 2, start, start + 2, start + 3))


for blade in BLADES:
    add_blade(*blade)

blob = bytearray()
views = []
accessors = []


def store(data: bytes, target: int | None = None) -> int:
    blob.extend(b"\0" * (-len(blob) % 4))
    offset = len(blob)
    blob.extend(data)
    view = {"buffer": 0, "byteOffset": offset, "byteLength": len(data)}
    if target is not None:
        view["target"] = target
    views.append(view)
    return len(views) - 1


def accessor(
    values,
    kind: str,
    component_type: int,
    target: int,
    bounds: bool = False,
) -> int:
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[kind]
    fmt = {5126: "f", 5123: "H"}[component_type]
    entry = {
        "bufferView": store(struct.pack("<" + fmt * len(values), *values), target),
        "componentType": component_type,
        "count": len(values) // width,
        "type": kind,
    }
    if bounds:
        rows = [values[i : i + width] for i in range(0, len(values), width)]
        entry["min"] = [
            float(min(row[j] for row in rows))
            for j in range(width)
        ]
        entry["max"] = [
            float(max(row[j] for row in rows))
            for j in range(width)
        ]
    accessors.append(entry)
    return len(accessors) - 1


position_accessor = accessor(positions, "VEC3", 5126, 34962, True)
normal_accessor = accessor(normals, "VEC3", 5126, 34962)
color_accessor = accessor(colors, "VEC4", 5126, 34962)
uv_accessor = accessor(uvs, "VEC2", 5126, 34962)
index_accessor = accessor(indices, "SCALAR", 5123, 34963)
image_view = store(grass_texture_png())

scene = {
    "asset": {
        "version": "2.0",
        "generator": "Asteria grass object generator v3",
    },
    "scene": 0,
    "scenes": [{"name": "GrassObject", "nodes": [0]}],
    "nodes": [
        {
            "name": "GrassRoot",
            "children": [1],
            "extras": {
                "asteria_asset": "object/grass",
                "unit": "meters",
                "origin": "ground_center",
                "collision": "none",
                "tint_material": "GrassTint",
                "style": "pixel_art_cards",
            },
        },
        {"name": "Visual", "mesh": 0},
    ],
    "meshes": [
        {
            "name": "grass_pixel_blades",
            "primitives": [
                {
                    "attributes": {
                        "POSITION": position_accessor,
                        "NORMAL": normal_accessor,
                        "COLOR_0": color_accessor,
                        "TEXCOORD_0": uv_accessor,
                    },
                    "indices": index_accessor,
                    "material": 0,
                    "mode": 4,
                }
            ],
        }
    ],
    "materials": [
        {
            "name": "GrassTint",
            "pbrMetallicRoughness": {
                "baseColorFactor": [1, 1, 1, 1],
                "baseColorTexture": {"index": 0},
                "metallicFactor": 0,
                "roughnessFactor": 1,
            },
            "alphaMode": "MASK",
            "alphaCutoff": 0.5,
            "doubleSided": True,
        }
    ],
    "samplers": [
        {
            "magFilter": 9728,
            "minFilter": 9728,
            "wrapS": 33071,
            "wrapT": 33071,
        }
    ],
    "images": [
        {
            "bufferView": image_view,
            "mimeType": "image/png",
            "name": "GrassPixels",
        }
    ],
    "textures": [{"sampler": 0, "source": 0}],
    "bufferViews": views,
    "accessors": accessors,
    "buffers": [{"byteLength": len(blob)}],
    "extras": {
        "asset_id": "asteria:grass_object",
        "kind": "world_object",
        "blade_count": len(BLADES),
        "tintable": True,
        "texture_size": [TEXTURE_WIDTH, TEXTURE_HEIGHT],
        "bounds_meters": {
            "width": .54,
            "height": .58,
            "depth": .34,
        },
    },
}

json_bytes = json.dumps(scene, separators=(",", ":")).encode()
json_bytes += b" " * (-len(json_bytes) % 4)
binary = bytes(blob) + b"\0" * (-len(blob) % 4)
glb = (
    struct.pack(
        "<4sII",
        b"glTF",
        2,
        12 + 8 + len(json_bytes) + 8 + len(binary),
    )
    + struct.pack("<I4s", len(json_bytes), b"JSON")
    + json_bytes
    + struct.pack("<I4s", len(binary), b"BIN\0")
    + binary
)
(OUT / "grass.glb").write_bytes(glb)
