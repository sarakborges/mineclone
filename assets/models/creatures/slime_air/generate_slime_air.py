#!/usr/bin/env python3
"""Generate Asteria's elemental Air Slime from the rounded slime blob base.

The body preserves the standard slime_blob shape/animations. All shell, wing and
air-detail colors are fixed in the GLB. The only runtime texture override is
SlimeFace.
"""
from __future__ import annotations

import json
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parent

BODY_WIDTH = 1.20
BODY_HEIGHT = 1.00
BODY_DEPTH = 1.14
NX, NY, NZ = 24, 20, 22
DX, DY, DZ = BODY_WIDTH / NX, BODY_HEIGHT / NY, BODY_DEPTH / NZ
BODY_HALF_HEIGHT = BODY_HEIGHT * 0.5
CX = (NX - 1) / 2
CZ = (NZ - 1) / 2

SRGB = (
    ("AirShell", (.30, .78, .67)),
    ("AirShellCenter", (.48, .88, .78)),
    ("AirShellOuter", (.20, .62, .53)),
    ("AirShellBottom", (.24, .72, .60)),
    ("AirShellTop", (.84, .97, .93)),
    ("AirShellLight", (.93, .995, .98)),
    ("AirShellBright", (1.0, 1.0, 1.0)),
    ("SlimeFace", (1.0, 1.0, 1.0)),
    ("AirWingWhite", (.96, 1.0, .995)),
    ("AirWingMint", (.72, .94, .88)),
    ("AirDetail", (.56, .90, .82)),
)

binary = bytearray()
views: list[dict] = []
accessors: list[dict] = []
meshes: list[dict] = []
nodes: list[dict] = []
animations: list[dict] = []


def linear(channel: float) -> float:
    return channel / 12.92 if channel <= .04045 else ((channel + .055) / 1.055) ** 2.4


def store(data: bytes, target: int | None = None) -> int:
    binary.extend(b"\0" * (-len(binary) % 4))
    offset = len(binary)
    binary.extend(data)
    entry = {"buffer": 0, "byteOffset": offset, "byteLength": len(data)}
    if target is not None:
        entry["target"] = target
    views.append(entry)
    return len(views) - 1


def accessor(values, kind="VEC3", component=5126, target=None, bounds=False):
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[kind]
    fmt = {5126: "f", 5123: "H"}[component]
    entry = {
        "bufferView": store(struct.pack("<" + fmt * len(values), *values), target),
        "componentType": component,
        "count": len(values) // width,
        "type": kind,
    }
    if bounds:
        rows = [values[i : i + width] for i in range(0, len(values), width)]
        entry["min"] = [float(min(row[j] for row in rows)) for j in range(width)]
        entry["max"] = [float(max(row[j] for row in rows)) for j in range(width)]
    accessors.append(entry)
    return len(accessors) - 1


def profile(t: float) -> float:
    keys = (
        (0, .66), (.06, .79), (.15, .91), (.28, .995), (.44, 1),
        (.58, .965), (.70, .89), (.80, .76), (.88, .60), (.94, .36),
        (.975, .14), (1, .02),
    )
    for (at, ar), (bt, br) in zip(keys, keys[1:]):
        if t <= bt:
            u = (t - at) / (bt - at)
            u = u * u * (3 - 2 * u)
            return ar + (br - ar) * u
    return .02


def occupied(ix: int, iy: int, iz: int) -> bool:
    x = (ix + .5) * DX - BODY_WIDTH * .5
    y = (iy + .5) * DY - BODY_HEIGHT * .5
    z = (iz + .5) * DZ - BODY_DEPTH * .5
    t = (y + BODY_HALF_HEIGHT) / BODY_HEIGHT
    radius = profile(t)
    x -= .032 * max(0, (t - .62) / .38) ** 1.7
    rx = BODY_WIDTH * .5 * radius
    rz = BODY_DEPTH * .5 * radius
    return rx > 0 and rz > 0 and abs(x / rx) ** 2.35 + abs(z / rz) ** 2.35 <= 1


