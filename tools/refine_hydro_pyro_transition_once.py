#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_detail_block(path: Path, replacement: str) -> str:
    text = path.read_text()
    start = text.index('    s = body_width / 1.20')
    end = text.index('    materials = [', start)
    return text[:start] + replacement + text[end:]


hydro_path = ROOT / 'assets/models/creatures/slime_hydro/generate_slime_hydro.py'
hydro = hydro_path.read_text()

hydro_block = r'''    s = body_width / 1.20
    top = body_height * 0.5

    # Hydro is not an accessory sitting on the blob. This mesh is a voxel
    # continuation of the slime's own top surface: broad at the root, then
    # narrowing into a water-drop point. Materials come from the body palette
    # so the root matches the visible top exactly and the remaining faces keep
    # the same voxel shading language instead of flat-colored slabs.
    drop_buckets = [bucket() for _ in range(len(PALETTE))]
    cell_x = 0.055 * s
    cell_y = 0.055 * s
    cell_z = 0.055 * s
    base_y = top - 0.105 * s

    if not large:
        profiles = [
            (0, 5.4, 4.7, 0.00),
            (1, 5.1, 4.5, 0.00),
            (2, 4.7, 4.1, 0.00),
            (3, 4.2, 3.7, 0.05),
            (4, 3.6, 3.2, 0.10),
            (5, 3.0, 2.7, 0.15),
            (6, 2.4, 2.2, 0.20),
            (7, 1.8, 1.7, 0.28),
            (8, 1.25, 1.20, 0.36),
            (9, 0.72, 0.72, 0.45),
        ]
    else:
        profiles = [
            (0, 6.2, 5.4, 0.00),
            (1, 5.9, 5.1, 0.00),
            (2, 5.5, 4.8, 0.00),
            (3, 5.0, 4.3, 0.04),
            (4, 4.4, 3.8, 0.09),
            (5, 3.8, 3.3, 0.14),
            (6, 3.2, 2.8, 0.19),
            (7, 2.6, 2.3, 0.25),
            (8, 2.0, 1.9, 0.31),
            (9, 1.45, 1.40, 0.38),
            (10, 0.90, 0.90, 0.46),
            (11, 0.55, 0.55, 0.54),
        ]

    droplet = set()
    layer_info = {}
    for iy, rx, rz, shift in profiles:
        layer_info[iy] = (rx, rz, shift)
        lim_x = int(rx) + 2
        lim_z = int(rz) + 2
        for ix in range(-lim_x, lim_x + 1):
            for iz in range(-lim_z, lim_z + 1):
                px = (ix - shift) / rx
                pz = iz / rz
                if px * px + pz * pz <= 1.0:
                    droplet.add((ix, iy, iz))

    detail_faces = (
        ((1, 0, 0), ((1, 0, 0), (1, 0, 1), (1, 1, 1), (1, 1, 0))),
        ((-1, 0, 0), ((0, 0, 1), (0, 0, 0), (0, 1, 0), (0, 1, 1))),
        ((0, 1, 0), ((0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1))),
        ((0, -1, 0), ((0, 0, 1), (1, 0, 1), (1, 0, 0), (0, 0, 0))),
        ((0, 0, 1), ((1, 0, 1), (0, 0, 1), (0, 1, 1), (1, 1, 1))),
        ((0, 0, -1), ((0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0))),
    )

    max_layer = max(layer_info)

    def droplet_material(ix: int, iy: int, iz: int, normal) -> int:
        # The first two layers are literally the top material of the body.
        # This removes the visible color seam where the droplet emerges.
        if iy <= 1:
            return 5  # SlimeShellLight: same material used by upward top faces.
        t = iy / max_layer
        rx, _rz, shift = layer_info[iy]
        rel_x = (ix - shift) / max(rx, 1.0)
        if normal == (0, 1, 0):
            return 10 if t > 0.78 else 6
        if normal == (1, 0, 0):
            return 2
        if normal == (-1, 0, 0):
            return 0
        if normal == (0, 0, 1):
            return 2 if t < 0.65 else 7
        if normal == (0, 0, -1):
            if rel_x < -0.35:
                return 0
            if rel_x > 0.42:
                return 5
            if t > 0.72:
                return 10
            if t > 0.42:
                return 6
            return 1
        return 3

    for ix, iy, iz in sorted(droplet, key=lambda v: (v[1], v[2], v[0])):
        for normal, corners in detail_faces:
            if (ix + normal[0], iy + normal[1], iz + normal[2]) in droplet:
                continue
            # Do not draw the buried underside: the extension is meant to grow
            # through the slime top, not sit on it as a separate prop.
            if iy == 0 and normal == (0, -1, 0):
                continue
            pts = [
                ((ix + px) * cell_x,
                 base_y + (iy + py) * cell_y,
                 (iz + pz) * cell_z)
                for px, py, pz in corners
            ]
            quad(drop_buckets[droplet_material(ix, iy, iz, normal)], pts, normal)

    details_prims = [
        p for p in (prim(b, i) for i, b in enumerate(drop_buckets)) if p
    ]
    meshes.append({"name": "hydro_water_droplet", "primitives": details_prims})
    details_mesh = len(meshes) - 1
    detail_height = (max_layer + 1) * cell_y - 0.105 * s
    detail_width = max((rx * 2.0 * cell_x for _iy, rx, _rz, _shift in profiles), default=0.0)

'''
hydro = replace_detail_block(hydro_path, hydro_block)
hydro = hydro.replace('"generator": "Asteria Hydro droplet rebuild v1"', '"generator": "Asteria Hydro integrated droplet rebuild v2"')
hydro = hydro.replace('"pixel_art_geometry": "Hydro droplet uses only axis-aligned voxel boxes; taper is expressed through stepped layer size changes"', '"pixel_art_geometry": "Hydro top extension is an exposed-face voxel volume; all faces are cardinal and the teardrop taper is encoded by shrinking voxel layers"')
hydro = hydro.replace('"reference_design": "clean turquoise water blob with no horns and one integrated teardrop on top"', '"reference_design": "the slime body itself continues upward from a broad top into a pointed water-drop silhouette; base color is identical to the slime top"')
hydro_path.write_text(hydro)


