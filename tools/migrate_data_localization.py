#!/usr/bin/env python3
"""One-shot migration: split embedded data localization by language/domain."""
from __future__ import annotations
import json, re, subprocess, sys
from pathlib import Path
from typing import Any
ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data"
LOC = DATA / "localization"
LANGS = ("english", "portuguese_brazil", "spanish")
JSON_FILE_RS = 'use std::{\n    collections::HashMap,\n    fs,\n    path::{Path, PathBuf},\n};\n\nuse serde::de::DeserializeOwned;\nuse serde_json::{Map, Value};\n\nuse crate::{app::runtime_paths::data_root, localization::Language};\n\ntype Catalog = HashMap<String, HashMap<String, String>>;\n\npub(super) struct DataLocalization {\n    languages: HashMap<Language, HashMap<String, Catalog>>,\n}\n\nimpl DataLocalization {\n    pub(super) fn load() -> Self {\n        let mut languages = HashMap::new();\n        for language in Language::ALL {\n            let directory = data_root().join("localization").join(language.key());\n            let mut domains = HashMap::new();\n            for entry in fs::read_dir(&directory).unwrap_or_else(|error| {\n                panic!("failed to read {}: {error}", directory.display())\n            }) {\n                let path = entry\n                    .unwrap_or_else(|error| panic!("failed to read localization entry: {error}"))\n                    .path();\n                if path.extension().and_then(|value| value.to_str()) != Some("json") {\n                    continue;\n                }\n                let domain = path\n                    .file_stem()\n                    .and_then(|value| value.to_str())\n                    .unwrap_or_else(|| panic!("invalid localization path: {}", path.display()));\n                if domain == "ui" {\n                    continue;\n                }\n                let source = fs::read_to_string(&path).unwrap_or_else(|error| {\n                    panic!("failed to read {}: {error}", path.display())\n                });\n                let catalog = serde_json::from_str(&source).unwrap_or_else(|error| {\n                    panic!("failed to parse {}: {error}", path.display())\n                });\n                assert!(\n                    domains.insert(domain.to_owned(), catalog).is_none(),\n                    "duplicate localization domain {domain}"\n                );\n            }\n            languages.insert(language, domains);\n        }\n        Self { languages }\n    }\n\n    fn hydrate(&self, path: &Path, definition: &mut Value) {\n        let root = data_root();\n        let relative = path.strip_prefix(&root).unwrap_or_else(|_| {\n            panic!("content is outside data root: {}", path.display())\n        });\n        let Some(domain) = relative\n            .components()\n            .next()\n            .and_then(|value| value.as_os_str().to_str())\n        else {\n            return;\n        };\n        if domain == "localization" {\n            return;\n        }\n        let Some(id) = definition\n            .get("id")\n            .and_then(Value::as_str)\n            .map(str::to_owned)\n        else {\n            return;\n        };\n        let Some(fields) = self\n            .languages\n            .get(&Language::English)\n            .and_then(|domains| domains.get(domain))\n            .and_then(|catalog| catalog.get(id.as_str()))\n        else {\n            return;\n        };\n\n        for pointer in fields.keys() {\n            let target = definition.pointer_mut(pointer).unwrap_or_else(|| {\n                panic!("missing localization target {domain}.{id}{pointer}")\n            });\n            let mut translations = Map::new();\n            for language in Language::ALL {\n                let text = self\n                    .languages\n                    .get(&language)\n                    .and_then(|domains| domains.get(domain))\n                    .and_then(|catalog| catalog.get(id.as_str()))\n                    .and_then(|fields| fields.get(pointer))\n                    .unwrap_or_else(|| {\n                        panic!("missing {} localization {domain}.{id}{pointer}", language.key())\n                    });\n                translations.insert(language.key().to_owned(), Value::String(text.clone()));\n            }\n            *target = Value::Object(translations);\n        }\n    }\n}\n\npub fn collect_json_files(directory: &Path, files: &mut Vec<PathBuf>) {\n    let entries = fs::read_dir(directory).unwrap_or_else(|error| {\n        panic!(\n            "failed to read content directory {}: {error}",\n            directory.display()\n        )\n    });\n\n    for entry in entries {\n        let path = entry\n            .unwrap_or_else(|error| panic!("failed to read content entry: {error}"))\n            .path();\n\n        if path.is_dir() {\n            collect_json_files(&path, files);\n        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {\n            files.push(path);\n        }\n    }\n}\n\nfn read_json_value(path: &Path) -> Value {\n    let source = fs::read_to_string(path)\n        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));\n    serde_json::from_str(&source)\n        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))\n}\n\npub fn read_localized_json_definition<T: DeserializeOwned>(\n    path: &Path,\n    localization: &DataLocalization,\n) -> T {\n    let mut value = read_json_value(path);\n    localization.hydrate(path, &mut value);\n    serde_json::from_value(value)\n        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))\n}\n'
AUDITOR = '#!/usr/bin/env python3\n"""Validate UI/data localization catalogs and reject embedded translations."""\nfrom __future__ import annotations\nimport json, re, sys\nfrom collections import Counter\nfrom pathlib import Path\nfrom typing import Any\nROOT = Path(__file__).resolve().parents[1]\nDATA = ROOT / "data"\nLOC = DATA / "localization"\nLANGS = ("english", "portuguese_brazil", "spanish")\nPH = re.compile(r"\\{([^{}]+)\\}")\nerrors: list[str] = []\nchecked = 0\n\ndef read(path: Path) -> Any:\n    try:\n        return json.loads(path.read_text(encoding="utf-8"))\n    except Exception as error:\n        errors.append(f"{path.relative_to(ROOT)}: {error}")\n        return None\n\ndef ph(text: str) -> Counter[str]:\n    return Counter(PH.findall(text))\n\ndef check_ui() -> None:\n    cats = {lang: read(LOC / lang / "ui.json") for lang in LANGS}\n    en = cats["english"]\n    if not isinstance(en, dict):\n        errors.append("english UI catalog must be an object")\n        return\n    for lang, cat in cats.items():\n        if not isinstance(cat, dict):\n            errors.append(f"{lang} UI catalog must be an object")\n            continue\n        if set(cat) != set(en):\n            errors.append(f"{lang} UI keys differ from english")\n        for key in set(cat) & set(en):\n            a, b = en[key], cat[key]\n            if not isinstance(a, str) or not a.strip() or not isinstance(b, str) or not b.strip():\n                errors.append(f"{lang} UI {key}: invalid text")\n            elif ph(a) != ph(b):\n                errors.append(f"{lang} UI {key}: placeholders differ")\n\ndef domains(lang: str) -> dict[str, Any]:\n    result = {}\n    for path in sorted((LOC / lang).glob("*.json")):\n        if path.name != "ui.json":\n            result[path.stem] = read(path)\n    return result\n\ndef check_data_catalogs() -> None:\n    global checked\n    cats = {lang: domains(lang) for lang in LANGS}\n    en = cats["english"]\n    if not en:\n        errors.append("no english data localization catalogs found")\n        return\n    for lang in LANGS:\n        if set(cats[lang]) != set(en):\n            errors.append(f"{lang} data localization domains differ from english")\n    for domain, defs in en.items():\n        if not isinstance(defs, dict):\n            errors.append(f"english/{domain}.json must be an object")\n            continue\n        for lang in LANGS:\n            translated = cats[lang].get(domain)\n            if not isinstance(translated, dict):\n                continue\n            if set(translated) != set(defs):\n                errors.append(f"{lang}/{domain}.json ids differ from english")\n            for definition_id in set(defs) & set(translated):\n                a, b = defs[definition_id], translated[definition_id]\n                if not isinstance(a, dict) or not isinstance(b, dict):\n                    errors.append(f"{lang}/{domain}:{definition_id} fields must be objects")\n                    continue\n                if set(a) != set(b):\n                    errors.append(f"{lang}/{domain}:{definition_id} fields differ from english")\n                for pointer in set(a) & set(b):\n                    checked += 1\n                    x, y = a[pointer], b[pointer]\n                    if not pointer.startswith("/"):\n                        errors.append(f"{lang}/{domain}:{definition_id} invalid pointer {pointer!r}")\n                    if not isinstance(x, str) or not x.strip() or not isinstance(y, str) or not y.strip():\n                        errors.append(f"{lang}/{domain}:{definition_id}{pointer} invalid text")\n                    elif ph(x) != ph(y):\n                        errors.append(f"{lang}/{domain}:{definition_id}{pointer} placeholders differ")\n\ndef inspect(value: Any, context: str) -> None:\n    if isinstance(value, dict):\n        if set(value) == set(LANGS) and all(isinstance(value.get(lang), str) for lang in LANGS):\n            errors.append(f"{context}: embedded localization is not allowed")\n            return\n        for key, child in value.items():\n            inspect(child, f"{context}.{key}")\n    elif isinstance(value, list):\n        for index, child in enumerate(value):\n            inspect(child, f"{context}[{index}]")\n\ndef main() -> int:\n    for path in LOC.glob("*.json"):\n        errors.append(f"{path.relative_to(ROOT)} must be inside a language directory")\n    check_ui()\n    check_data_catalogs()\n    for path in sorted(DATA.rglob("*.json")):\n        if LOC in path.parents:\n            continue\n        value = read(path)\n        if value is not None:\n            inspect(value, str(path.relative_to(ROOT)))\n    if errors:\n        print("\\n".join(errors), file=sys.stderr)\n        print(f"Localization audit failed: {len(errors)} problem(s)", file=sys.stderr)\n        return 1\n    print(f"Localization audit passed: {checked} data translations plus UI catalogs")\n    return 0\n\nif __name__ == "__main__":\n    raise SystemExit(main())\n'

