# Creature models and skins

Creature definitions in `data/creatures/**/*.json` own their texture selection. Model files under `assets/models/creatures/` describe reusable geometry, UVs, material names, nodes and animation clips; they must not embed species-specific PNGs or choose a species texture in the generator.

For each material used by a model, the optional `textures` map associates its **glTF material name** with a PNG path relative to `assets/`:

```json
"textures": {
  "SlimeShell": "textures/creatures/meadow_slime.png",
  "SlimeCore": "textures/creatures/meadow_slime.png",
  "SlimeEyes": "textures/creatures/meadow_slime.png",
  "SlimeHighlights": "textures/creatures/meadow_slime.png",
  "SlimeCheeks": "textures/creatures/meadow_slime.png"
}
```

Paths must be safe, relative PNG paths beneath `textures/creatures/`; the actual files live in `assets/textures/creatures/{name}.png`. Distinct materials may use distinct PNG paths. Unmapped materials keep their model-default appearance for backwards compatibility. Creature-specific `materialTints` are applied independently of the textures; grayscale skins preserve the authoritative HSI tint and material alpha. Runtime loads a texture through Bevy's `AssetServer`, whose global image plugin is configured for nearest-neighbor filtering. The material cache includes source material, tint and texture identity to prevent one species from recoloring another.

**Art rule:** all 3D model geometry and details must be square or cubical, with sharp corners and flat faces; no bevels, rounded geometry, ellipsoids, curves, smoothed normals, or rounded eyes/mouth/cheeks/highlights. Slime shell, inner core and facial details follow this rule. Slime skins are pixelated PNGs exactly 64×64. Species share `models/creatures/slime/slime.glb` and select their own `meadow_slime.png` or `ember_slime.png` in JSON. Their existing `SlimeRoot`, nonanimated AABB collider, named materials and six animation clips must remain intact. Running `generate_slime.py` creates missing starter skins but never overwrites existing artist-authored PNGs.

**Verification:** structural checks of GLB geometry/UVs, named clips, PNG resolution and JSON paths are independent of visual/gameplay QA. Actual glTF import, texture appearance, HSI colors, transparency, colliders and animation transitions still require in-game testing; CI runs Clippy and cargo check, not unit-test execution.