voxels = {
    (x, y, z)
    for y in range(NY)
    for z in range(NZ)
    for x in range(NX)
    if occupied(x, y, z)
}

FACES = (
    ((1, 0, 0), ((1, 0, 0), (1, 0, 1), (1, 1, 1), (1, 1, 0))),
    ((-1, 0, 0), ((0, 0, 1), (0, 0, 0), (0, 1, 0), (0, 1, 1))),
    ((0, 1, 0), ((0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1))),
    ((0, -1, 0), ((0, 0, 1), (1, 0, 1), (1, 0, 0), (0, 0, 0))),
    ((0, 0, 1), ((1, 0, 1), (0, 0, 1), (0, 1, 1), (1, 1, 1))),
    ((0, 0, -1), ((0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0))),
)


def shell_material(ix: int, iy: int, iz: int, normal) -> int:
    gx = (ix - CX) / (NX * .5)
    gy = iy / (NY - 1)
    gz = (iz - CZ) / (NZ * .5)

    if normal == (0, 0, -1):
        if -.58 < gx < -.25 and .64 < gy < .86:
            return 6
        if -.74 < gx < -.10 and .50 < gy < .90:
            return 5
        if gy < .18:
            return 3
        if abs(gx) > .73:
            return 2
        if gy > .70:
            return 4
        if abs(gx) < .60 and .20 < gy < .72:
            return 1
        return 0
    if normal == (0, 1, 0):
        return 5 if gy > .56 else 4
    if normal == (0, -1, 0):
        return 3
    if normal == (1, 0, 0):
        return 4 if gy > .58 else 2
    if normal == (-1, 0, 0):
        return 4 if gy > .48 else 0
    if normal == (0, 0, 1):
        return 4 if gy > .70 else (2 if abs(gx) > .66 or abs(gz) > .66 else 0)
    return 0


def bucket():
    return [[], [], [], [], []]


UV = ((0, 1), (1, 1), (1, 0), (0, 0))


def quad(target, points, normal, *, front=False):
    positions, normals, uvs, colors, indices = target
    offset = len(positions) // 3
    for index, point in enumerate(points):
        positions.extend(point)
        normals.extend(normal)
        uvs.extend(UV[index])
        colors.extend((1, 1, 1, 1))
    if front:
        indices.extend((offset, offset + 1, offset + 2, offset, offset + 2, offset + 3))
    else:
        indices.extend((offset, offset + 2, offset + 1, offset, offset + 3, offset + 2))


def primitive(target, material_index):
    positions, normals, uvs, colors, indices = target
    if not indices:
        return None
    return {
        "attributes": {
            "POSITION": accessor(positions, bounds=True, target=34962),
            "NORMAL": accessor(normals, target=34962),
            "TEXCOORD_0": accessor(uvs, kind="VEC2", target=34962),
            "COLOR_0": accessor(colors, kind="VEC4", target=34962),
        },
        "indices": accessor(indices, kind="SCALAR", component=5123, target=34963),
        "material": material_index,
        "mode": 4,
    }


shell_buckets = [bucket() for _ in range(7)]
for ix, iy, iz in sorted(voxels, key=lambda value: (value[1], value[2], value[0])):
    for normal, corners in FACES:
        if (ix + normal[0], iy + normal[1], iz + normal[2]) in voxels:
            continue
        points = [
            (
                -BODY_WIDTH * .5 + (ix + cx) * DX,
                -BODY_HEIGHT * .5 + (iy + cy) * DY,
                -BODY_DEPTH * .5 + (iz + cz) * DZ,
            )
            for cx, cy, cz in corners
        ]
        quad(shell_buckets[shell_material(ix, iy, iz, normal)], points, normal)

meshes.append({
    "name": "air_slime_blob",
    "primitives": [
        item for item in
        (primitive(value, index) for index, value in enumerate(shell_buckets))
        if item
    ],
})
shell_mesh = len(meshes) - 1