def once(source: str, old: str, new: str, context: str) -> str:
    if source.count(old) != 1:
        raise RuntimeError(f"{context}: expected one exact match")
    return source.replace(old, new, 1)

def patch_rust() -> None:
    (ROOT / "src/content/json_file.rs").write_text(JSON_FILE_RS, encoding="utf-8")
    path = ROOT / "src/content/loader.rs"
    source = path.read_text(encoding="utf-8")
    source = once(source, "    json_file::{collect_json_files, read_json_definition},\n", "    json_file::{collect_json_files, read_localized_json_definition, DataLocalization},\n", str(path))
    source = once(source, "    let root = data_root();\n    let mut content = LoadedContent::default();", "    let root = data_root();\n    let localizations = DataLocalization::load();\n    let mut content = LoadedContent::default();", str(path))
    source = once(source, "    for path in files {\n        load_definition(&path, &mut content, &mut player_loaded);\n    }", "    for path in files {\n        load_definition(&path, &mut content, &mut player_loaded, &localizations);\n    }", str(path))
    source = once(source, "fn load_definition(path: &Path, content: &mut LoadedContent, player_loaded: &mut bool) {", "fn load_definition(\n    path: &Path,\n    content: &mut LoadedContent,\n    player_loaded: &mut bool,\n    localizations: &DataLocalization,\n) {", str(path))
    source, count = re.subn(r"read_json_definition::<([^>]+)>\(path\)", r"read_localized_json_definition::<\1>(path, localizations)", source)
    if count == 0 or "read_json_definition" in source:
        raise RuntimeError(f"{path}: reader migration incomplete ({count} replacements)")
    path.write_text(source, encoding="utf-8")

