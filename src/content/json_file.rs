use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::{app::runtime_paths::data_root, localization::Language};

type Catalog = HashMap<String, HashMap<String, String>>;

pub(super) struct DataLocalization {
    languages: HashMap<Language, HashMap<String, Catalog>>,
}

fn localization_target_mut<'a>(
    definition: &'a mut Value,
    pointer: &str,
    domain: &str,
    id: &str,
) -> &'a mut Value {
    if definition.pointer(pointer).is_some() {
        return definition
            .pointer_mut(pointer)
            .expect("localization pointer disappeared");
    }

    let (parent_pointer, token) = pointer.rsplit_once('/').unwrap_or_else(|| {
        panic!("invalid localization pointer {domain}.{id}{pointer}")
    });
    assert!(
        !token.is_empty(),
        "invalid localization pointer {domain}.{id}{pointer}"
    );
    let key = token.replace("~1", "/").replace("~0", "~");
    let parent = if parent_pointer.is_empty() {
        definition
    } else {
        definition.pointer_mut(parent_pointer).unwrap_or_else(|| {
            panic!("missing localization parent {domain}.{id}{parent_pointer}")
        })
    };
    let Value::Object(object) = parent else {
        panic!("localization parent {domain}.{id}{parent_pointer} must be an object")
    };
    object.entry(key).or_insert(Value::Null)
}

impl DataLocalization {
    pub(super) fn load() -> Self {
        let mut languages = HashMap::new();
        for language in Language::ALL {
            let directory = data_root().join("localization").join(language.key());
            let mut domains = HashMap::new();
            for entry in fs::read_dir(&directory).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", directory.display())
            }) {
                let path = entry
                    .unwrap_or_else(|error| panic!("failed to read localization entry: {error}"))
                    .path();
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let domain = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_else(|| panic!("invalid localization path: {}", path.display()));
                if domain == "ui" {
                    continue;
                }
                let source = fs::read_to_string(&path).unwrap_or_else(|error| {
                    panic!("failed to read {}: {error}", path.display())
                });
                let catalog = serde_json::from_str(&source).unwrap_or_else(|error| {
                    panic!("failed to parse {}: {error}", path.display())
                });
                assert!(
                    domains.insert(domain.to_owned(), catalog).is_none(),
                    "duplicate localization domain {domain}"
                );
            }
            languages.insert(language, domains);
        }
        Self { languages }
    }

    fn hydrate(&self, path: &Path, definition: &mut Value) {
        let root = data_root();
        let relative = path.strip_prefix(&root).unwrap_or_else(|_| {
            panic!("content is outside data root: {}", path.display())
        });
        let Some(domain) = relative
            .components()
            .next()
            .and_then(|value| value.as_os_str().to_str())
        else {
            return;
        };
        if domain == "localization" {
            return;
        }
        let Some(id) = definition
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return;
        };
        let Some(fields) = self
            .languages
            .get(&Language::English)
            .and_then(|domains| domains.get(domain))
            .and_then(|catalog| catalog.get(id.as_str()))
        else {
            return;
        };

        for pointer in fields.keys() {
            let target = localization_target_mut(definition, pointer, domain, &id);
            let mut translations = Map::new();
            for language in Language::ALL {
                let text = self
                    .languages
                    .get(&language)
                    .and_then(|domains| domains.get(domain))
                    .and_then(|catalog| catalog.get(id.as_str()))
                    .and_then(|fields| fields.get(pointer))
                    .unwrap_or_else(|| {
                        panic!("missing {} localization {domain}.{id}{pointer}", language.key())
                    });
                translations.insert(language.key().to_owned(), Value::String(text.clone()));
            }
            *target = Value::Object(translations);
        }
    }
}

pub fn collect_json_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory).unwrap_or_else(|error| {
        panic!(
            "failed to read content directory {}: {error}",
            directory.display()
        )
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("failed to read content entry: {error}"))
            .path();

        if path.is_dir() {
            collect_json_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            files.push(path);
        }
    }
}

fn read_json_value(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

pub fn read_localized_json_definition<T: DeserializeOwned>(
    path: &Path,
    localization: &DataLocalization,
) -> T {
    let mut value = read_json_value(path);
    localization.hydrate(path, &mut value);
    serde_json::from_value(value)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}
