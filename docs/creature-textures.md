# Creature models and skins

Creature definitions in `data/creatures/**/*.json` own their texture selection. Model files under `assets/models/creatures/` describe reusable geometry, UVs, material names, nodes and animation clips; they must not embed species-specific PNGs or choose a species texture in the generator.

For each material used by a model, the optional `textures` map associates its **glTF material name** with a PNG path relative to `assets/`:

```json
"textures": {
  "SlimeShell": "textures/creatures/meadow_slime.png",
  "SlimeCore": "textures/creatures/meadow_slime.png"
}
```

Paths must be safe, relative PNG paths beneath `textures/creatures/`; the actual files live in `assets/textures/creatures/{name}.png`. Distinct materials may use distinct PNG paths. Unmapped materials keep their model-default appearance for backwards compatibility. Creature-specific `materialTints` are applied independently of the textures; grayscale skins preserve the authoritative HSI tint and material alpha. Runtime loads a texture through Bevy's `AssetServer`, whose global image plugin is configured for nearest-neighbor filtering. The material cache includes source material, tint and texture identity to prevent one species from recoloring another.

**Art rule:** all 3D model geometry and details must be square or cubical, with sharp corners and flat faces; no bevels, rounded geometry, ellipsoids, curves, smoothed normals, or rounded eyes/mouth/cheeks/highlights. Slime skins are pixelated PNGs exactly 64×64. The shared `models/creatures/slime/slime.glb` contains **only two meshes**: the square translucent shell (`0.96×0.90×0.96`) and the enlarged cubic nucleus (`0.58×0.62×0.58`, formerly `0.36×0.40×0.36`). Eyes, eye highlights, cheeks and mouth have **no separate meshes**; they are pixel art on the shell's forward-facing -Z UV tile `(2,0)` in each external PNG. All other shell faces use tile `(0,0)`; the nucleus uses `(1,0)`. Species select their own `meadow_slime.png` or `ember_slime.png` in JSON. Keep the existing `SlimeRoot`, nonanimated AABB collider, `SlimeShell` and `SlimeCore` materials and six named animation clips intact. The generator only creates missing starter skins; the one-time v0.20.3 migration checked the previous default PNG blob hashes before updating them, so artist-edited skins are never overwritten by normal regeneration.

**Verification:** the v0.20.3 migration validated the generated GLB header, two meshes/materials, enlarged nucleus bounds, six clips, absence of facial nodes and embedded image data, the two 64×64 PNGs and their face pixels. Structural verification is independent of visual/gameplay QA. Actual glTF import, texture orientation and appearance, HSI colors, transparency, colliders and animation transitions still require in-game testing; CI runs Clippy and cargo check, not unit-test execution.
