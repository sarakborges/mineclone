#!/usr/bin/env python3
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / 'assets/models/creatures/slime_hydro/generate_slime_hydro.py'
text = path.read_text()

text = text.replace(
    '    # Keep the droplet broad, but compress it vertically so it reads as a drop rather than a spike.\n    cell_y = 0.040 * s\n    cell_z = 0.055 * s\n    base_y = top - 0.105 * s\n',
    '    # Keep the drop compact and rounded. The lower layers hold their width\n'
    '    # before the upper third tapers quickly into a short point, avoiding a cone/spike silhouette.\n'
    '    cell_y = 0.038 * s\n'
    '    cell_z = 0.055 * s\n'
    '    base_y = top - 0.115 * s\n'
)

pattern = re.compile(
    r'    if not large:\n'
    r'        profiles = \[.*?\n'
    r'        \]\n'
    r'    else:\n'
    r'        profiles = \[.*?\n'
    r'        \]\n',
    re.S,
)
replacement = '''    if not large:\n        # Rounded water-drop silhouette: broad/bulbous lower body, then a late taper.\n        profiles = [\n            (0, 5.2, 4.6, 0.00),\n            (1, 5.4, 4.8, 0.00),\n            (2, 5.3, 4.7, 0.02),\n            (3, 5.0, 4.4, 0.04),\n            (4, 4.5, 4.0, 0.07),\n            (5, 3.8, 3.4, 0.10),\n            (6, 2.7, 2.5, 0.15),\n            (7, 1.5, 1.4, 0.21),\n            (8, 0.60, 0.60, 0.27),\n        ]\n    else:\n        profiles = [\n            (0, 6.0, 5.3, 0.00),\n            (1, 6.3, 5.6, 0.00),\n            (2, 6.2, 5.5, 0.02),\n            (3, 5.9, 5.2, 0.04),\n            (4, 5.5, 4.9, 0.07),\n            (5, 4.9, 4.4, 0.10),\n            (6, 4.1, 3.7, 0.14),\n            (7, 3.1, 2.9, 0.19),\n            (8, 2.0, 1.9, 0.25),\n            (9, 0.72, 0.72, 0.31),\n        ]\n'''
text2, n = pattern.subn(replacement, text, count=1)
if n != 1:
    raise SystemExit(f'expected one Hydro profile block, replaced {n}')
text = text2

text = text.replace(
    '"generator": "Asteria Hydro integrated droplet rebuild v2"',
    '"generator": "Asteria Hydro rounded droplet rebuild v3"'
)
text = text.replace(
    '"reference_design": "the slime body itself continues upward from a broad top into a pointed water-drop silhouette; base color is identical to the slime top"',
    '"reference_design": "the slime top swells into a compact rounded water droplet, holding a bulbous lower silhouette before a short upper taper; base color is identical to the slime top"'
)
path.write_text(text)

for collider in [
    ROOT / 'assets/models/creatures/slime_hydro/slime_hydro.collider.json',
    ROOT / 'assets/models/creatures/slime_hydro_large/slime_hydro_large.collider.json',
]:
    import json
    data = json.loads(collider.read_text())
    data.setdefault('authoring', {})['construction'] = (
        'rounded Hydro blob + compact rounded voxel droplet grown from the top; '
        'lower drop remains bulbous before a short late taper; root uses exact top body material + square texture face'
    )
    data['authoring']['reference'] = (
        'Hydro top should read as a water droplet, not a cone, spike, or hat: rounded body with a short pointed tip'
    )
    collider.write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')

(ROOT / 'VERSION').write_text('0.68.62\n')
handoff = ROOT / 'HANDOFF.md'
if handoff.exists():
    note = '- 0.68.62: Hydro droplet reshaped from a cone/spike silhouette into a compact rounded drop: lower layers hold a bulbous width, taper begins later, and the point is short. Pyro unchanged.'
    h = handoff.read_text()
    if note not in h:
        handoff.write_text(h.rstrip() + '\n\n' + note + '\n')
