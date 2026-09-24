#!/usr/bin/env python3
"""Generate Asteria's compact organic grass world-object model."""
from __future__ import annotations

import json
import math
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parent

# x, z, height, width, depth, lean_x, lean_z, yaw_degrees, brightness
BLADES = (
    (-.17, -.11, .42, .060, .011, -.030, .018, -18, .96),
    (-.06, -.13, .55, .068, .012, .018, -.032, 6, 1.00),
    (.07, -.12, .48, .058, .010, .042, .010, 22, .98),
    (.18, -.07, .34, .052, .010, .038, .025, -27, .94),
    (-.22, 0, .31, .050, .009, -.045, .015, 14, .93),
    (-.10, .02, .47, .064, .011, -.020, .042, 31, .98),
    (.01, 0, .58, .072, .012, .010, .050, -5, 1.00),
    (.12, .02, .44, .060, .010, .035, .038, -34, .97),
    (.23, .08, .29, .046, .009, .048, .012, 18, .92),
    (-.18, .12, .36, .054, .010, -.040, .030, -35, .95),
    (-.04, .13, .49, .062, .010, -.012, .052, 39, .99),
    (.09, .14, .38, .052, .009, .028, .040, 12, .95),
    (.18, .16, .27, .044, .008, .035, .025, -12, .92),
)

positions: list[float] = []
normals: list[float] = []
colors: list[float] = []
indices: list[int] = []


def sub(a, b):
    return tuple(a[i] - b[i] for i in range(3))


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


def normalized(vector):
    length = math.sqrt(dot(vector, vector))
    return tuple(value / length for value in vector)


def add_quad(points, center, shade):
    points = list(points)
    normal = cross(sub(points[1], points[0]), sub(points[2], points[0]))
    face_center = tuple(sum(point[i] for point in points) / 4 for i in range(3))
    if dot(normal, sub(face_center, center)) < 0:
        points.reverse()
        normal = cross(sub(points[1], points[0]), sub(points[2], points[0]))
    normal = normalized(normal)
    start = len(positions) // 3
    shade = max(0.0, min(1.0, shade))
    for point in points:
        positions.extend(point)
        normals.extend(normal)
        colors.extend((shade, shade, shade, 1.0))
    indices.extend((start, start + 1, start + 2, start, start + 2, start + 3))


for x, z, height, width, depth, lean_x, lean_z, yaw_degrees, brightness in BLADES:
    yaw = math.radians(yaw_degrees)
    right = (math.cos(yaw), 0.0, -math.sin(yaw))
    forward = (math.sin(yaw), 0.0, math.cos(yaw))
    rings = []
    for t, width_scale, depth_scale in (
        (0.00, 1.00, 1.00),
        (0.38, 0.82, 0.88),
        (0.72, 0.54, 0.72),
        (1.00, 0.16, 0.44),
    ):
        curve = t * t
        cx = x + lean_x * (0.28 * t + 0.72 * curve)
        cz = z + lean_z * (0.28 * t + 0.72 * curve)
        cy = height * t
        half_width = width * width_scale * 0.5
        half_depth = depth * depth_scale * 0.5
        rings.append(
            [
                (
                    cx + right[0] * half_width * sw + forward[0] * half_depth * sd,
                    cy,
                    cz + right[2] * half_width * sw + forward[2] * half_depth * sd,
                )
                for sw, sd in ((-1, -1), (1, -1), (1, 1), (-1, 1))
            ]
        )

    center = (x + lean_x * .45, height * .5, z + lean_z * .45)
    add_quad(rings[0], center, brightness * .94)
    for lower, upper in zip(rings, rings[1:]):
        for side, shade in enumerate((.965, .985, 1.0, .975)):
            next_side = (side + 1) % 4
            add_quad(
                (lower[side], lower[next_side], upper[next_side], upper[side]),
                center,
                brightness * shade,
            )
    add_quad(rings[-1], center, min(1.0, brightness * 1.01))


blob = bytearray()
views = []
accessors = []


def store(data, target):
    blob.extend(b"\0" * (-len(blob) % 4))
    offset = len(blob)
    blob.extend(data)
    views.append(
        {"buffer": 0, "byteOffset": offset, "byteLength": len(data), "target": target}
    )
    return len(views) - 1


def accessor(values, kind, component_type, target, bounds=False):
    width = {"SCALAR": 1, "VEC3": 3, "VEC4": 4}[kind]
    fmt = {5126: "f", 5123: "H"}[component_type]
    entry = {
        "bufferView": store(struct.pack("<" + fmt * len(values), *values), target),
        "componentType": component_type,
        "count": len(values) // width,
        "type": kind,
    }
    if bounds:
        rows = [values[i : i + width] for i in range(0, len(values), width)]
        entry["min"] = [float(min(row[j] for row in rows)) for j in range(width)]
        entry["max"] = [float(max(row[j] for row in rows)) for j in range(width)]
    accessors.append(entry)
    return len(accessors) - 1


position_accessor = accessor(positions, "VEC3", 5126, 34962, True)
normal_accessor = accessor(normals, "VEC3", 5126, 34962)
color_accessor = accessor(colors, "VEC4", 5126, 34962)
index_accessor = accessor(indices, "SCALAR", 5123, 34963)

scene = {
    "asset": {"version": "2.0", "generator": "Asteria grass object generator v2"},
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
            },
        },
        {"name": "Visual", "mesh": 0},
    ],
    "meshes": [
        {
            "name": "grass_tuft",
            "primitives": [
                {
                    "attributes": {
                        "POSITION": position_accessor,
                        "NORMAL": normal_accessor,
                        "COLOR_0": color_accessor,
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
                "metallicFactor": 0,
                "roughnessFactor": 1,
            },
            "doubleSided": False,
        }
    ],
    "bufferViews": views,
    "accessors": accessors,
    "buffers": [{"byteLength": len(blob)}],
    "extras": {
        "asset_id": "asteria:grass_object",
        "kind": "world_object",
        "blade_count": len(BLADES),
        "tintable": True,
        "bounds_meters": {"width": .58, "height": .58, "depth": .48},
    },
}

json_bytes = json.dumps(scene, separators=(",", ":")).encode()
json_bytes += b" " * (-len(json_bytes) % 4)
binary = bytes(blob) + b"\0" * (-len(blob) % 4)
glb = (
    struct.pack("<4sII", b"glTF", 2, 12 + 8 + len(json_bytes) + 8 + len(binary))
    + struct.pack("<I4s", len(json_bytes), b"JSON")
    + json_bytes
    + struct.pack("<I4s", len(binary), b"BIN\0")
    + binary
)
(OUT / "grass.glb").write_bytes(glb)
