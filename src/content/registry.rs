use bevy::platform::collections::HashMap;

#[derive(Clone)]
pub(super) struct DefinitionMap<T> {
    definitions: HashMap<String, T>,
}

impl<T> Default for DefinitionMap<T> {
    fn default() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }
}

impl<T> DefinitionMap<T> {
    pub(super) fn insert(&mut self, id: String, definition: T) {
        assert!(
            !self.definitions.contains_key(&id),
            "duplicate content definition id {id}"
        );
        self.definitions.insert(id, definition);
    }

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
