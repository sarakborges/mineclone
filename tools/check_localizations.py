#!/usr/bin/env python3
"""Validate UI/data localization catalogs and reject embedded translations."""
from __future__ import annotations
import json, re, sys
from collections import Counter
from pathlib import Path
from typing import Any
ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data"
LOC = DATA / "localization"
LANGS = ("english", "portuguese_brazil", "spanish")
PH = re.compile(r"\{([^{}]+)\}")
errors: list[str] = []
checked = 0

def read(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as error:
        errors.append(f"{path.relative_to(ROOT)}: {error}")
        return None

def ph(text: str) -> Counter[str]:
    return Counter(PH.findall(text))

def check_ui() -> None:
    cats = {lang: read(LOC / lang / "ui.json") for lang in LANGS}
    en = cats["english"]
    if not isinstance(en, dict):
        errors.append("english UI catalog must be an object")
        return
    for lang, cat in cats.items():
        if not isinstance(cat, dict):
            errors.append(f"{lang} UI catalog must be an object")
            continue
        if set(cat) != set(en):
            errors.append(f"{lang} UI keys differ from english")
        for key in set(cat) & set(en):
            a, b = en[key], cat[key]
            if not isinstance(a, str) or not a.strip() or not isinstance(b, str) or not b.strip():
                errors.append(f"{lang} UI {key}: invalid text")
            elif ph(a) != ph(b):
                errors.append(f"{lang} UI {key}: placeholders differ")

def domains(lang: str) -> dict[str, Any]:
    result = {}
    for path in sorted((LOC / lang).glob("*.json")):
        if path.name != "ui.json":
            result[path.stem] = read(path)
    return result

def check_data_catalogs() -> None:
    global checked
    cats = {lang: domains(lang) for lang in LANGS}
    en = cats["english"]
    if not en:
        errors.append("no english data localization catalogs found")
        return
    for lang in LANGS:
        if set(cats[lang]) != set(en):
            errors.append(f"{lang} data localization domains differ from english")
    for domain, defs in en.items():
        if not isinstance(defs, dict):
            errors.append(f"english/{domain}.json must be an object")
            continue
        for lang in LANGS:
            translated = cats[lang].get(domain)
            if not isinstance(translated, dict):
                continue
            if set(translated) != set(defs):
                errors.append(f"{lang}/{domain}.json ids differ from english")
            for definition_id in set(defs) & set(translated):
                a, b = defs[definition_id], translated[definition_id]
                if not isinstance(a, dict) or not isinstance(b, dict):
                    errors.append(f"{lang}/{domain}:{definition_id} fields must be objects")
                    continue
                if set(a) != set(b):
                    errors.append(f"{lang}/{domain}:{definition_id} fields differ from english")
                for pointer in set(a) & set(b):
                    checked += 1
                    x, y = a[pointer], b[pointer]
                    if not pointer.startswith("/"):
                        errors.append(f"{lang}/{domain}:{definition_id} invalid pointer {pointer!r}")
                    if not isinstance(x, str) or not x.strip() or not isinstance(y, str) or not y.strip():
                        errors.append(f"{lang}/{domain}:{definition_id}{pointer} invalid text")
                    elif ph(x) != ph(y):
                        errors.append(f"{lang}/{domain}:{definition_id}{pointer} placeholders differ")

def inspect(value: Any, context: str) -> None:
    if isinstance(value, dict):
        if set(value) == set(LANGS) and all(isinstance(value.get(lang), str) for lang in LANGS):
            errors.append(f"{context}: embedded localization is not allowed")
            return
        for key, child in value.items():
            inspect(child, f"{context}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            inspect(child, f"{context}[{index}]")

def main() -> int:
    for path in LOC.glob("*.json"):
        errors.append(f"{path.relative_to(ROOT)} must be inside a language directory")
    check_ui()
    check_data_catalogs()
    for path in sorted(DATA.rglob("*.json")):
        if LOC in path.parents:
            continue
        value = read(path)
        if value is not None:
            inspect(value, str(path.relative_to(ROOT)))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        print(f"Localization audit failed: {len(errors)} problem(s)", file=sys.stderr)
        return 1
    print(f"Localization audit passed: {checked} data translations plus UI catalogs")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