face_bucket = bucket()
face_z = -BODY_DEPTH * .5 - .006
face_w = .86
face_h = .48
face_y = -.075
quad(
    face_bucket,
    (
        (face_w / 2, face_y - face_h / 2, face_z),
        (-face_w / 2, face_y - face_h / 2, face_z),
        (-face_w / 2, face_y + face_h / 2, face_z),
        (face_w / 2, face_y + face_h / 2, face_z),
    ),
    (0, 0, -1),
    front=True,
)
meshes.append({"name": "air_slime_face", "primitives": [primitive(face_bucket, 7)]})
face_mesh = len(meshes) - 1


def box_into(target, minimum, maximum):
    x0, y0, z0 = minimum
    x1, y1, z1 = maximum
    faces = (
        ((1, 0, 0), ((x1, y0, z0), (x1, y0, z1), (x1, y1, z1), (x1, y1, z0))),
        ((-1, 0, 0), ((x0, y0, z1), (x0, y0, z0), (x0, y1, z0), (x0, y1, z1))),
        ((0, 1, 0), ((x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1))),
        ((0, -1, 0), ((x0, y0, z1), (x1, y0, z1), (x1, y0, z0), (x0, y0, z0))),
        ((0, 0, 1), ((x1, y0, z1), (x0, y0, z1), (x0, y1, z1), (x1, y1, z1))),
        ((0, 0, -1), ((x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0))),
    )
    for normal, points in faces:
        quad(target, points, normal)


wing_white = bucket()
wing_mint = bucket()
for minimum, maximum, material_index in (
    ((.50, .05, -.065), (.68, .27, .065), 8),
    ((.62, -.01, -.060), (.79, .18, .060), 9),
    ((.75, -.07, -.055), (.90, .09, .055), 8),
):
    target = wing_white if material_index == 8 else wing_mint
    box_into(target, minimum, maximum)
    box_into(
        target,
        (-maximum[0], minimum[1], minimum[2]),
        (-minimum[0], maximum[1], maximum[2]),
    )

meshes.append({
    "name": "air_wings",
    "primitives": [
        item for item in (primitive(wing_white, 8), primitive(wing_mint, 9)) if item
    ],
})
wings_mesh = len(meshes) - 1

detail_mint = bucket()
detail_white = bucket()
box_into(detail_mint, (.33, .43, -.18), (.41, .51, -.10))
box_into(detail_white, (-.33, .36, -.20), (-.26, .43, -.13))
meshes.append({
    "name": "air_motes",
    "primitives": [primitive(detail_mint, 10), primitive(detail_white, 8)],
})
detail_mesh = len(meshes) - 1

materials = []
for index, (name, color) in enumerate(SRGB):
    material = {
        "name": name,
        "pbrMetallicRoughness": {
            "baseColorFactor": [linear(color[0]), linear(color[1]), linear(color[2]), 1],
            "metallicFactor": 0,
            "roughnessFactor": 1,
        },
        "doubleSided": False,
        "extensions": {"KHR_materials_unlit": {}},
    }
    if index == 7:
        material["alphaMode"] = "BLEND"
    materials.append(material)


def node(name, mesh=None, children=None, translation=None, extras=None):
    entry = {"name": name}
    if mesh is not None:
        entry["mesh"] = mesh
    if children is not None:
        entry["children"] = children
    if translation is not None:
        entry["translation"] = translation
    if extras is not None:
        entry["extras"] = extras
    nodes.append(entry)
    return len(nodes) - 1


root = node("SlimeRoot", children=[], extras={
    "asteria_asset": "creature/slime_air",
    "unit": "meters",
    "forward": "-Z",
    "collider": {"shape": "aabb", "size": [.78, .84, .78], "offset": [0, .42, 0]},
    "collider_is_animated": False,
})
visual = node("Visual", children=[])
body = node("BodyPivot", children=[], translation=[0, BODY_HALF_HEIGHT, 0])
shell = node("Shell", mesh=shell_mesh)
wings = node("Wings", mesh=wings_mesh)
details = node("AirDetails", mesh=detail_mesh)
face = node("Face", mesh=face_mesh)
nodes[body]["children"] = [shell, wings, details, face]
nodes[visual]["children"] = [body]
hitbox = node("Hitbox_AABB", translation=[0, .42, 0], extras={
    "asteria_collider": {
        "shape": "aabb",
        "size": [.78, .84, .78],
        "solid": True,
        "targetable": True,
    },
    "debug_display": False,
})
nodes[root]["children"] = [visual, hitbox]