pyro_path = ROOT / 'assets/models/creatures/slime_pyro/generate_slime_pyro.py'
pyro = pyro_path.read_text()
# Preserve every flame coordinate/shape. Only introduce a dedicated root bucket
# using exactly the same material as the top surface of the blob.
pyro = pyro.replace('    fire_dark = bucket()\n    fire_orange = bucket()', '    fire_root = bucket()\n    fire_dark = bucket()\n    fire_orange = bucket()', 1)
pyro = pyro.replace(', fire_dark),', ', fire_root),')
pyro = pyro.replace('    details_prims = [\n        prim(fire_dark, 9),', '    details_prims = [\n        prim(fire_root, 5),\n        prim(fire_dark, 9),', 1)
pyro = pyro.replace('"generator": "Asteria Pyro flame rebuild v1"', '"generator": "Asteria Pyro flame rebuild v2"')
pyro = pyro.replace('"reference_design": "orange fire blob with no horns and an asymmetric integrated flame tuft on top"', '"reference_design": "irregular flames rise from the slime top; every flame root begins in the exact top-surface color before transitioning into fire colors"')
pyro_path.write_text(pyro)


for path, construction, reference in [
    (
        ROOT / 'assets/models/creatures/slime_hydro/slime_hydro.collider.json',
        'rounded Hydro blob + broad exposed-face voxel continuation of the top that tapers into a pointed droplet; root uses the exact top body material + square texture face',
        'Hydro top is pulled upward as part of the slime silhouette, not a separate droplet object; no color seam at the root',
    ),
    (
        ROOT / 'assets/models/creatures/slime_hydro_large/slime_hydro_large.collider.json',
        'rounded Hydro blob + broad exposed-face voxel continuation of the top that tapers into a pointed droplet; root uses the exact top body material + square texture face',
        'Hydro top is pulled upward as part of the slime silhouette, not a separate droplet object; no color seam at the root',
    ),
    (
        ROOT / 'assets/models/creatures/slime_pyro/slime_pyro.collider.json',
        'rounded Pyro blob + irregular asymmetric voxel flames; flame roots use the exact top body material before transitioning to fire colors + square texture face',
        'Pyro flame geometry retained; color transition now starts from the slime top color',
    ),
    (
        ROOT / 'assets/models/creatures/slime_pyro_large/slime_pyro_large.collider.json',
        'rounded Pyro blob + irregular asymmetric voxel flames; flame roots use the exact top body material before transitioning to fire colors + square texture face',
        'Pyro flame geometry retained; color transition now starts from the slime top color',
    ),
]:
    data = json.loads(path.read_text())
    data.setdefault('authoring', {})['construction'] = construction
    data['authoring']['reference'] = reference
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')

(ROOT / 'VERSION').write_text('0.68.60\n')
handoff = ROOT / 'HANDOFF.md'
if handoff.exists():
    text = handoff.read_text()
    note = '- 0.68.60: Hydro top rebuilt as a shaded exposed-face voxel continuation of the body, broad at the root and tapering to a droplet point; its root uses the exact slime-top material. Pyro flame geometry kept, but all flame roots now begin in the exact top-surface material before transitioning to fire colors.'
    if note not in text:
        handoff.write_text(text.rstrip() + '\n\n' + note + '\n')