def localized(value: Any) -> bool:
    return isinstance(value, dict) and set(value) == set(LANGS) and all(isinstance(value.get(lang), str) for lang in LANGS)

def segment(value: str) -> str:
    return value.replace("~", "~0").replace("/", "~1")

def extract(value: Any, pointer: str, texts: dict[str, dict[str, str]]) -> Any:
    if localized(value):
        for lang in LANGS:
            texts[lang][pointer] = value[lang]
        return None
    if isinstance(value, dict):
        return {key: extract(child, f"{pointer}/{segment(key)}", texts) for key, child in value.items()}
    if isinstance(value, list):
        return [extract(child, f"{pointer}/{index}", texts) for index, child in enumerate(value)]
    return value

def write(path: Path, value: Any, sort_keys: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2, sort_keys=sort_keys) + "\n", encoding="utf-8")

def migrate() -> tuple[int, int]:
    catalogs: dict[str, dict[str, dict[str, dict[str, str]]]] = {lang: {} for lang in LANGS}
    definitions = fields = 0
    for lang in LANGS:
        for path in (LOC / lang).glob("*.json"):
            if path.name != "ui.json":
                path.unlink()
    for path in sorted(DATA.rglob("*.json")):
        if LOC in path.parents:
            continue
        content = json.loads(path.read_text(encoding="utf-8"))
        texts = {lang: {} for lang in LANGS}
        cleaned = extract(content, "", texts)
        if not texts["english"]:
            continue
        if not isinstance(content, dict) or not isinstance(content.get("id"), str) or not content["id"].strip():
            raise RuntimeError(f"{path.relative_to(ROOT)} has localized text but no non-empty id")
        definition_id = content["id"]
        domain = path.relative_to(DATA).parts[0]
        for lang in LANGS:
            domain_catalog = catalogs[lang].setdefault(domain, {})
            if definition_id in domain_catalog:
                raise RuntimeError(f"duplicate localized id {definition_id!r} in {domain}")
            domain_catalog[definition_id] = texts[lang]
        write(path, cleaned)
        definitions += 1
        fields += len(texts["english"])
    if not fields:
        raise RuntimeError("no embedded data localization found")
    for lang in LANGS:
        for domain, catalog in sorted(catalogs[lang].items()):
            write(LOC / lang / f"{domain}.json", catalog, True)
    return definitions, fields

def main() -> int:
    patch_rust()
    (ROOT / "tools/check_localizations.py").write_text(AUDITOR, encoding="utf-8")
    definitions, fields = migrate()
    subprocess.run([sys.executable, str(ROOT / "tools/check_localizations.py")], cwd=ROOT, check=True)
    print(f"Migrated {fields} localized fields from {definitions} definitions.")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
