#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GEN = ROOT / "assets/models/creatures/slime_hydro/generate_slime_hydro.py"

text = GEN.read_text()

old_doc = '''"""Generate pixel-art Hydro Slime models from the rounded slime_blob body language.\n\nThe Hydro body is a clean turquoise blob with no horns, armor, or side features.\nIts only modeled elemental feature is one water droplet growing directly from the\ntop of the body. The droplet tapers through axis-aligned voxel stair-steps so the\nsilhouette reads as pixel art without smooth curves or diagonal faces. The face\nremains a square texture-driven SlimeFace decal.\n"""'''
new_doc = '''"""Generate pixel-art Hydro Slime models from the rounded slime_blob body language.\n\nHydro has no separate modeled accessory on top. The slime body itself keeps the\nrounded blob volume through the lower/middle mass and then continues upward,\nprogressively narrowing into a short water-drop point. The entire silhouette is\none exposed-face voxel body mesh; the face remains texture-driven.\n"""'''
if old_doc not in text:
    raise SystemExit("Hydro docstring anchor changed")
text = text.replace(old_doc, new_doc, 1)

old_profile = '''def profile(t: float) -> float:\n    keys = (\n        (0.0, 0.66), (0.06, 0.79), (0.15, 0.91), (0.28, 0.995),\n        (0.44, 1.0), (0.58, 0.965), (0.70, 0.89), (0.80, 0.76),\n        (0.88, 0.60), (0.94, 0.36), (0.975, 0.14), (1.0, 0.02),\n    )'''
new_profile = '''def profile(t: float) -> float:\n    # One continuous Hydro body: rounded blob below, then the same body\n    # narrows through the upper silhouette into a short droplet point.\n    # There is deliberately no secondary bulb, brim, mound, or accessory.\n    keys = (\n        (0.0, 0.66), (0.06, 0.79), (0.15, 0.91), (0.28, 0.995),\n        (0.44, 1.0), (0.58, 0.97), (0.70, 0.91), (0.79, 0.82),\n        (0.86, 0.72), (0.91, 0.60), (0.95, 0.46), (0.975, 0.31),\n        (0.99, 0.17), (1.0, 0.04),\n    )'''
if old_profile not in text:
    raise SystemExit("Hydro profile anchor changed")
text = text.replace(old_profile, new_profile, 1)

start = text.index('    s = body_width / 1.20\n    top = body_height * 0.5\n')
end = text.index('    materials = [', start)
text = text[:start] + text[end:]

old_nodes = '''    body_node = node("Shell", mesh=body_mesh)\n    face_node = node("Face", mesh=face_mesh)\n    details_node = node("HydroDroplet", mesh=details_mesh)\n    nodes[body_pivot]["children"] = [body_node, face_node, details_node]'''
new_nodes = '''    body_node = node("Shell", mesh=body_mesh)\n    face_node = node("Face", mesh=face_mesh)\n    nodes[body_pivot]["children"] = [body_node, face_node]'''
if old_nodes not in text:
    raise SystemExit("Hydro detail-node anchor changed")
text = text.replace(old_nodes, new_nodes, 1)

old_visual = '''    visual_height = body_height + detail_height\n    visual_width = max(body_width, detail_width)'''
new_visual = '''    visual_height = body_height\n    visual_width = body_width'''
if old_visual not in text:
    raise SystemExit("Hydro visual bounds anchor changed")
text = text.replace(old_visual, new_visual, 1)

text = text.replace(
    '"generator": "Asteria Hydro integrated droplet rebuild v3"',
    '"generator": "Asteria Hydro body-silhouette droplet rebuild v4"',
    1,
)
text = text.replace(
    '"pixel_art_geometry": "Hydro top extension is an exposed-face voxel volume; all faces are cardinal and the teardrop taper is encoded by shrinking voxel layers"',
    '"pixel_art_geometry": "Hydro is one exposed-face voxel body mesh; the body profile itself narrows into the top droplet point and all faces remain cardinal"',
    1,
)
text = text.replace(
    '"reference_design": "clean turquoise blob whose own top continues upward into one broad-based teardrop point"',
    '"reference_design": "clean turquoise slime whose entire upper body silhouette continues naturally into one short droplet point; no separate droplet mesh or hat-like mound"',
    1,
)

GEN.write_text(text)

for rel in [
    "assets/models/creatures/slime_hydro/slime_hydro.collider.json",
    "assets/models/creatures/slime_hydro_large/slime_hydro_large.collider.json",
]:
    path = ROOT / rel
    data = json.loads(path.read_text())
    authoring = data.setdefault("authoring", {})
    authoring["construction"] = (
        "single rounded Hydro voxel body whose own upper silhouette narrows into a short droplet point; "
        "no separate top detail mesh + square texture face"
    )
    authoring["reference"] = (
        "the top of the slime itself is the water-drop shape: rounded body below, continuous taper above, "
        "short point at the apex; no droplet sitting on the slime"
    )
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")

(ROOT / "VERSION").write_text("0.68.64\n")

handoff = ROOT / "HANDOFF.md"
if handoff.exists():
    note = (
        "- 0.68.64: Hydro corrected structurally: removed the separate HydroDroplet mesh/node entirely. "
        "The main hydro_blob_body profile itself now carries the water-drop silhouette and tapers continuously "
        "into a short apex point; Pyro unchanged."
    )
    existing = handoff.read_text()
    if note not in existing:
        handoff.write_text(existing.rstrip() + "\n\n" + note + "\n")
