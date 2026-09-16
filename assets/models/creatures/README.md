# Creature models (moddable)

Put `.glb` or `.gltf` model assets here, for example:

`assets/models/creatures/slime/slime.glb`

Creature definitions live in `data/creatures/**/*.json` and reference their model by its path relative to `assets/`, e.g. `"model": "models/creatures/slime/slime.glb"`. The model is **not** selected or hard-coded in Rust. Export a `.glb` from Blender (or another glTF-compatible tool), place it in this directory, and point your creature JSON at it. Named animation clips and named material slots are selected by the JSON; different creatures can share one GLB and use distinct tint palettes.

The original `slime.glb` is a separate binary asset supplied as the `asteria_slime_asset.zip` package in the project conversation. Copy that **exact** file to `assets/models/creatures/slime/slime.glb` before launching the preview. It has not been uploaded to this branch: the GitHub text-only file connector cannot upload the GLB binary. Do not replace it with a placeholder or generated lookalike.

`previewSpawn: true` is strictly an opt-in developer preview near the player, not final biome spawning or world persistence. Creature AABBs are declared in the JSON, not inferred from mesh triangles or changed by squash/stretch animations. Current model import is glTF runtime loading; exporting/editing is through a glTF-compatible modeling application, not an in-game export button.