def tracks(name, times, scales, center=None):
    center = center if center is not None else [BODY_HALF_HEIGHT * scale[1] for scale in scales]
    time_accessor = accessor(times, kind="SCALAR", bounds=True)
    channels = []
    samplers = []

    def add(target, path, values, kind):
        output = accessor([value for row in values for value in row], kind=kind)
        samplers.append({"input": time_accessor, "output": output, "interpolation": "LINEAR"})
        channels.append({"sampler": len(samplers) - 1, "target": {"node": target, "path": path}})

    add(body, "scale", scales, "VEC3")
    add(body, "translation", [[0, y, 0] for y in center], "VEC3")
    animations.append({
        "name": name,
        "channels": channels,
        "samplers": samplers,
        "extras": {"loop_recommended": name in ("Idle", "Airborne")},
    })


tracks("Idle", [0, .5, 1, 1.5, 2], [[1,1,1],[1.014,.982,1.014],[1,1,1],[.991,1.016,.991],[1,1,1]])
tracks("Anticipate", [0,.07,.17,.24], [[1,1,1],[1.075,.87,1.075],[1.10,.80,1.10],[1.075,.86,1.075]])
tracks("Airborne", [0,.12,.35,.55,.72], [[1.075,.86,1.075],[.94,1.12,.94],[.965,1.07,.965],[.98,1.04,.98],[1,1,1]])
tracks("Land", [0,.045,.12,.20,.34], [[.98,1.04,.98],[1.13,.79,1.13],[1.09,.85,1.09],[.985,1.03,.985],[1,1,1]])
tracks("Hurt", [0,.085,.15,.24,.38], [[1,1,1],[1.08,.90,1.08],[.95,1.06,.95],[1.02,.98,1.02],[1,1,1]])
tracks("Death", [0,.12,.31,.55,.75], [[1,1,1],[1.10,.84,1.10],[1.16,.65,1.16],[1.10,.20,1.10],[.001,.001,.001]], center=[BODY_HALF_HEIGHT,.42,.31,.10,.0005])

scene = {
    "asset": {"version": "2.0", "generator": "Asteria air slime v1"},
    "scene": 0,
    "scenes": [{"name": "AirSlime", "nodes": [root]}],
    "extensionsUsed": ["KHR_materials_unlit"],
    "nodes": nodes,
    "meshes": meshes,
    "materials": materials,
    "animations": animations,
    "bufferViews": views,
    "accessors": accessors,
    "buffers": [{"byteLength": len(binary)}],
    "extras": {
        "asset_id": "asteria:slime_air",
        "color_materials": [material["name"] for material in materials],
        "collision_source": "slime_air.collider.json",
        "voxel_resolution": [NX, NY, NZ],
        "occupied_voxels": len(voxels),
        "notes": "Air elemental derived from rounded blob: green-leaning turquoise base, near-white top, blocky feather wings and two air motes.",
    },
}

json_chunk = json.dumps(scene, separators=(",", ":")).encode()
json_chunk += b" " * (-len(json_chunk) % 4)
bin_chunk = bytes(binary) + b"\0" * (-len(binary) % 4)
glb = (
    struct.pack("<4sII", b"glTF", 2, 12 + 8 + len(json_chunk) + 8 + len(bin_chunk))
    + struct.pack("<I4s", len(json_chunk), b"JSON") + json_chunk
    + struct.pack("<I4s", len(bin_chunk), b"BIN\0") + bin_chunk
)
(OUT / "slime_air.glb").write_bytes(glb)
print(f"Generated {OUT / 'slime_air.glb'}: {len(voxels)} body voxels")
