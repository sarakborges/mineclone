# Asteria Agent Rules

These rules are mandatory for every implementation, refactor, review, and planning task in this repository.

## Mandatory reading

Before changing code, read and follow:

- `ARCHITECTURE.md`
- `ENGINEERING_PRACTICES.md`

These documents are jointly normative.

## Forward-only feature policy

> **NEW FEATURES DO NOT CARE ABOUT BACKWARD COMPATIBILITY.**

Asteria is developed forward-only. When implementing a new feature, prefer the cleanest current architecture and data model even if that breaks compatibility with previous implementations, serialized formats, APIs, authored data, saves, configs, or internal behavior.

Do not add compatibility shims, legacy branches, migrations, aliases, duplicated paths, deprecated fallbacks, or preservation logic for old behavior unless the user explicitly requests compatibility for that specific change.

When old code or data conflicts with the intended new design, replace or remove the old contract instead of preserving it.
