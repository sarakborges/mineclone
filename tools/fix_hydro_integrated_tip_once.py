#!/usr/bin/env python3
from pathlib import Path

p = Path('assets/models/creatures/slime_hydro/generate_slime_hydro.py')
s = p.read_text()

s = s.replace(
    'NX, NY, NZ = 24, 20, 22\n',
    'NX, NY, NZ = 24, 20, 22\nTIP_EXTRA_LAYERS = 3\nTIP_PROFILES = (\n    (NY - 2, 5.8, 5.0, 0.00),\n    (NY - 1, 4.6, 4.0, 0.00),\n    (NY,     3.6, 3.2, 0.10),\n    (NY + 1, 2.0, 1.8, 0.24),\n    (NY + 2, 0.65, 0.65, 0.42),\n)\n'
)

old_occ = '''    def occupied(ix: int, iy: int, iz: int) -> bool:\n        x = (ix + 0.5) * dx - body_width * 0.5\n        y = (iy + 0.5) * dy - body_height * 0.5\n        z = (iz + 0.5) * dz - body_depth * 0.5\n        t = (y + half_h) / body_height\n        radius = profile(t)\n        x -= 0.032 * (body_width / 1.20) * max(0.0, (t - 0.62) / 0.38) ** 1.7\n        rx = body_width * 0.5 * radius\n        rz = body_depth * 0.5 * radius\n        return rx > 0 and rz > 0 and abs(x / rx) ** 2.35 + abs(z / rz) ** 2.35 <= 1\n\n    vox = {(x, y, z) for y in range(NY) for z in range(NZ) for x in range(NX) if occupied(x, y, z)}\n'''

new_occ = '''    def occupied(ix: int, iy: int, iz: int) -> bool:\n        # Main rounded slime body.\n        body = False\n        if iy < NY:\n            x = (ix + 0.5) * dx - body_width * 0.5\n            y = (iy + 0.5) * dy - body_height * 0.5\n            z = (iz + 0.5) * dz - body_depth * 0.5\n            t = (y + half_h) / body_height\n            radius = profile(t)\n            x -= 0.032 * (body_width / 1.20) * max(0.0, (t - 0.62) / 0.38) ** 1.7\n            rx = body_width * 0.5 * radius\n            rz = body_depth * 0.5 * radius\n            body = rx > 0 and rz > 0 and abs(x / rx) ** 2.35 + abs(z / rz) ** 2.35 <= 1\n\n        # The pointed Hydro top is part of this SAME occupancy field / mesh.\n        # Its root overlaps the body's top two voxel layers, so there is no\n        # accessory seam, cap, hat brim, or second mesh. Only the central top\n        # continues a few layers upward and tapers to one voxel-scale point.\n        tip = False\n        cx = (NX - 1) * 0.5\n        cz = (NZ - 1) * 0.5\n        for ty, trx, trz, shift in TIP_PROFILES:\n            if iy == ty:\n                tx = (ix - (cx + shift)) / trx\n                tz = (iz - cz) / trz\n                tip = tx * tx + tz * tz <= 1.0\n                break\n\n        return body or tip\n\n    vox = {(x, y, z) for y in range(NY + TIP_EXTRA_LAYERS) for z in range(NZ) for x in range(NX) if occupied(x, y, z)}\n'''

if old_occ not in s:
    raise SystemExit('occupied block not found')
s = s.replace(old_occ, new_occ)

s = s.replace('        gy = iy / (NY - 1)\n', '        gy = min(1.0, iy / (NY - 1))\n')
s = s.replace('    visual_height = body_height\n', '    visual_height = body_height + TIP_EXTRA_LAYERS * dy\n')
s = s.replace('            "voxel_resolution": [NX, NY, NZ],\n', '            "voxel_resolution": [NX, NY + TIP_EXTRA_LAYERS, NZ],\n')
s = s.replace(
    '            "pixel_art_geometry": "Hydro is one exposed-face voxel body mesh; the body profile itself narrows into the top droplet point and all faces remain cardinal",\n',
    '            "pixel_art_geometry": "Hydro is one exposed-face voxel body mesh; a central top continuation overlaps the upper body and extends three voxel layers above it into a short droplet point; all faces remain cardinal",\n'
)
s = s.replace(
    '            "reference_design": "clean turquoise slime whose entire upper body silhouette continues naturally into one short droplet point; no separate droplet mesh or hat-like mound",\n',
    '            "reference_design": "clean turquoise slime whose own central top rises naturally into a short droplet point; no separate droplet mesh, no flat cap, and no hat-like mound",\n'
)
s = s.replace('Asteria Hydro body-silhouette droplet rebuild v4', 'Asteria Hydro integrated top-point rebuild v5')

p.write_text(s)
print('patched', p)
