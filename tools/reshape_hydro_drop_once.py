#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GEN = ROOT / "assets/models/creatures/slime_hydro/generate_slime_hydro.py"
text = GEN.read_text()

old = '''    drop_buckets = [bucket() for _ in range(len(PALETTE))]\n    cell_x = 0.055 * s\n    # Keep the drop compact and rounded. The lower layers hold their width\n    # before the upper third tapers quickly into a short point, avoiding a cone/spike silhouette.\n    cell_y = 0.038 * s\n    cell_z = 0.055 * s\n    base_y = top - 0.115 * s\n\n    if not large:\n        # Rounded water-drop silhouette: broad/bulbous lower body, then a late taper.\n        profiles = [\n            (0, 5.2, 4.6, 0.00),\n            (1, 5.4, 4.8, 0.00),\n            (2, 5.3, 4.7, 0.02),\n            (3, 5.0, 4.4, 0.04),\n            (4, 4.5, 4.0, 0.07),\n            (5, 3.8, 3.4, 0.10),\n            (6, 2.7, 2.5, 0.15),\n            (7, 1.5, 1.4, 0.21),\n            (8, 0.60, 0.60, 0.27),\n        ]\n    else:\n        profiles = [\n            (0, 6.0, 5.3, 0.00),\n            (1, 6.3, 5.6, 0.00),\n            (2, 6.2, 5.5, 0.02),\n            (3, 5.9, 5.2, 0.04),\n            (4, 5.5, 4.9, 0.07),\n            (5, 4.9, 4.4, 0.10),\n            (6, 4.1, 3.7, 0.14),\n            (7, 3.1, 2.9, 0.19),\n            (8, 2.0, 1.9, 0.25),\n            (9, 0.72, 0.72, 0.31),\n        ]\n'''

new = '''    drop_buckets = [bucket() for _ in range(len(PALETTE))]\n    # The previous version still read as a hat because its visible root was a\n    # broad stepped mound. This version buries a narrow root inside the body,\n    # lets the exposed middle swell into a bulb, then tapers only the short\n    # upper section into a point. That creates a true teardrop silhouette.\n    cell_x = 0.050 * s\n    cell_y = 0.038 * s\n    cell_z = 0.050 * s\n    base_y = top - 0.150 * s\n\n    if not large:\n        profiles = [\n            (0, 2.10, 1.90, 0.00),\n            (1, 2.35, 2.10, 0.00),\n            (2, 2.60, 2.35, 0.01),\n            (3, 2.82, 2.55, 0.02),\n            (4, 3.00, 2.72, 0.04),\n            (5, 3.08, 2.80, 0.06),\n            (6, 2.95, 2.68, 0.08),\n            (7, 2.62, 2.40, 0.11),\n            (8, 2.18, 2.02, 0.14),\n            (9, 1.64, 1.54, 0.18),\n            (10, 1.05, 1.00, 0.22),\n            (11, 0.52, 0.52, 0.26),\n        ]\n    else:\n        profiles = [\n            (0, 2.20, 2.00, 0.00),\n            (1, 2.45, 2.20, 0.00),\n            (2, 2.70, 2.45, 0.01),\n            (3, 2.92, 2.65, 0.02),\n            (4, 3.10, 2.82, 0.04),\n            (5, 3.20, 2.90, 0.06),\n            (6, 3.12, 2.84, 0.08),\n            (7, 2.90, 2.65, 0.10),\n            (8, 2.58, 2.38, 0.13),\n            (9, 2.18, 2.02, 0.16),\n            (10, 1.68, 1.58, 0.20),\n            (11, 1.12, 1.06, 0.24),\n            (12, 0.56, 0.56, 0.28),\n        ]\n'''

if old not in text:
    raise SystemExit("Hydro profile block did not match current develop")
text = text.replace(old, new, 1)
text = text.replace(
    '        if iy <= 1:\n            return 5  # exact upward/top material of the Hydro body',
    '        if iy <= 5:\n            return 5  # buried root + first exposed bulb layers match the Hydro top exactly',
    1,
)
text = text.replace(
    '    detail_height = (max_layer + 1) * cell_y - 0.105 * s',
    '    detail_height = (max_layer + 1) * cell_y - 0.150 * s',
    1,
)
text = text.replace(
    '"reference_design": "compact rounded water-drop continuation: broad lower bulb held for several layers, then a late short taper into the point; root color matches slime top"',
    '"reference_design": "true teardrop continuation: narrow buried root, exposed rounded bulb, then a short upper taper into a point; no hat brim or stepped mound"',
)
GEN.write_text(text)

for path in [
    ROOT / "assets/models/creatures/slime_hydro/slime_hydro.collider.json",
    ROOT / "assets/models/creatures/slime_hydro_large/slime_hydro_large.collider.json",
]:
    data = json.loads(path.read_text())
    authoring = data.setdefault("authoring", {})
    authoring["construction"] = "rounded Hydro blob + narrow buried voxel root that swells into a rounded exposed teardrop bulb and ends in a short point; no cap/brim silhouette"
    authoring["reference"] = "Hydro top should read as a small water droplet pulled out of the slime surface, not a hat, cone, horn, or mound"
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")

(ROOT / "VERSION").write_text("0.68.63\n")
handoff = ROOT / "HANDOFF.md"
if handoff.exists():
    note = "- 0.68.63: Hydro top reshaped again from the in-game screenshot: removed the broad cap/mound silhouette. The droplet now has a narrow root buried into the slime, a small exposed bulb, and only a short pointed upper taper. Pyro unchanged."
    current = handoff.read_text().rstrip()
    if note not in current:
        handoff.write_text(current + "\n\n" + note + "\n")
