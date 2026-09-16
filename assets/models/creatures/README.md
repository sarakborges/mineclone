# Creature models (moddable)

Place `.glb` or `.gltf` model assets under `assets/models/creatures/`, for example `assets/models/creatures/slime/slime.glb`. Definitions live in `data/creatures/**/*.json` and choose the model with the path **relative to `assets/`**, e.g. `"model": "models/creatures/slime/slime.glb"`. No individual model or creature species is hard-coded in Rust.

The original animated Slime GLB, its authoring generator and metadata are committed in `assets/models/creatures/slime/`. The asset workflow checked the binary against the original SHA-256 before committing it. Modders can use a glTF-compatible editor such as Blender to import, edit and export their own GLB, put it here, then point a new JSON at that file. The engine loads the model at runtime; there is no in-game export UI.

Every creature JSON requires a stable internal `id` and a separate, localized display `name`: `"name": { "english": "Meadow Slime" }`. The name uses the existing `LocalizedText` mechanism and requires a nonempty English fallback. Named animation clips and named material slots are selected by JSON. Multiple species may share one GLB while applying different HSI tints to their own material instances.

`previewSpawn: true` is opt-in for a nearby developer preview only; biome spawning, persistence, combat, and a complete mod-management UI are separate features. The AABB is defined by the creature JSON on the unanimated entity root and does not deform with squash/stretch clips. The engine currently implements glTF import, not an in-game model export command.
