# Asteria Documentation

This directory contains project documentation that is not required to live at the repository root.

## Start here

Repository-wide authority and active work state stay in the root because agents and contributors must find them immediately:

- [`../AGENTS.md`](../AGENTS.md) — mandatory agent/contributor rules and forward-only feature policy.
- [`../ARCHITECTURE.md`](../ARCHITECTURE.md) — canonical Asteria architecture contracts.
- [`../ENGINEERING_PRACTICES.md`](../ENGINEERING_PRACTICES.md) — canonical engineering rules.
- [`../HANDOFF.md`](../HANDOFF.md) — current implementation handoff only.

When documents disagree, `AGENTS.md`, `ARCHITECTURE.md`, and `ENGINEERING_PRACTICES.md` are normative. Archived plans and handoffs are historical evidence, not current authority.

## Current documentation

### Design

- [`design/worldgen-coherence.md`](design/worldgen-coherence.md) — worldgen coherence package: biome relationships, ocean margins, volume biomes, surface indicators, and floating islands.

### Features

- [`features/artisans-kit-microblocking.md`](features/artisans-kit-microblocking.md) — Artisan's Kit microgeometry, persistence, and QA requirements.

### Guides and content contracts

- [`guides/localization.md`](guides/localization.md) — mandatory localization contract.
- [`guides/creature-textures.md`](guides/creature-textures.md) — creature model/texture ownership and art constraints.

### QA

- [`qa/save-roundtrip.md`](qa/save-roundtrip.md) — Windows save/restore verification protocol.

### Handoffs

- [`handoffs/`](handoffs/) — historical task-specific handoffs and audits.
- [`handoffs/archive/`](handoffs/archive/) — large superseded root handoff archives.
- The only active handoff is [`../HANDOFF.md`](../HANDOFF.md).

## Archive

- [`archive/core-rebuild/`](archive/core-rebuild/) — historical architecture rebuild plan, ownership inventory, and implementation progress.
- [`archive/qa/`](archive/qa/) — superseded/version-specific QA checklists.

Archived documents must not be used as current requirements when they conflict with the normative root documents or current implementation.

## Documentation layout rules

1. Keep only mandatory, automatically discoverable project authority and the current handoff in the repository root.
2. New feature/design documentation goes under the matching `docs/` category, not the root.
3. Completed or superseded plans/checklists move to `docs/archive/`; do not leave historical plans beside current contracts.
4. Historical handoffs belong under `docs/handoffs/` or `docs/handoffs/archive/`; do not create new `HANDOFF_ARCHIVE_*.md` files in the root.
5. Use lowercase kebab-case filenames inside `docs/`.
6. Prefer updating an existing canonical document over creating another document that describes the same contract.
7. If a document becomes normative for all implementation work, promote the rule into `AGENTS.md`, `ARCHITECTURE.md`, or `ENGINEERING_PRACTICES.md` instead of relying on a buried feature document.
