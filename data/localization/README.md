# Localization

Localization is grouped first by language, then by data domain.

- `data/localization/{language}/ui.json` contains UI strings.
- `data/localization/{language}/{domain}.json` contains player-facing text extracted from `data/{domain}/**` definitions.
- Data catalogs are keyed by definition ID, then by JSON Pointer (for example `/name` or `/hint`).
- The original data definition keeps `null` at localized fields. The content loader rehydrates those fields before deserialization.
- Every language must provide the same domain files, definition IDs, field pointers, and placeholders as English.

Do not embed `{ english, portuguese_brazil, spanish }` objects inside gameplay data definitions.
