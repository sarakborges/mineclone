use std::{collections::HashMap, fs};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::app::runtime_paths::data_root;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    #[default]
    English,
}

impl Language {
    pub(crate) const ALL: [Self; 1] = [Self::English];

    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::English => "english",
        }
    }

    pub(crate) const fn localization_key(self) -> &'static str {
        match self {
            Self::English => "language.english",
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
            .or_else(|| self.0.get(&Language::English))
            .map(String::as_str)
            .unwrap_or_else(|| panic!("localized text is missing english fallback"))
    }

    pub(crate) fn validate(&self, context: &str) {
        assert!(
            !self.text(Language::English).trim().is_empty(),
            "{context} english localization cannot be empty"
        );
    }

    pub(crate) fn as_str(&self) -> &str {
        self.text(Language::English)
    }

    pub(crate) fn to_lowercase(&self) -> String {
        self.as_str().to_lowercase()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }

    pub(crate) fn trim(&self) -> &str {
        self.as_str().trim()
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

        Self { languages }
    }

    pub(crate) fn text(&self, language: Language, key: &str) -> &str {
        self.languages
            .get(&language)
            .and_then(|strings| strings.get(key))
            .or_else(|| {
                self.languages
                    .get(&Language::English)
                    .and_then(|strings| strings.get(key))
            })
            .map(String::as_str)
            .unwrap_or_else(|| panic!("missing localization key: {key}"))
    }
}

pub(crate) struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveLanguage>()
            .insert_resource(UiLocalization::load());
    }
}
