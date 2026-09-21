use std::sync::Arc;

use bevy::platform::collections::HashMap;

#[derive(Clone)]
pub(super) struct DefinitionMap<T> {
    definitions: Arc<HashMap<String, T>>,
}

impl<T> Default for DefinitionMap<T> {
    fn default() -> Self {
        Self {
            definitions: Arc::new(HashMap::new()),
        }
    }
}

impl<T: Clone> DefinitionMap<T> {
    pub(super) fn insert(&mut self, id: String, definition: T) {
        let definitions = Arc::make_mut(&mut self.definitions);
        assert!(
            !definitions.contains_key(&id),
            "duplicate content definition id {id}"
        );
        definitions.insert(id, definition);
    }
}

impl<T> DefinitionMap<T> {
    pub(super) fn get(&self, id: &str) -> Option<&T> {
        self.definitions.get(id)
    }

    pub(super) fn values(&self) -> impl Iterator<Item = &T> {
        self.definitions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "duplicate content definition id test")]
    fn duplicate_id_is_rejected() {
        let mut definitions = DefinitionMap::default();
        definitions.insert("test".to_owned(), 1);
        definitions.insert("test".to_owned(), 2);
    }
}
