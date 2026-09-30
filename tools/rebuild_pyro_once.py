#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "assets/models/creatures/slime_hydro/generate_slime_hydro.py"
DST = ROOT / "assets/models/creatures/slime_pyro/generate_slime_pyro.py"

text = SRC.read_text()

new_header = '''#!/usr/bin/env python3
"""Generate pixel-art Pyro Slime models from the rounded slime_blob body language.

The Pyro body is a clean orange-gold blob with no horns or armor. Its modeled
identity comes from several flame tongues growing directly from the top surface.
Every flame uses axis-aligned voxel stair-steps: apparent curves and diagonals
are built from stepped boxes, never rotated or smooth geometry. The face remains
a square texture-driven SlimeFace decal.
"""
'''
text = re.sub(r'^#!/usr/bin/env python3\n""".*?"""\n', new_header, text, count=1, flags=re.S)

p0 = text.index('PALETTE = (')
p1 = text.index('\n\n\ndef lin', p0)
palette = '''PALETTE = (
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
)'''
text = text[:p0] + palette + text[p1:]
text = text.replace('"hydro_blob_body"', '"pyro_blob_body"')

d0 = text.index('    s = body_width / 1.20')
d1 = text.index('    materials = [', d0)
details = '''    s = body_width / 1.20
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

'''
text = text[:d0] + details + text[d1:]

text = text.replace('details_node = node("HydroDroplet", mesh=details_mesh)', 'details_node = node("PyroFlames", mesh=details_mesh)')
text = text.replace('collider_size = [1.248, 1.26, 1.2376]', 'collider_size = [1.222, 1.2432, 1.224]')
text = text.replace('collider_y = 0.63', 'collider_y = 0.6216')
text = text.replace('"Asteria Hydro droplet rebuild v1"', '"Asteria Pyro flame rebuild v1"')
text = text.replace('"GeoSlime"', '"PyroSlime"')
text = text.replace('"face_source": "textures/creatures/slime_hydro/face.png"', '"face_source": "textures/creatures/slime_pyro/face.png"')
text = text.replace(
    '"pixel_art_geometry": "Hydro droplet uses only axis-aligned voxel boxes; taper is expressed through stepped layer size changes"',
    '"pixel_art_geometry": "Pyro flames use only axis-aligned voxel boxes; flame curves and leans are expressed through staircase layer offsets"',
)
text = text.replace(
    '"reference_design": "clean turquoise water blob with no horns and one integrated teardrop on top"',
    '"reference_design": "clean orange-gold blob with no horns; several asymmetric flame tongues grow directly from the top"',
)

m0 = text.index('def main() -> None:')
text = text[:m0] + '''def main() -> None:
    small = generate("slime_pyro", ROOT / "slime_pyro.glb", 1.20, 1.00, 1.14, False)
    large_dir = ROOT.parent / "slime_pyro_large"
    large = generate("slime_pyro_large", large_dir / "slime_pyro_large.glb", 1.88, 1.48, 1.80, True)
    print("small", small)
    print("large", large)


if __name__ == "__main__":
    main()
'''
DST.write_text(text)

for path in [
    ROOT / 'assets/models/creatures/slime_pyro/slime_pyro.collider.json',
    ROOT / 'assets/models/creatures/slime_pyro_large/slime_pyro_large.collider.json',
]:
    d = json.loads(path.read_text())
    d['authoring']['construction'] = 'clean orange-gold slime_blob body + asymmetric stepped flame cluster growing from the top + square texture-driven face'
    d['authoring']['reference'] = 'user Pyro reference: ignore horns; top should read as flames built in strict pixel-art geometry'
    path.write_text(json.dumps(d, indent=2, ensure_ascii=False) + '\n')

(ROOT / 'VERSION').write_text('0.68.58\n')
h = ROOT / 'HANDOFF.md'
if h.exists():
    t = h.read_text()
    note = '- 0.68.58: Pyro Slime rebuilt from the reference as a clean orange-gold blob with no horns. Normal and large now use an integrated asymmetric flame cluster on top, built only from cardinal voxel stair-steps; face remains texture-driven.'
    if note not in t:
        h.write_text(t.rstrip() + '\n\n' + note + '\n')

print(DST)
