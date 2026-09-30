#!/usr/bin/env python3
"""Generate pixel-art Pyro Slime models from the rounded slime_blob body language.

The Pyro body is a clean orange-gold blob with no horns or armor. Its modeled
identity comes from several flame tongues growing directly from the top surface.
Every flame uses axis-aligned voxel stair-steps: apparent curves and diagonals
are built from stepped boxes, never rotated or smooth geometry. The face remains
a square texture-driven SlimeFace decal.
"""
from __future__ import annotations

import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent
NX, NY, NZ = 24, 20, 22

PALETTE = (
    ("SlimeShell", (1.00, 0.40, 0.035)),
    ("SlimeShellCenter", (1.00, 0.53, 0.055)),
    ("SlimeShellOuter", (0.88, 0.22, 0.015)),
    ("SlimeShellBottom", (1.00, 0.67, 0.075)),
    ("SlimeShellTop", (0.94, 0.29, 0.010)),
    ("SlimeShellLight", (1.00, 0.77, 0.14)),
    ("SlimeShellBright", (1.00, 0.92, 0.36)),
    ("ElementAccent", (0.96, 0.19, 0.010)),
    ("ElementPale", (1.00, 0.57, 0.025)),
    ("ElementDark", (0.73, 0.085, 0.000)),
    ("ElementWhite", (1.00, 0.91, 0.27)),
)


def lin(c: float) -> float:
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def profile(t: float) -> float:
    keys = (
        (0.0, 0.66), (0.06, 0.79), (0.15, 0.91), (0.28, 0.995),
        (0.44, 1.0), (0.58, 0.965), (0.70, 0.89), (0.80, 0.76),
        (0.88, 0.60), (0.94, 0.36), (0.975, 0.14), (1.0, 0.02),
    )
    for (a, ar), (b, br) in zip(keys, keys[1:]):
        if t <= b:
            u = (t - a) / (b - a)
            u = u * u * (3.0 - 2.0 * u)
            return ar + (br - ar) * u
    return 0.02


