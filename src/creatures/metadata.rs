use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub(crate) const NO_AI_META_TAG: &str = "NO_AI";

pub(crate) fn normalized_meta_tag(tag: &str) -> Option<&'static str> {
    if tag.eq_ignore_ascii_case(NO_AI_META_TAG) {
        Some(NO_AI_META_TAG)
    } else {
        None
    }
}

#[derive(Component, Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(crate) struct EntityMetaTags(BTreeMap<String, Option<String>>);

impl EntityMetaTags {
    pub(crate) fn contains(&self, tag: &str) -> bool {
        normalized_meta_tag(tag).is_some_and(|tag| self.0.contains_key(tag))
    }

    pub(crate) fn is_no_ai(&self) -> bool {
        self.contains(NO_AI_META_TAG)
    }

    pub(crate) fn add(&mut self, tag: &str, value: Option<String>) -> Result<(), String> {
        let Some(tag) = normalized_meta_tag(tag) else {
            return Err(format!("unknown meta tag: {tag}"));
        };
        if self.0.contains_key(tag) {
            return Err(format!("meta tag already exists: {tag}"));
        }
        self.0.insert(tag.to_owned(), value);
        Ok(())
    }

    pub(crate) fn remove(&mut self, tag: &str) -> Result<(), String> {
        let Some(tag) = normalized_meta_tag(tag) else {
            return Err(format!("unknown meta tag: {tag}"));
        };
        if self.0.remove(tag).is_none() {
            return Err(format!("meta tag is not set: {tag}"));
        }
        Ok(())
    }

    pub(crate) fn edit(&mut self, tag: &str, value: Option<String>) -> Result<(), String> {
        let Some(tag) = normalized_meta_tag(tag) else {
            return Err(format!("unknown meta tag: {tag}"));
        };
        let Some(current) = self.0.get_mut(tag) else {
            return Err(format!("meta tag is not set: {tag}"));
        };
        *current = value;
        Ok(())
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if let Some(tag) = self.0.keys().find(|tag| normalized_meta_tag(tag).is_none()) {
            return Err(format!("unknown saved meta tag: {tag}"));
        }
        Ok(())
    }
}
