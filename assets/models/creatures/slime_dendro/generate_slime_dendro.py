#!/usr/bin/env python3
"""Generate Dendro Slime models from the rounded slime_blob body language.

The body stays a clean pale-green blob. Dendro identity lives entirely above
it: the small slime carries a leafy fern/sprout crown with curled tendrils; the
large slime carries a broad orange flower with a green bud and surrounding
foliage. The face remains a square texture-driven SlimeFace decal.
"""
from __future__ import annotations

import json
import math
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
    ("FlowerOrange", (0.98, 0.34, 0.055)),
    ("FlowerOrangeLight", (1.00, 0.55, 0.10)),
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


def vadd(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def vsub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def vmul(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


def vdot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def vcross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def vnorm(a):
    length = math.sqrt(vdot(a, a))
    if length < 1e-9:
        return (0.0, 1.0, 0.0)
    return (a[0] / length, a[1] / length, a[2] / length)


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
        # Clean body: these are only broad authored light/depth zones, never Dendro motifs.
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

    def add_prism(b, center, axis, side, normal, length, width, depth):
        axis, side, normal = vnorm(axis), vnorm(side), vnorm(normal)
        ah, sh, nh = length * 0.5, width * 0.5, depth * 0.5
        c = center
        def pt(a, s, n):
            return vadd(c, vadd(vmul(axis, a * ah), vadd(vmul(side, s * sh), vmul(normal, n * nh))))
        p000, p001 = pt(-1, -1, -1), pt(-1, -1, 1)
        p010, p011 = pt(-1, 1, -1), pt(-1, 1, 1)
        p100, p101 = pt(1, -1, -1), pt(1, -1, 1)
        p110, p111 = pt(1, 1, -1), pt(1, 1, 1)
        quad(b, (p100, p110, p111, p101), axis)
        quad(b, (p010, p000, p001, p011), vmul(axis, -1))
        quad(b, (p110, p010, p011, p111), side)
        quad(b, (p000, p100, p101, p001), vmul(side, -1))
        quad(b, (p101, p111, p011, p001), normal)
        quad(b, (p000, p010, p110, p100), vmul(normal, -1))

    def add_segment(b, start, end, thickness, depth=None):
        axis = vsub(end, start)
        length = math.sqrt(vdot(axis, axis))
        if length < 1e-6:
            return
        axis_n = vnorm(axis)
        helper = (0.0, 0.0, 1.0) if abs(axis_n[2]) < 0.90 else (1.0, 0.0, 0.0)
        side = vnorm(vcross(helper, axis_n))
        normal = vnorm(vcross(axis_n, side))
        center = vmul(vadd(start, end), 0.5)
        add_prism(b, center, axis_n, side, normal, length, thickness, depth or thickness)

    def add_leaf(b, start, end, width, thickness=0.025, tilt=0.0):
        # A pixel-art leaf made from two tapered-looking cuboids: broad base/mid + narrow tip.
        mid = vadd(start, vmul(vsub(end, start), 0.62))
        axis1 = vsub(mid, start)
        axis2 = vsub(end, mid)
        for a, c, w in ((start, mid, width), (mid, end, width * 0.62)):
            axis = vsub(c, a)
            length = math.sqrt(vdot(axis, axis))
            if length < 1e-6:
                continue
            axis_n = vnorm(axis)
            helper = (0.0, 1.0, 0.0)
            if abs(vdot(axis_n, helper)) > 0.90:
                helper = (0.0, 0.0, 1.0)
            side = vnorm(vcross(helper, axis_n))
            normal = vnorm(vcross(axis_n, side))
            if tilt:
                # Rotate leaf plane around its long axis.
                ct, st = math.cos(tilt), math.sin(tilt)
                side, normal = vadd(vmul(side, ct), vmul(normal, st)), vadd(vmul(normal, ct), vmul(side, -st))
            add_prism(b, vmul(vadd(a, c), 0.5), axis_n, side, normal, length, w, thickness)

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

    meshes.append({
        "name": "dendro_blob_body",
        "primitives": [p for p in (prim(b, i) for i, b in enumerate(body_buckets)) if p],
    })
    body_mesh = len(meshes) - 1

    # Square face plane for the 64x64 face texture.
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

    scale = body_width / 1.20
    leaf = bucket()
    pale = bucket()
    dark = bucket()
    orange = bucket()
    orange_light = bucket()
    top = body_height * 0.5

    def curved_stem(points, thickness):
        for a, b in zip(points, points[1:]):
            add_segment(dark, a, b, thickness)

    def fern(origin, direction, length, rise, leaflet_count=4, width=0.065):
        # Three-segment arc keeps the fronds organic instead of ruler-straight.
        dxy = vnorm((direction[0], 0.0, direction[2]))
        p1 = (origin[0] + dxy[0] * length * 0.34, origin[1] + rise * 0.24, origin[2] + dxy[2] * length * 0.34)
        p2 = (origin[0] + dxy[0] * length * 0.70, origin[1] + rise * 0.68, origin[2] + dxy[2] * length * 0.70)
        end = (origin[0] + dxy[0] * length, origin[1] + rise, origin[2] + dxy[2] * length)
        points = [origin, p1, p2, end]
        curved_stem(points, 0.022 * scale)
        side = (-dxy[2], 0.0, dxy[0])
        for i in range(1, leaflet_count + 1):
            t = i / (leaflet_count + 1)
            if t < 0.34:
                a, b, local = origin, p1, t / 0.34
            elif t < 0.70:
                a, b, local = p1, p2, (t - 0.34) / 0.36
            else:
                a, b, local = p2, end, (t - 0.70) / 0.30
            base = vadd(a, vmul(vsub(b, a), local))
            leaflet = length * (0.20 - 0.018 * i)
            for sign in (-1, 1):
                tip = (
                    base[0] + side[0] * leaflet * sign + dxy[0] * leaflet * 0.12,
                    base[1] + (0.025 + 0.018 * (1.0 - t)) * scale,
                    base[2] + side[2] * leaflet * sign + dxy[2] * leaflet * 0.12,
                )
                add_leaf(leaf if (i + (sign > 0)) % 2 else pale, base, tip,
                         width * scale * (1.0 - 0.06 * i), 0.018 * scale, sign * 0.12)
        add_leaf(pale, p2, end, width * scale * 0.70, 0.018 * scale)

    def tendril(center, radius, start_angle, sweep, y0, y1, front_z=0.0, segments=10):
        pts = []
        for i in range(segments + 1):
            t = i / segments
            a = start_angle + sweep * t
            r = radius * (1.0 - 0.66 * t)
            pts.append((center[0] + math.cos(a) * r, y0 + (y1 - y0) * t,
                        front_z + math.sin(a) * r * 0.58))
        curved_stem(pts, 0.017 * scale)

    def add_petal(bucket_main, bucket_hi, angle, length, max_half_width, y_center, arch, droop):
        # Broad extruded fan with a gently arched top and drooping outer lip.
        radial = (math.cos(angle), 0.0, math.sin(angle))
        side = (-radial[2], 0.0, radial[0])
        sections = [
            (0.06, max_half_width * 0.28, y_center),
            (0.34, max_half_width * 0.78, y_center + arch),
            (0.68, max_half_width, y_center + arch * 0.55),
            (1.00, max_half_width * 0.68, y_center - droop),
        ]
        thickness = 0.055 * scale
        top_pts = []
        bot_pts = []
        for t, hw, yy in sections:
            center = (radial[0] * length * scale * t, yy, radial[2] * length * scale * t)
            left = vadd(center, vmul(side, hw * scale))
            right = vadd(center, vmul(side, -hw * scale))
            top_pts.append((left, right))
            bot_pts.append(((left[0], left[1] - thickness, left[2]), (right[0], right[1] - thickness, right[2])))
        # top and bottom strips
        for i in range(len(sections) - 1):
            l0, r0 = top_pts[i]; l1, r1 = top_pts[i + 1]
            quad(bucket_main, (r0, l0, l1, r1), (0, 1, 0))
            bl0, br0 = bot_pts[i]; bl1, br1 = bot_pts[i + 1]
            quad(bucket_main, (br1, bl1, bl0, br0), (0, -1, 0))
            # left/right thickness walls
            quad(bucket_main, (l0, bl0, bl1, l1), side)
            quad(bucket_main, (br0, r0, r1, br1), vmul(side, -1))
        # outer cap
        l, r = top_pts[-1]; bl, br = bot_pts[-1]
        quad(bucket_main, (r, l, bl, br), radial)
        # inner warm stripe follows the petal, deliberately thin and inset.
        h0 = (radial[0] * length * scale * 0.16, y_center + 0.018 * scale, radial[2] * length * scale * 0.16)
        h1 = (radial[0] * length * scale * 0.58, y_center + arch * 0.56 + 0.022 * scale,
              radial[2] * length * scale * 0.58)
        add_leaf(bucket_hi, h0, h1, max_half_width * 0.34 * scale, 0.018 * scale)

    if not large:
        # Reference-like small crown: thin sprouting blades, side ferns and one curled tendril.
        stem_base = (0.0, top - 0.004 * scale, 0.015 * scale)
        curved_stem([
            stem_base,
            (-0.015 * scale, top + 0.17 * scale, 0.01 * scale),
            (-0.055 * scale, top + 0.36 * scale, 0.025 * scale),
            (-0.10 * scale, top + 0.50 * scale, 0.04 * scale),
        ], 0.024 * scale)
        blades = [
            ((-0.015, 0.13, 0.01), (-0.23, 0.34, -0.03), 0.062),
            ((-0.02, 0.18, 0.01), (0.18, 0.43, -0.01), 0.058),
            ((-0.045, 0.28, 0.02), (-0.16, 0.55, 0.04), 0.055),
            ((-0.035, 0.25, 0.03), (0.07, 0.52, 0.07), 0.050),
            ((0.0, 0.10, 0.04), (0.27, 0.27, 0.01), 0.050),
        ]
        for i, (a, b, w) in enumerate(blades):
            aa = (a[0] * scale, top + a[1] * scale, a[2] * scale)
            bb = (b[0] * scale, top + b[1] * scale, b[2] * scale)
            add_leaf(leaf if i % 2 == 0 else pale, aa, bb, w * scale, 0.018 * scale, (-0.12 if i % 2 else 0.10))
        fern((-0.035 * scale, top + 0.06 * scale, 0.035 * scale), (-0.96, 0, -0.28), 0.44 * scale, 0.13 * scale, 5, 0.060)
        fern((0.035 * scale, top + 0.07 * scale, 0.045 * scale), (0.91, 0, -0.42), 0.40 * scale, 0.16 * scale, 5, 0.058)
        fern((0.015 * scale, top + 0.08 * scale, 0.08 * scale), (0.36, 0, 0.93), 0.30 * scale, 0.18 * scale, 4, 0.054)
        tendril((-0.34 * scale, 0, 0), 0.15 * scale, 0.18, math.pi * 1.75,
                top + 0.17 * scale, top + 0.28 * scale, -0.09 * scale, 11)
        detail_height = 0.56 * scale
        detail_radius = 0.54 * scale
    else:
        # Large reference: low fern collar + a broad orange flower sitting directly on the blob.
        flower_y = top + 0.10 * scale
        for angle, length, rise in [
            (math.radians(195), 0.56, 0.07),
            (math.radians(-15), 0.58, 0.08),
            (math.radians(145), 0.46, 0.11),
            (math.radians(35), 0.48, 0.11),
            (math.radians(265), 0.40, 0.05),
        ]:
            fern((0.0, top + 0.04 * scale, 0.02 * scale), (math.cos(angle), 0, math.sin(angle)),
                 length * scale, rise * scale, 5, 0.060)

        # Two broad lateral petals dominate, with four shorter petals filling the flower disk.
        for angle, length, width, arch, droop in [
            (math.radians(0),   0.78, 0.27, 0.10, 0.09),
            (math.radians(180), 0.78, 0.27, 0.10, 0.09),
            (math.radians(55),  0.63, 0.24, 0.13, 0.06),
            (math.radians(125), 0.63, 0.24, 0.13, 0.06),
            (math.radians(235), 0.58, 0.23, 0.07, 0.12),
            (math.radians(305), 0.58, 0.23, 0.07, 0.12),
        ]:
            add_petal(orange, orange_light, angle, length, width, flower_y,
                      arch * scale, droop * scale)

        # Green sepals visibly poke from under the orange petals.
        for angle in (35, 145, 215, 325):
            r = math.radians(angle)
            base = (0.0, flower_y - 0.025 * scale, 0.0)
            tip = (math.cos(r) * 0.34 * scale, flower_y - 0.12 * scale,
                   math.sin(r) * 0.34 * scale)
            add_leaf(dark, base, tip, 0.13 * scale, 0.028 * scale)

        # Compact pointed green bud; much lower than the petals' span, as in the reference.
        bud_base_y = flower_y + 0.015 * scale
        bud_apex = (0.0, flower_y + 0.25 * scale, -0.015 * scale)
        for angle in (35, 145, 215, 325):
            r = math.radians(angle)
            base = (math.cos(r) * 0.13 * scale, bud_base_y,
                    math.sin(r) * 0.13 * scale)
            add_leaf(leaf if angle in (35, 215) else pale, base, bud_apex,
                     0.14 * scale, 0.032 * scale, (0.14 if angle in (35, 215) else -0.14))
        add_segment(dark, (0.0, top + 0.01 * scale, 0.0),
                    (0.0, flower_y + 0.055 * scale, 0.0), 0.034 * scale)
        tendril((-0.48 * scale, 0, 0), 0.18 * scale, 0.0, math.pi * 1.75,
                top + 0.12 * scale, top + 0.24 * scale, -0.22 * scale, 11)
        tendril((0.50 * scale, 0, 0), 0.15 * scale, math.pi, -math.pi * 1.55,
                top + 0.11 * scale, top + 0.22 * scale, -0.12 * scale, 10)
        detail_height = 0.40 * scale
        detail_radius = 0.80 * scale

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
    visual_width = max(body_width, detail_radius * 2)
    scene = {
        "asset": {"version": "2.0", "generator": "Asteria Dendro slime reference rebuild v1"},
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
            "visual_width": visual_width,
            "reference_design": "clean pale-green blob; foliage above body; large variant has orange flower and green central bud",
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
    return {
        "bytes": len(glb),
        "occupied_voxels": len(vox),
        "visual_height": visual_height,
        "visual_width": visual_width,
    }


def main() -> None:
    small = generate("slime_dendro", ROOT / "slime_dendro.glb", 1.20, 1.00, 1.14, False)
    large_dir = ROOT.parent / "slime_dendro_large"
    large = generate("slime_dendro_large", large_dir / "slime_dendro_large.glb", 1.92, 1.38, 1.82, True)
    print("small", small)
    print("large", large)


if __name__ == "__main__":
    main()