def generate(asset_id: str, out_path: Path, body_width: float, body_height: float,
             body_depth: float, large: bool = False) -> dict:
    dx, dy, dz = body_width / NX, body_height / NY, body_depth / NZ
    half_h = body_height * 0.5
    binary = bytearray()
    views: list[dict] = []
    accessors: list[dict] = []
    meshes: list[dict] = []
    nodes: list[dict] = []
    animations: list[dict] = []

    def store(data: bytes, target: int | None = None) -> int:
        binary.extend(b"\0" * (-len(binary) % 4))
        offset = len(binary)
        binary.extend(data)
        entry = {"buffer": 0, "byteOffset": offset, "byteLength": len(data)}
        if target is not None:
            entry["target"] = target
        views.append(entry)
        return len(views) - 1

    def acc(values, kind="VEC3", component=5126, target=None, bounds=False) -> int:
        width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[kind]
        fmt = {5126: "f", 5123: "H"}[component]
        packed = struct.pack("<" + fmt * len(values), *values)
        entry = {
            "bufferView": store(packed, target),
            "componentType": component,
            "count": len(values) // width,
            "type": kind,
        }
        if bounds:
            rows = [values[i:i + width] for i in range(0, len(values), width)]
            entry["min"] = [float(min(r[j] for r in rows)) for j in range(width)]
            entry["max"] = [float(max(r[j] for r in rows)) for j in range(width)]
        accessors.append(entry)
        return len(accessors) - 1

    def occupied(ix: int, iy: int, iz: int) -> bool:
        x = (ix + 0.5) * dx - body_width * 0.5
        y = (iy + 0.5) * dy - body_height * 0.5
        z = (iz + 0.5) * dz - body_depth * 0.5
        t = (y + half_h) / body_height
        radius = profile(t)
        x -= 0.032 * (body_width / 1.20) * max(0.0, (t - 0.62) / 0.38) ** 1.7
        rx = body_width * 0.5 * radius
        rz = body_depth * 0.5 * radius
        return rx > 0 and rz > 0 and abs(x / rx) ** 2.35 + abs(z / rz) ** 2.35 <= 1

    vox = {(x, y, z) for y in range(NY) for z in range(NZ) for x in range(NX) if occupied(x, y, z)}
    faces = (
        ((1, 0, 0), ((1, 0, 0), (1, 0, 1), (1, 1, 1), (1, 1, 0))),
        ((-1, 0, 0), ((0, 0, 1), (0, 0, 0), (0, 1, 0), (0, 1, 1))),
        ((0, 1, 0), ((0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1))),
        ((0, -1, 0), ((0, 0, 1), (1, 0, 1), (1, 0, 0), (0, 0, 0))),
        ((0, 0, 1), ((1, 0, 1), (0, 0, 1), (0, 1, 1), (1, 1, 1))),
        ((0, 0, -1), ((0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0))),
    )
    cx_mid, cz_mid = (NX - 1) / 2, (NZ - 1) / 2

    def body_material(ix: int, iy: int, iz: int, normal) -> int:
        gx = (ix - cx_mid) / (NX * 0.5)
        gy = iy / (NY - 1)
        gz = (iz - cz_mid) / (NZ * 0.5)
        if normal == (0, 0, -1):
            if gy < 0.16:
                return 3
            if abs(gx) > 0.74:
                return 2
            if gy > 0.80:
                return 4
            if abs(gx) < 0.58 and 0.18 < gy < 0.72:
                return 1
            return 0
        if normal == (0, 1, 0):
            return 5 if gy > 0.70 else 4
        if normal == (0, -1, 0):
            return 3
        if normal == (1, 0, 0):
            return 2
        if normal == (-1, 0, 0):
            return 0
        if normal == (0, 0, 1):
            return 2 if abs(gx) > 0.68 or abs(gz) > 0.68 else 0
        return 0

    def bucket():
        return [[], [], [], [], []]

    uv = ((0, 1), (1, 1), (1, 0), (0, 0))

    def quad(b, pts, normal):
        positions, normals, uvs, colors, indices = b
        origin = len(positions) // 3
        for j, pt in enumerate(pts):
            positions.extend(pt)
            normals.extend(normal)
            uvs.extend(uv[j])
            colors.extend((1, 1, 1, 1))
        indices.extend((origin, origin + 2, origin + 1, origin, origin + 3, origin + 2))

    def add_box(b, center, size):
        cx, cy, cz = center
        sx, sy, sz = (v * 0.5 for v in size)
        corners = {
            "lbf": (cx - sx, cy - sy, cz - sz), "rbf": (cx + sx, cy - sy, cz - sz),
            "ltf": (cx - sx, cy + sy, cz - sz), "rtf": (cx + sx, cy + sy, cz - sz),
            "lbb": (cx - sx, cy - sy, cz + sz), "rbb": (cx + sx, cy - sy, cz + sz),
            "ltb": (cx - sx, cy + sy, cz + sz), "rtb": (cx + sx, cy + sy, cz + sz),
        }
        quad(b, (corners["rbf"], corners["rbb"], corners["rtb"], corners["rtf"]), (1, 0, 0))
        quad(b, (corners["lbb"], corners["lbf"], corners["ltf"], corners["ltb"]), (-1, 0, 0))
        quad(b, (corners["ltf"], corners["rtf"], corners["rtb"], corners["ltb"]), (0, 1, 0))
        quad(b, (corners["lbb"], corners["rbb"], corners["rbf"], corners["lbf"]), (0, -1, 0))
        quad(b, (corners["rbb"], corners["lbb"], corners["ltb"], corners["rtb"]), (0, 0, 1))
        quad(b, (corners["lbf"], corners["rbf"], corners["rtf"], corners["ltf"]), (0, 0, -1))

    def prim(b, material: int):
        p, n, u, c, i = b
        if not i:
            return None
        return {
            "attributes": {
                "POSITION": acc(p, bounds=True, target=34962),
                "NORMAL": acc(n, target=34962),
                "TEXCOORD_0": acc(u, kind="VEC2", target=34962),
                "COLOR_0": acc(c, kind="VEC4", target=34962),
            },
            "indices": acc(i, kind="SCALAR", component=5123, target=34963),
            "material": material,
            "mode": 4,
        }

    body_buckets = [bucket() for _ in range(len(PALETTE))]
    for ix, iy, iz in sorted(vox, key=lambda v: (v[1], v[2], v[0])):
        for normal, corners in faces:
            if (ix + normal[0], iy + normal[1], iz + normal[2]) in vox:
                continue
            pts = [
                (-body_width * 0.5 + (ix + px) * dx,
                 -body_height * 0.5 + (iy + py) * dy,
                 -body_depth * 0.5 + (iz + pz) * dz)
                for px, py, pz in corners
            ]
            quad(body_buckets[body_material(ix, iy, iz, normal)], pts, normal)

    meshes.append({
        "name": "pyro_blob_body",
        "primitives": [p for p in (prim(b, i) for i, b in enumerate(body_buckets)) if p],
    })
    body_mesh = len(meshes) - 1

    face_bucket = bucket()
    face_z = -body_depth * 0.5 - max(0.006, body_depth * 0.005)
    face_size = body_width * (0.86 / 1.20)
    face_y = -body_height * 0.075
    quad(face_bucket,
         ((face_size / 2, face_y - face_size / 2, face_z),
          (-face_size / 2, face_y - face_size / 2, face_z),
          (-face_size / 2, face_y + face_size / 2, face_z),
          (face_size / 2, face_y + face_size / 2, face_z)),
         (0, 0, -1))
    face_bucket[4][:] = [0, 1, 2, 0, 2, 3]
    meshes.append({"name": "slime_face_quad", "primitives": [prim(face_bucket, len(PALETTE))]})
    face_mesh = len(meshes) - 1

    s = body_width / 1.20
    fire_dark = bucket()
    fire_orange = bucket()
    fire_yellow = bucket()
    fire_hot = bucket()
    top = body_height * 0.5

    def flame_step(bucket_, x, y, z, w, h, d):
        add_box(bucket_, (x * s, top + y * s, z * s), (w * s, h * s, d * s))

    # Flames grow out of the blob itself. Each tongue is a staircase silhouette:
    # wider buried roots, then progressively smaller layers shifted sideways/up.
    if not large:
        # Dominant central flame.
        for x, y, z, w, h, d, mat in [
            (0.000, -0.090,  0.010, 0.39, 0.20, 0.33, fire_dark),
            (0.005,  0.045,  0.005, 0.33, 0.17, 0.29, fire_orange),
            (0.015,  0.165,  0.000, 0.26, 0.14, 0.24, fire_orange),
            (0.028,  0.265, -0.004, 0.19, 0.11, 0.18, fire_yellow),
            (0.044,  0.342, -0.008, 0.12, 0.085,0.13, fire_yellow),
            (0.060,  0.400, -0.012, 0.060,0.055,0.075,fire_hot),
        ]:
            flame_step(mat, x, y, z, w, h, d)

        # Left tongue leans outward by pixel stair-steps.
        for x, y, z, w, h, d, mat in [
            (-0.235, -0.075, 0.020, 0.25, 0.17, 0.25, fire_dark),
            (-0.275,  0.040, 0.015, 0.20, 0.14, 0.21, fire_orange),
            (-0.315,  0.138, 0.010, 0.14, 0.11, 0.16, fire_yellow),
            (-0.345,  0.215, 0.005, 0.075,0.070,0.10, fire_hot),
        ]:
            flame_step(mat, x, y, z, w, h, d)

        # Right tongue is shorter and offset, avoiding horn-like symmetry.
        for x, y, z, w, h, d, mat in [
            (0.245, -0.080,-0.005, 0.23, 0.16, 0.24, fire_dark),
            (0.278,  0.030,-0.010, 0.18, 0.13, 0.20, fire_orange),
            (0.305,  0.118,-0.014, 0.12, 0.10, 0.15, fire_yellow),
            (0.325,  0.185,-0.018, 0.060,0.065,0.09, fire_hot),
        ]:
            flame_step(mat, x, y, z, w, h, d)

        # Small rear lick gives the top a flame cluster instead of three spikes.
        for x, y, z, w, h, d, mat in [
            (-0.055,-0.050, 0.205,0.20,0.15,0.18,fire_dark),
            (-0.075, 0.050, 0.220,0.15,0.12,0.14,fire_orange),
            (-0.095, 0.130, 0.232,0.09,0.085,0.10,fire_yellow),
        ]:
            flame_step(mat, x, y, z, w, h, d)

        detail_height = 0.43 * s
        detail_width = 0.78 * s
    else:
        # Large variant has taller, denser flames but the same non-horn language.
        for x, y, z, w, h, d, mat in [
            (0.000, -0.105, 0.012, 0.44, 0.23, 0.37, fire_dark),
            (0.006,  0.050, 0.006, 0.38, 0.19, 0.33, fire_orange),
            (0.018,  0.185, 0.000, 0.30, 0.16, 0.27, fire_orange),
            (0.034,  0.300,-0.005, 0.22, 0.13, 0.21, fire_yellow),
            (0.052,  0.392,-0.010, 0.15, 0.10, 0.15, fire_yellow),
            (0.072,  0.463,-0.015, 0.085,0.075,0.10, fire_hot),
            (0.088,  0.515,-0.018, 0.045,0.045,0.055,fire_hot),
        ]:
            flame_step(mat, x, y, z, w, h, d)

        for x, y, z, w, h, d, mat in [
            (-0.260,-0.090, 0.025,0.29,0.19,0.28,fire_dark),
            (-0.305, 0.040, 0.018,0.23,0.16,0.24,fire_orange),
            (-0.350, 0.150, 0.012,0.17,0.13,0.19,fire_yellow),
            (-0.385, 0.238, 0.006,0.10,0.085,0.12,fire_hot),
            ( 0.270,-0.095,-0.010,0.27,0.18,0.27,fire_dark),
            ( 0.310, 0.025,-0.015,0.21,0.15,0.23,fire_orange),
            ( 0.345, 0.125,-0.020,0.15,0.12,0.17,fire_yellow),
            ( 0.372, 0.205,-0.025,0.08,0.075,0.11,fire_hot),
            (-0.070,-0.060, 0.225,0.23,0.17,0.20,fire_dark),
            (-0.095, 0.055, 0.242,0.17,0.14,0.16,fire_orange),
            (-0.120, 0.150, 0.255,0.10,0.10,0.11,fire_yellow),
            ( 0.145,-0.045, 0.185,0.19,0.15,0.18,fire_dark),
            ( 0.170, 0.055, 0.200,0.13,0.11,0.13,fire_orange),
        ]:
            flame_step(mat, x, y, z, w, h, d)

        detail_height = 0.54 * s
        detail_width = 0.86 * s

    details_prims = [
        prim(fire_dark, 9),
        prim(fire_orange, 7),
        prim(fire_yellow, 8),
        prim(fire_hot, 10),
    ]
    meshes.append({"name": "pyro_flame_tuft", "primitives": [p for p in details_prims if p]})
    details_mesh = len(meshes) - 1

    materials = [
        {
            "name": name,
            "pbrMetallicRoughness": {
                "baseColorFactor": [lin(rgb[0]), lin(rgb[1]), lin(rgb[2]), 1],
                "metallicFactor": 0,
                "roughnessFactor": 1,
            },
            "doubleSided": False,
            "extensions": {"KHR_materials_unlit": {}},
        }
        for name, rgb in PALETTE
    ]
    materials.append({
        "name": "SlimeFace",
        "pbrMetallicRoughness": {
            "baseColorFactor": [1, 1, 1, 1],
            "metallicFactor": 0,
            "roughnessFactor": 1,
        },
        "doubleSided": False,
        "alphaMode": "BLEND",
        "extensions": {"KHR_materials_unlit": {}},
    })

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

    if large:
        collider_size = [1.222, 1.2432, 1.224]
        collider_y = 0.6216
    else:
        collider_size = [0.78, 0.84, 0.7752]
        collider_y = 0.42

    root = node(
        "SlimeRoot",
        children=[],
        extras={
            "asteria_asset": f"creature/{asset_id}",
            "unit": "meters",
            "forward": "-Z",
            "collider": {"shape": "aabb", "size": collider_size, "offset": [0, collider_y, 0]},
            "collider_is_animated": False,
        },
    )
    visual = node("Visual", children=[])
    body_pivot = node("BodyPivot", children=[], translation=[0, half_h, 0])
    body_node = node("Shell", mesh=body_mesh)
    face_node = node("Face", mesh=face_mesh)
    details_node = node("PyroFlames", mesh=details_mesh)
    nodes[body_pivot]["children"] = [body_node, face_node, details_node]
    nodes[visual]["children"] = [body_pivot]
    hit = node(
        "Hitbox_AABB",
        translation=[0, collider_y, 0],
        extras={
            "asteria_collider": {"shape": "aabb", "size": collider_size, "solid": True, "targetable": True},
            "debug_display": False,
        },
    )
    nodes[root]["children"] = [visual, hit]

    def track(name, times, scales, center=None):
        center = center if center is not None else [half_h * scale_[1] for scale_ in scales]
        time_acc = acc(times, kind="SCALAR", bounds=True)
        channels, samplers = [], []

        def add(target, path, values, kind):
            out = acc([v for row in values for v in row], kind=kind)
            samplers.append({"input": time_acc, "output": out, "interpolation": "LINEAR"})
            channels.append({"sampler": len(samplers) - 1, "target": {"node": target, "path": path}})

        add(body_pivot, "scale", scales, "VEC3")
        add(body_pivot, "translation", [[0, y, 0] for y in center], "VEC3")
        animations.append({
            "name": name,
            "channels": channels,
            "samplers": samplers,
            "extras": {"loop_recommended": name in ("Idle", "Airborne")},
        })

    track("Idle", [0, .5, 1, 1.5, 2], [[1, 1, 1], [1.018, .974, 1.018], [1, 1, 1], [.988, 1.021, .988], [1, 1, 1]])
    track("Anticipate", [0, .07, .17, .24], [[1, 1, 1], [1.09, .845, 1.09], [1.125, .76, 1.125], [1.09, .83, 1.09]])
    track("Airborne", [0, .12, .35, .55, .72], [[1.09, .83, 1.09], [.91, 1.15, .91], [.96, 1.085, .96], [.97, 1.06, .97], [1, 1, 1]])
    track("Land", [0, .045, .12, .20, .34], [[.97, 1.055, .97], [1.17, .74, 1.17], [1.12, .805, 1.12], [.975, 1.047, .975], [1, 1, 1]])
    track("Hurt", [0, .085, .15, .24, .38], [[1, 1, 1], [1.1, .88, 1.1], [.94, 1.08, .94], [1.025, .968, 1.025], [1, 1, 1]])
    track("Death", [0, .12, .31, .55, .75], [[1, 1, 1], [1.13, .8, 1.13], [1.2, .60, 1.2], [1.12, .19, 1.12], [.001, .001, .001]], center=[half_h, half_h * .8, half_h * .6, half_h * .19, half_h * .001])

    visual_height = body_height + detail_height
    visual_width = max(body_width, detail_width)
    scene = {
        "asset": {"version": "2.0", "generator": "Asteria Pyro flame rebuild v1"},
        "scene": 0,
        "scenes": [{"name": "PyroSlime", "nodes": [root]}],
        "extensionsUsed": ["KHR_materials_unlit"],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "animations": animations,
        "bufferViews": views,
        "accessors": accessors,
        "buffers": [{"byteLength": len(binary)}],
        "extras": {
            "asset_id": f"asteria:{asset_id}",
            "base_language": "slime_blob",
            "face_source": "textures/creatures/slime_pyro/face.png",
            "collision_source": f"{asset_id}.collider.json",
            "voxel_resolution": [NX, NY, NZ],
            "occupied_voxels": len(vox),
            "visual_height": visual_height,
            "visual_width": visual_width,
            "pixel_art_geometry": "Pyro flames use only axis-aligned voxel boxes; flame curves and leans are expressed through staircase layer offsets",
            "reference_design": "clean orange-gold blob with no horns; several asymmetric flame tongues grow directly from the top",
        },
    }
    json_chunk = json.dumps(scene, separators=(",", ":")).encode("utf-8")
    json_chunk += b" " * (-len(json_chunk) % 4)
    bin_chunk = bytes(binary) + b"\0" * (-len(binary) % 4)
    glb = (
        struct.pack("<4sII", b"glTF", 2, 12 + 8 + len(json_chunk) + 8 + len(bin_chunk))
        + struct.pack("<I4s", len(json_chunk), b"JSON") + json_chunk
        + struct.pack("<I4s", len(bin_chunk), b"BIN\0") + bin_chunk
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(glb)
    return {"bytes": len(glb), "occupied_voxels": len(vox), "visual_height": visual_height, "visual_width": visual_width}


def main() -> None:
    small = generate("slime_pyro", ROOT / "slime_pyro.glb", 1.20, 1.00, 1.14, False)
    large_dir = ROOT.parent / "slime_pyro_large"
    large = generate("slime_pyro_large", large_dir / "slime_pyro_large.glb", 1.88, 1.48, 1.80, True)
    print("small", small)
    print("large", large)


if __name__ == "__main__":
    main()
