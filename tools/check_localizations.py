#!/usr/bin/env python3
"""Validate player-facing JSON text without loading the Bevy application.

Run from any directory: python3 tools/check_localizations.py
"""

from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data"
LANGUAGES = ("english", "portuguese_brazil", "spanish")
VISIBLE_FIELDS = frozenset({"name", "displayName", "description", "title", "label", "tooltip"})
PLACEHOLDERS = re.compile(r"\{([^{}]+)\}")
errors: list[str] = []
checked_fields = 0


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        errors.append(f"{path.relative_to(ROOT)}: cannot read JSON: {error}")
        return None


def validate_text(value: Any, context: str) -> None:
    global checked_fields
    checked_fields += 1
    if not isinstance(value, dict):
        errors.append(f"{context}: visible text must be a localized object")
        return
    if set(value) != set(LANGUAGES):
        errors.append(
            f"{context}: language keys must be {LANGUAGES}; got {sorted(value)}"
        )
    english = value.get("english")
    if not isinstance(english, str) or not english.strip():
        errors.append(f"{context}: english text is missing or empty")
        return
    for language in LANGUAGES:
        translated = value.get(language)
        if not isinstance(translated, str) or not translated.strip():
            errors.append(f"{context}: {language} text is missing or empty")
        elif Counter(PLACEHOLDERS.findall(translated)) != Counter(
            PLACEHOLDERS.findall(english)
        ):
            errors.append(f"{context}: {language} placeholders differ from english")


def inspect(value: Any, context: str) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key in VISIBLE_FIELDS:
                validate_text(child, f"{context}.{key}")
            else:
                inspect(child, f"{context}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            inspect(child, f"{context}[{index}]")


def check_ui_catalogs() -> None:
    catalogs: dict[str, dict[str, str]] = {}
    for language in LANGUAGES:
        path = DATA / "localization" / f"{language}.json"
        catalog = read_json(path)
        if isinstance(catalog, dict):
            catalogs[language] = catalog
        elif catalog is not None:
            errors.append(f"{path.relative_to(ROOT)}: catalog must be a JSON object")
    english = catalogs.get("english")
    if english is None:
        return
    for language in LANGUAGES:
        catalog = catalogs.get(language)
        if catalog is None:
            continue
        if set(catalog) != set(english):
            errors.append(
                f"data/localization/{language}.json: missing {sorted(set(english) - set(catalog))}; "
                f"unexpected {sorted(set(catalog) - set(english))}"
            )
        for key in set(english) & set(catalog):
            original, translated = english[key], catalog[key]
            if not isinstance(original, str) or not original.strip():
                errors.append(f"data/localization/english.json:{key}: empty or non-string")
            if not isinstance(translated, str) or not translated.strip():
                errors.append(f"data/localization/{language}.json:{key}: empty or non-string")
            elif isinstance(original, str) and Counter(
                PLACEHOLDERS.findall(original)
            ) != Counter(PLACEHOLDERS.findall(translated)):
                errors.append(f"data/localization/{language}.json:{key}: placeholders differ")


def main() -> int:
    if not DATA.is_dir():
        print(f"Missing content directory: {DATA}", file=sys.stderr)
        return 1
    for path in sorted(DATA.rglob("*.json")):
        if path.parent == DATA / "localization":
            continue
        content = read_json(path)
        if content is not None:
            inspect(content, str(path.relative_to(ROOT)))
    check_ui_catalogs()
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        print(f"Localization audit failed: {len(errors)} problem(s)", file=sys.stderr)
        return 1
    print(f"Localization audit passed: {checked_fields} visible fields and three UI catalogs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
