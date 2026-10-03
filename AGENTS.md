# Asteria Agent Rules

These rules are mandatory for every implementation, refactor, review, and planning task in this repository.

## Mandatory reading

Before changing code, read and follow:

- `ARCHITECTURE.md`
- `ENGINEERING_PRACTICES.md`

These documents are jointly normative.

Use `docs/README.md` as the documentation map. `HANDOFF.md` is the only active handoff; files under `docs/archive/` and `docs/handoffs/` are historical context unless the current task explicitly points to them.

## Repository navigation discipline

> **DO NOT WALK THE ENTIRE REPOSITORY TREE OR START WITH BROAD, UNBOUNDED SEARCHES. THEY DO NOT WORK RELIABLY IN THIS PROJECT.**

Repository discovery must be scoped and incremental. Full-tree traversal, recursive directory dumps, repository-wide `find`, and broad `grep`/search queries create too much noise, frequently time out or truncate, and are not a valid default discovery strategy.

When locating code or assets:

- Start from the most likely subsystem, directory, filename, symbol, asset id, error string, or documentation pointer.
- Prefer direct file reads and narrowly scoped searches over recursive exploration.
- Use `docs/README.md`, `ARCHITECTURE.md`, `ENGINEERING_PRACTICES.md`, and known module paths to choose the first search scope.
- Search for exact or distinctive identifiers first: type names, function names, config keys, asset ids, log messages, or filenames.
- Expand the scope one level at a time only when the targeted search fails.
- If the location is unknown, use a small number of specific search terms to discover candidate files; inspect those candidates instead of enumerating the repository.
- Never use an exhaustive tree walk merely to "understand the project" before beginning work. Build context from the relevant paths outward.

Treat a broad repository scan as a failure mode to avoid, not as a fallback strategy.

## Forward-only feature policy

> **NEW FEATURES DO NOT CARE ABOUT BACKWARD COMPATIBILITY.**

Asteria is developed forward-only. When implementing a new feature, prefer the cleanest current architecture and data model even if that breaks compatibility with previous implementations, serialized formats, APIs, authored data, saves, configs, or internal behavior.

Do not add compatibility shims, legacy branches, migrations, aliases, duplicated paths, deprecated fallbacks, or preservation logic for old behavior unless the user explicitly requests compatibility for that specific change.

When old code or data conflicts with the intended new design, replace or remove the old contract instead of preserving it.

## River generation architecture

Rivers are authored connected structures, not a separate hydrology graph or world-generation subsystem.

- A river has no independent `source`. Its endpoints are existing bodies of water such as oceans and lakes.
- Oceans/lakes may author spaced, deterministic connector placements on their margins. Connector spacing/chance/jitter belong to the normal structure-placement rules.
- River mouths and river segments use the generic Structure/connector system. River segments excavate terrain and place water through authored structure payloads.
- River chains continue through authored river-structure variations until they reach another body of water or ordinary generic structure constraints prevent continuation.
- River variation belongs in Structure groups; do not hardcode river shapes, meanders, downstream graphs, sinks, basins, source selection, or bespoke river rasterization in Rust.
- Do not recreate `rivers.rs`, `DimensionRiverNetwork`, drainage graphs, basin/source selection, or any equivalent parallel hydrology owner.
- Generic connector improvements required by rivers must remain generic and usable by other connected structures.

Any older documentation describing rivers as a dedicated hydrology network is superseded by this section.

## Documentation placement

- Keep repository-wide normative rules in `AGENTS.md`, `ARCHITECTURE.md`, or `ENGINEERING_PRACTICES.md`.
- Keep only the current implementation handoff in root `HANDOFF.md`.
- Put feature/design/guides/QA documentation under the matching category in `docs/`.
- Put superseded plans and checklists in `docs/archive/`.
- Put historical handoffs in `docs/handoffs/`; never create new `HANDOFF_ARCHIVE_*.md` files in the repository root.
