use std::{collections::{BTreeMap, HashMap}, fs};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::app::runtime_paths::data_root;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    #[default]
    English,
    #[serde(rename = "portuguese_brazil")]
    PortugueseBrazil,
    Spanish,
}

impl Language {
    pub(crate) const ALL: [Self; 3] = [Self::English, Self::PortugueseBrazil, Self::Spanish];

    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::PortugueseBrazil => "portuguese_brazil",
            Self::Spanish => "spanish",
        }
    }

    pub(crate) const fn localization_key(self) -> &'static str {
        match self {
            Self::English => "language.english",
            Self::PortugueseBrazil => "language.portugueseBrazil",
            Self::Spanish => "language.spanish",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct LocalizedText(HashMap<Language, String>);

impl LocalizedText {
    pub(crate) fn text(&self, language: Language) -> &str {
        self.0
            .get(&language)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("localized text is missing {} translation", language.key()))
    }

    pub(crate) fn validate(&self, context: &str) {
        let english = self.0.get(&Language::English).unwrap_or_else(|| {
            panic!("{context} is missing english localization")
        });
        for language in Language::ALL {
            let translated = self.0.get(&language).unwrap_or_else(|| {
                panic!("{context} is missing {} localization", language.key())
            });
            assert!(
                !translated.trim().is_empty(),
                "{context} {} localization cannot be empty",
                language.key()
            );
            assert_eq!(
                placeholders(translated), placeholders(english),
                "{context} {} localization has mismatched placeholders",
                language.key()
            );
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ActiveLanguage(Language);

impl ActiveLanguage {
    pub(crate) const fn get(&self) -> Language {
        self.0
    }

    pub(crate) fn set(&mut self, language: Language) {
        self.0 = language;
    }
}

#[derive(Resource, Debug)]
pub(crate) struct UiLocalization {
    languages: HashMap<Language, HashMap<String, String>>,
}

fn placeholders(text: &str) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for name in text
        .split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name))
    {
        *counts.entry(name).or_default() += 1;
    }
    counts
}

impl UiLocalization {
    fn load() -> Self {
        let mut languages = HashMap::new();

        for language in Language::ALL {
            let path = data_root()
                .join("localization")
                .join(format!("{}.json", language.key()));
            let source = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("failed to read localization file {}: {error}", path.display())
            });
            let strings = serde_json::from_str::<HashMap<String, String>>(&source)
                .unwrap_or_else(|error| {
                    panic!("failed to parse localization file {}: {error}", path.display())
                });
            languages.insert(language, strings);
        }

        let english = languages
            .get(&Language::English)
            .expect("english UI localization must exist");
        for language in Language::ALL {
            let strings = languages
                .get(&language)
                .expect("all configured UI localizations must exist");
            assert_eq!(
                strings.len(), english.len(),
                "{} UI localization must have exactly the English keys",
                language.key()
            );
            for (key, original) in english {
                let translated = strings.get(key).unwrap_or_else(|| {
                    panic!("{} UI localization is missing key {key}", language.key())
                });
                assert!(
                    !translated.trim().is_empty(),
                    "{} UI localization has empty key {key}",
                    language.key()
                );
                assert_eq!(
                    placeholders(translated), placeholders(original),
                    "{} UI localization has mismatched placeholders for {key}",
                    language.key()
                );
            }
        }

        Self { languages }
    }

    pub(crate) fn text(&self, language: Language, key: &str) -> &str {
        self.languages
            .get(&language)
            .and_then(|strings| strings.get(key))
            .map(String::as_str)
            .unwrap_or_else(|| panic!("missing {} localization key: {key}", language.key()))
    }
}

pub(crate) struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveLanguage>()
            .insert_resource(UiLocalization::load());
    }
}

#[cfg(test)]
mod tests {
    use super::placeholders;

    #[test]
    fn placeholder_validation_preserves_multiplicity() {
        assert_eq!(
            placeholders("{value} / {value}"),
            placeholders("{value} / {value}")
        );
        assert_ne!(
            placeholders("{value} / {value}"),
            placeholders("{value}")
        );
    }
}
