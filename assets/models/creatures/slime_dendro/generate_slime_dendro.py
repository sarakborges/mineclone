#!/usr/bin/env python3
"""Generate Dendro Slime models with strict pixel-art detail geometry.

The rounded slime body remains voxel-authored. Every Dendro-specific feature
above it is built only from axis-aligned boxes and orthogonal stepped paths:
no smooth curves, rotated prisms, or straight diagonal geometry.
"""
from __future__ import annotations

import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent
NX, NY, NZ = 24, 20, 22

PALETTE = (
    ("SlimeShell", (0.55, 0.68, 0.42)),
    ("SlimeShellCenter", (0.68, 0.78, 0.54)),
    ("SlimeShellOuter", (0.34, 0.48, 0.24)),
    ("SlimeShellBottom", (0.38, 0.46, 0.29)),
    ("SlimeShellTop", (0.50, 0.63, 0.32)),
    ("SlimeShellLight", (0.75, 0.84, 0.62)),
    ("SlimeShellBright", (0.88, 0.91, 0.75)),
    ("ElementAccent", (0.30, 0.63, 0.13)),
    ("ElementPale", (0.55, 0.82, 0.20)),
    ("ElementDark", (0.12, 0.34, 0.07)),
    ("ElementWhite", (0.78, 0.90, 0.42)),
    ("FlowerOrange", (1.00, 0.46, 0.07)),
    ("FlowerOrangeLight", (1.00, 0.68, 0.16)),
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
            if gy < 0.13:
                return 3
            if abs(gx) > 0.72:
                return 2
            if gy > 0.78:
                return 4
            if abs(gx) < 0.58 and 0.20 < gy < 0.72:
                return 1
            return 0
        if normal == (0, 1, 0):
            return 5 if gy > 0.72 else 4
        if normal == (0, -1, 0):
            return 3
        if normal == (1, 0, 0):
            return 2
        if normal == (-1, 0, 0):
            return 4 if gy > 0.62 else 0
        if normal == (0, 0, 1):
            return 2 if abs(gx) > 0.66 or abs(gz) > 0.66 else 0
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

    def add_axis_segment(b, start, end, thickness, depth=None):
        """Add one orthogonal cuboid segment; diagonal endpoints are rejected."""
        dx_, dy_, dz_ = end[0] - start[0], end[1] - start[1], end[2] - start[2]
        moving = sum(abs(v) > 1e-7 for v in (dx_, dy_, dz_))
        if moving == 0:
            return
        if moving != 1:
            raise ValueError(f"pixel-art segment must be axis-aligned: {start} -> {end}")
        cx, cy, cz = ((start[i] + end[i]) * 0.5 for i in range(3))
        d = depth if depth is not None else thickness
        sx, sy, sz = thickness, thickness, d
        if abs(dx_) > 1e-7:
            sx = abs(dx_) + thickness
        elif abs(dy_) > 1e-7:
            sy = abs(dy_) + thickness
        else:
            sz = abs(dz_) + d
        add_box(b, (cx, cy, cz), (sx, sy, sz))

    def add_pixel_path(b, points, thickness, depth=None):
        """Draw a Manhattan/stepped path from axis-aligned cuboids only."""
        for a, c in zip(points, points[1:]):
            add_axis_segment(b, a, c, thickness, depth)

    def add_pixel_leaf(b, base, steps, cell, wide=2.0, thickness=None):
        """Blocky leaf: staircase spine plus tapered axis-aligned plates."""
        thickness = thickness if thickness is not None else cell * 0.48
        x, y, z = base
        points = [(x, y, z)]
        cells = []
        for i, (axis, sign) in enumerate(steps):
            if axis == "x":
                x += sign * cell
            elif axis == "y":
                y += sign * cell
            elif axis == "z":
                z += sign * cell
            else:
                raise ValueError(axis)
            points.append((x, y, z))
            taper = 1.0 if i < max(1, len(steps) // 2) else 0.72
            cells.append((x, y, z, taper))
        add_pixel_path(b, points, max(cell * 0.36, thickness * 0.85), thickness)
        # Plates are intentionally axis-aligned. Their changing size forms the pixel silhouette.
        dominant_x = sum(1 for axis, _ in steps if axis == "x") >= sum(1 for axis, _ in steps if axis == "z")
        for cx, cy, cz, taper in cells:
            if dominant_x:
                size = (cell * 1.18, cell * wide * taper, thickness)
            else:
                size = (cell * wide * taper, cell * 1.18, thickness)
            add_box(b, (cx, cy, cz), size)

    def add_square_hook(b, origin, cell, mirror=1.0, z=0.0):
        """A tiny square spiral/tendril made of orthogonal steps."""
        x, y, _ = origin
        pts = [
            (x, y, z),
            (x + mirror * 2.0 * cell, y, z),
            (x + mirror * 2.0 * cell, y + 2.0 * cell, z),
            (x + mirror * 0.7 * cell, y + 2.0 * cell, z),
            (x + mirror * 0.7 * cell, y + 0.9 * cell, z),
            (x + mirror * 1.35 * cell, y + 0.9 * cell, z),
        ]
        add_pixel_path(b, pts, cell * 0.42, cell * 0.40)

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

    body_buckets = [bucket() for _ in range(7)]
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
        "name": "dendro_blob_body",
        "primitives": [p for p in (prim(b, i) for i, b in enumerate(body_buckets)) if p],
    })
    body_mesh = len(meshes) - 1

    face_bucket = bucket()
    face_z = -body_depth * 0.5 - max(0.006, body_depth * 0.005)
    face_size = body_width * (0.86 / 1.20)
    face_w = face_size
    face_h = face_size
    face_y = -body_height * 0.075
    quad(face_bucket,
         ((face_w / 2, face_y - face_h / 2, face_z),
          (-face_w / 2, face_y - face_h / 2, face_z),
          (-face_w / 2, face_y + face_h / 2, face_z),
          (face_w / 2, face_y + face_h / 2, face_z)),
         (0, 0, -1))
    face_bucket[4][:] = [0, 1, 2, 0, 2, 3]
    meshes.append({"name": "slime_face_quad", "primitives": [prim(face_bucket, len(PALETTE))]})
    face_mesh = len(meshes) - 1

    s = body_width / 1.20
    top = body_height * 0.5
    leaf = bucket()
    pale = bucket()
    dark = bucket()
    orange = bucket()
    orange_light = bucket()

    if not large:
        # Small reference: thin central sprout, stepped fern fronds and square-hook tendril.
        c = 0.055 * s
        stalk = [
            (0.0, top - 0.01 * s, 0.02 * s),
            (0.0, top + 0.15 * s, 0.02 * s),
            (-0.05 * s, top + 0.15 * s, 0.02 * s),
            (-0.05 * s, top + 0.33 * s, 0.02 * s),
            (-0.10 * s, top + 0.33 * s, 0.02 * s),
            (-0.10 * s, top + 0.49 * s, 0.02 * s),
        ]
        add_pixel_path(dark, stalk, 0.030 * s, 0.026 * s)

        # Upright blades. Diagonal silhouettes are staircases rather than rotated strips.
        add_pixel_leaf(leaf, (-0.04 * s, top + 0.18 * s, 0.015 * s),
                       [("y", 1), ("x", -1), ("y", 1), ("x", -1), ("y", 1), ("y", 1)], c, 1.45, 0.020 * s)
        add_pixel_leaf(pale, (-0.01 * s, top + 0.19 * s, 0.00),
                       [("y", 1), ("y", 1), ("x", 1), ("y", 1), ("y", 1)], c, 1.35, 0.020 * s)
        add_pixel_leaf(leaf, (0.02 * s, top + 0.16 * s, 0.03 * s),
                       [("x", 1), ("y", 1), ("x", 1), ("y", 1), ("x", 1)], c, 1.30, 0.020 * s)

        # Left/right fern spines use explicit Manhattan steps.
        left_spine = [
            (-0.02 * s, top + 0.13 * s, 0.04 * s),
            (-0.16 * s, top + 0.13 * s, 0.04 * s),
            (-0.16 * s, top + 0.18 * s, 0.04 * s),
            (-0.31 * s, top + 0.18 * s, 0.04 * s),
            (-0.31 * s, top + 0.23 * s, 0.04 * s),
            (-0.48 * s, top + 0.23 * s, 0.04 * s),
        ]
        right_spine = [
            (0.02 * s, top + 0.14 * s, 0.02 * s),
            (0.15 * s, top + 0.14 * s, 0.02 * s),
            (0.15 * s, top + 0.20 * s, 0.02 * s),
            (0.30 * s, top + 0.20 * s, 0.02 * s),
            (0.30 * s, top + 0.25 * s, 0.02 * s),
            (0.47 * s, top + 0.25 * s, 0.02 * s),
        ]
        add_pixel_path(dark, left_spine, 0.026 * s, 0.024 * s)
        add_pixel_path(dark, right_spine, 0.026 * s, 0.024 * s)

        # Leaflets are tiny block clusters, alternating above/below the spine.
        for x, y, sign in [(-0.15, 0.15, 1), (-0.25, 0.19, -1), (-0.36, 0.23, 1), (-0.44, 0.24, -1)]:
            base=(x*s, top+y*s, 0.035*s)
            add_pixel_leaf(leaf if sign > 0 else pale, base, [("y", sign), ("x", -1), ("y", sign)], 0.040*s, 1.25, 0.018*s)
        for x, y, sign in [(0.14, 0.16, 1), (0.24, 0.20, -1), (0.35, 0.24, 1), (0.43, 0.26, -1)]:
            base=(x*s, top+y*s, 0.015*s)
            add_pixel_leaf(leaf if sign > 0 else pale, base, [("y", sign), ("x", 1), ("y", sign)], 0.040*s, 1.25, 0.018*s)

        # Reference-like curled vine, but squared off as pixel art.
        add_square_hook(dark, (-0.50 * s, top + 0.24 * s, 0.0), 0.050 * s, -1.0, -0.06 * s)
        detail_height = 0.58 * s
        detail_radius = 0.62 * s
        notes = "Strict pixel-art Dendro: clean blob with stepped sprout, fern blocks and square-hook vine."
    else:
        # Large reference: stepped fern collar supporting a broad pixel-art orange flower.
        c = 0.070 * s
        flower_y = top + 0.11 * s
        add_axis_segment(dark, (0.0, top - 0.01 * s, 0.0), (0.0, flower_y + 0.05 * s, 0.0), 0.040 * s)

        # Low foliage under the flower.
        collar_specs = [
            ((-0.02, 0.06, 0.02), [("x", -1), ("x", -1), ("y", 1), ("x", -1), ("x", -1)]),
            ((0.02, 0.07, 0.01), [("x", 1), ("x", 1), ("y", 1), ("x", 1), ("x", 1)]),
            ((-0.02, 0.05, 0.03), [("z", 1), ("z", 1), ("y", 1), ("x", -1), ("z", 1)]),
            ((0.02, 0.05, -0.02), [("z", -1), ("z", -1), ("y", 1), ("x", 1), ("z", -1)]),
        ]
        for idx, (origin, steps) in enumerate(collar_specs):
            base=(origin[0]*s, top+origin[1]*s, origin[2]*s)
            add_pixel_leaf(leaf if idx % 2 == 0 else pale, base, steps, c, 1.55, 0.025*s)

        # Pixel flower petals. Six distinct lobes built as chunky orthogonal terraces.
        def petal(cells, highlight_cells=()):
            for xoff, yoff, zoff, sx_, sy_, sz_ in cells:
                add_box(orange, (xoff*s, flower_y+yoff*s, zoff*s), (sx_*s, sy_*s, sz_*s))
            for xoff, yoff, zoff, sx_, sy_, sz_ in highlight_cells:
                add_box(orange_light, (xoff*s, flower_y+yoff*s, zoff*s), (sx_*s, sy_*s, sz_*s))

        # Left / right petals dominate the silhouette, with clear stepped shoulders and tips.
        side_cells = [
            (0.18, 0.08, 0.00, 0.24, 0.10, 0.26),
            (0.38, 0.13, 0.00, 0.28, 0.10, 0.30),
            (0.58, 0.09, 0.00, 0.27, 0.10, 0.28),
            (0.75, 0.02, 0.00, 0.20, 0.09, 0.22),
        ]
        side_hi = [
            (0.30, 0.18, -0.03, 0.10, 0.032, 0.11),
            (0.52, 0.15, -0.03, 0.10, 0.032, 0.11),
        ]
        petal(side_cells, side_hi)
        petal([(-x,y,z,sx_,sy_,sz_) for x,y,z,sx_,sy_,sz_ in side_cells],
              [(-x,y,z,sx_,sy_,sz_) for x,y,z,sx_,sy_,sz_ in side_hi])

        # Rear pair rises above the side petals. Their diagonal direction is represented by a staircase.
        rear_right = [
            (0.10, 0.14, 0.12, 0.20, 0.09, 0.20),
            (0.22, 0.20, 0.24, 0.24, 0.10, 0.22),
            (0.34, 0.22, 0.37, 0.23, 0.10, 0.22),
            (0.45, 0.17, 0.49, 0.18, 0.09, 0.18),
        ]
        rear_hi = [(0.25,0.25,0.27,0.09,0.03,0.09),(0.38,0.25,0.40,0.08,0.03,0.08)]
        petal(rear_right, rear_hi)
        petal([(-x,y,z,sx_,sy_,sz_) for x,y,z,sx_,sy_,sz_ in rear_right],
              [(-x,y,z,sx_,sy_,sz_) for x,y,z,sx_,sy_,sz_ in rear_hi])

        # Front pair droops below the side petals, matching the reference flower's lower lobes.
        front_right = [
            (0.10, 0.03, -0.12, 0.20, 0.09, 0.20),
            (0.23, 0.00, -0.25, 0.25, 0.10, 0.23),
            (0.36, -0.05, -0.38, 0.23, 0.10, 0.21),
            (0.47, -0.10, -0.49, 0.17, 0.09, 0.17),
        ]
        front_hi = [(0.26,0.04,-0.28,0.09,0.03,0.09)]
        petal(front_right, front_hi)
        petal([(-x,y,z,sx_,sy_,sz_) for x,y,z,sx_,sy_,sz_ in front_right],
              [(-x,y,z,sx_,sy_,sz_) for x,y,z,sx_,sy_,sz_ in front_hi])

        # Green sepals remain visible between petals.
        for sx, sz in ((-1,-1),(1,-1),(-1,1),(1,1)):
            base=(0.0,flower_y-0.03*s,0.0)
            steps=[("x",sx),("z",sz),("x",sx),("y",-1),("z",sz)]
            add_pixel_leaf(dark,base,steps,0.055*s,1.25,0.022*s)

        # Compact central bud: a stepped voxel pyramid, never a diagonal prism.
        add_box(leaf,(0.0,flower_y+0.09*s,0.0),(0.34*s,0.11*s,0.32*s))
        add_box(pale,(0.0,flower_y+0.17*s,-0.01*s),(0.26*s,0.10*s,0.24*s))
        add_box(leaf,(0.0,flower_y+0.245*s,-0.015*s),(0.18*s,0.09*s,0.17*s))
        add_box(dark,(0.0,flower_y+0.305*s,-0.02*s),(0.09*s,0.07*s,0.09*s))

        # Square-hook tendrils peeking from both sides.
        add_square_hook(dark,(-0.58*s,top+0.12*s,0.0),0.055*s,-1.0,-0.18*s)
        add_square_hook(dark,(0.58*s,top+0.12*s,0.0),0.055*s,1.0,-0.12*s)
        detail_height = 0.45 * s
        detail_radius = 0.86 * s
        notes = "Strict pixel-art Dendro large: stepped orange flower, voxel bud, fern blocks and square-hook vines."

    detail_primitives = [
        prim(leaf, 7),
        prim(pale, 8),
        prim(dark, 9),
        prim(orange, 11),
        prim(orange_light, 12),
    ]
    meshes.append({"name": "dendro_top_growth", "primitives": [p for p in detail_primitives if p]})
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
        collider_size = [1.235, 1.2432, 1.2376]
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
    details_node = node("DendroGrowth", mesh=details_mesh)
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
        center = center if center is not None else [half_h * scale[1] for scale in scales]
        time_acc = acc(times, kind="SCALAR", bounds=True)
        channels, samplers = [], []

        def add_track(target, path, values, kind):
            out = acc([v for row in values for v in row], kind=kind)
            samplers.append({"input": time_acc, "output": out, "interpolation": "LINEAR"})
            channels.append({"sampler": len(samplers) - 1, "target": {"node": target, "path": path}})

        add_track(body_pivot, "scale", scales, "VEC3")
        add_track(body_pivot, "translation", [[0, y, 0] for y in center], "VEC3")
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
    scene = {
        "asset": {"version": "2.0", "generator": "Asteria Dendro slime pixel-art rebuild v2"},
        "scene": 0,
        "scenes": [{"name": "DendroSlime", "nodes": [root]}],
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
            "face_source": "textures/creatures/slime_dendro/face.png",
            "collision_source": f"{asset_id}.collider.json",
            "voxel_resolution": [NX, NY, NZ],
            "occupied_voxels": len(vox),
            "visual_height": visual_height,
            "visual_width": max(body_width, detail_radius * 2),
            "pixel_art_geometry": True,
            "notes": notes,
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
    return {"bytes": len(glb), "occupied_voxels": len(vox), "visual_height": visual_height}


def main() -> None:
    small = generate("slime_dendro", ROOT / "slime_dendro.glb", 1.20, 1.00, 1.14, False)
    large_dir = ROOT.parent / "slime_dendro_large"
    large = generate("slime_dendro_large", large_dir / "slime_dendro_large.glb", 1.92, 1.38, 1.82, True)
    print("small", small)
    print("large", large)


if __name__ == "__main__":
    main()
